use anyhow::{anyhow, bail, Context};
use base64::{engine::general_purpose, Engine};
use zeroize::{ZeroizeOnDrop, Zeroizing};

use crate::common::protocol::KEY_ID_SIZE;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use crate::common::protocol::{CIPHERTEXT_SIZE, PLAINTEXT_SIZE};
#[cfg(feature = "with-client")]
use openssl::rand::rand_bytes;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use openssl::symm::{Cipher, Crypter, Mode};

const KEY_SIZE: usize = 32;
#[cfg(any(feature = "with-client", feature = "with-server"))]
const IV_SIZE: usize = 12;
#[cfg(any(feature = "with-client", feature = "with-server"))]
const TAG_SIZE: usize = 16;

#[derive(ZeroizeOnDrop)]
pub(crate) struct CryptoHandler {
    pub(crate) key: [u8; KEY_SIZE],
    pub(crate) id: [u8; KEY_ID_SIZE],
}

impl core::fmt::Debug for CryptoHandler {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CryptoHandler").field("id", &self.id).field("key", &"<redacted>").finish()
    }
}

impl CryptoHandler {
    pub(crate) fn create(key_string: &str) -> anyhow::Result<Self> {
        let bytes = Zeroizing::new(
            general_purpose::STANDARD
                .decode(key_string.trim())
                .with_context(|| "Could not decode base64 key")?,
        );

        let (id, key) =
            bytes.split_at_checked(KEY_ID_SIZE).ok_or_else(|| anyhow!("Key too short"))?;

        if key.len() != KEY_SIZE {
            bail!("Key length must be {KEY_SIZE} bytes");
        }

        Ok(CryptoHandler {
            key: key.try_into().with_context(|| "Could not convert key")?,
            id: id.try_into().with_context(|| "Could not convert key id")?,
        })
    }
}

#[cfg(feature = "with-client")]
impl CryptoHandler {
    pub(crate) fn gen_key() -> anyhow::Result<String> {
        let mut key = [0u8; KEY_SIZE];
        rand_bytes(&mut key).with_context(|| "Could not generate key")?;

        let mut id = [0u8; KEY_ID_SIZE];
        rand_bytes(&mut id).with_context(|| "Could not generate key id")?;

        Ok(general_purpose::STANDARD.encode([id.as_slice(), key.as_slice()].concat()))
    }

    pub(crate) fn encrypt(
        &self,
        plaintext: &[u8; PLAINTEXT_SIZE],
    ) -> anyhow::Result<[u8; CIPHERTEXT_SIZE]> {
        let cipher = Cipher::aes_256_gcm();
        let mut iv = [0u8; IV_SIZE];
        rand_bytes(&mut iv).with_context(|| "Could not generate IV")?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &self.key, Some(&iv))
            .with_context(|| "Could not create crypter")?;

        let mut ciphertext = [0u8; PLAINTEXT_SIZE];
        let count = crypter
            .update(plaintext, &mut ciphertext)
            .with_context(|| "Could not update crypter")?;

        if count != PLAINTEXT_SIZE {
            anyhow::bail!("ciphertext length mismatch");
        }

        if crypter.finalize(&mut []).with_context(|| "Could not finalize crypter")? != 0 {
            anyhow::bail!("GCM finalize returned unexpected bytes");
        }

        let mut tag = [0u8; TAG_SIZE];
        crypter.get_tag(&mut tag).with_context(|| "Could not get tag from crypter")?;

        let mut out = [0u8; CIPHERTEXT_SIZE];
        out[0..IV_SIZE].copy_from_slice(&iv);
        out[IV_SIZE..IV_SIZE + TAG_SIZE].copy_from_slice(&tag);
        out[IV_SIZE + TAG_SIZE..].copy_from_slice(&ciphertext);

        Ok(out)
    }
}

#[cfg(feature = "with-server")]
impl CryptoHandler {
    pub(crate) fn decrypt(
        &self,
        iv_tag_ciphertext: &[u8; CIPHERTEXT_SIZE],
    ) -> anyhow::Result<[u8; PLAINTEXT_SIZE]> {
        let iv = &iv_tag_ciphertext[..IV_SIZE];
        let tag = &iv_tag_ciphertext[IV_SIZE..IV_SIZE + TAG_SIZE];
        let ciphertext = &iv_tag_ciphertext[IV_SIZE + TAG_SIZE..];

        let cipher = Cipher::aes_256_gcm();
        let mut decrypter = Crypter::new(cipher, Mode::Decrypt, &self.key, Some(iv))
            .with_context(|| "Could not create decrypter")?;

        let mut plaintext = [0u8; PLAINTEXT_SIZE];
        let written = decrypter
            .update(ciphertext, &mut plaintext)
            .with_context(|| "Could not update decrypter")?;

        if written != PLAINTEXT_SIZE {
            anyhow::bail!("Plaintext length mismatch");
        }

        decrypter.set_tag(tag).with_context(|| "Could not set tag")?;

        if decrypter.finalize(&mut []).with_context(|| "Could not finalize decrypter")? != 0 {
            anyhow::bail!("GCM finalize returned unexpected bytes");
        }

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::CryptoHandler;

    #[test]
    fn test_create_key_too_short() {
        use base64::engine::general_purpose;
        use base64::Engine;
        let short = general_purpose::STANDARD.encode([0u8; 4]);
        let result = CryptoHandler::create(&short);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Key too short");
    }

    #[test]
    fn test_create_key_wrong_length() {
        use base64::engine::general_purpose;
        use base64::Engine;
        let data = [0u8; 24];
        let encoded = general_purpose::STANDARD.encode(data);
        let result = CryptoHandler::create(&encoded);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Key length must be 32 bytes");
    }

    #[test]
    fn test_create_invalid_base64() {
        let result = CryptoHandler::create("not valid base64!!!");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not decode base64 key"));
    }

    #[test]
    fn test_debug_redacts_key() {
        use base64::engine::general_purpose;
        use base64::Engine;
        // id bytes = 0x01 (prints as 1), key bytes = 0xAB (prints as 171 if not redacted)
        let mut raw = [0u8; 40];
        raw[..8].fill(0x01);
        raw[8..].fill(0xAB);
        let encoded = general_purpose::STANDARD.encode(raw);
        let handler = CryptoHandler::create(&encoded).unwrap();
        let debug = format!("{handler:?}");
        assert!(debug.contains("<redacted>"), "key must be redacted in Debug output");
        assert!(!debug.contains("171"), "raw key bytes must not appear in Debug output");
    }
}

#[cfg(feature = "with-client")]
#[cfg(test)]
mod encrypt_tests {
    use super::*;
    use crate::common::protocol::PLAINTEXT_SIZE;

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let mut plaintext = [0u8; PLAINTEXT_SIZE];
        rand_bytes(&mut plaintext).unwrap();

        let key = CryptoHandler::gen_key().unwrap();
        let handler = CryptoHandler::create(&key).unwrap();

        let ct1 = handler.encrypt(&plaintext).unwrap();
        let ct2 = handler.encrypt(&plaintext).unwrap();
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_key_with_whitespace() {
        let key = CryptoHandler::gen_key().unwrap();
        let padded = format!("  {key}  \n");
        let handler = CryptoHandler::create(&padded).unwrap();
        let from_str = CryptoHandler::create(&key).unwrap();
        assert_eq!(handler.key, from_str.key);
    }
}

#[cfg(all(feature = "with-client", feature = "with-server"))]
#[cfg(test)]
mod cross_tests {
    use super::CryptoHandler;
    use crate::common::protocol::PLAINTEXT_SIZE;
    use openssl::rand::rand_bytes;

    #[test]
    fn test_encrypt() {
        let mut plaintext = [0u8; PLAINTEXT_SIZE];
        rand_bytes(&mut plaintext).unwrap();

        let key = CryptoHandler::gen_key().unwrap();
        let handler = CryptoHandler::create(&key).unwrap();

        let ciphertext = handler.encrypt(&plaintext).unwrap();
        let decrypted = handler.decrypt(&ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let mut plaintext = [0u8; PLAINTEXT_SIZE];
        rand_bytes(&mut plaintext).unwrap();

        let key1 = CryptoHandler::gen_key().unwrap();
        let key2 = CryptoHandler::gen_key().unwrap();
        let handler1 = CryptoHandler::create(&key1).unwrap();
        let handler2 = CryptoHandler::create(&key2).unwrap();

        let ciphertext = handler1.encrypt(&plaintext).unwrap();
        let result = handler2.decrypt(&ciphertext);
        assert!(result.is_err());
    }
}
