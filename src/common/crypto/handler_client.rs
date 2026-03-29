use crate::common::crypto::handler::CryptoHandler;
use crate::common::protocol::{CIPHERTEXT_SIZE, KEY_ID_SIZE, PLAINTEXT_SIZE};
use anyhow::Context;
use base64::{engine::general_purpose, Engine};
use openssl::hash::MessageDigest;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};

const IV_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 16;
const KEY_DERIVATION_ITERATIONS: usize = 100_000;

impl CryptoHandler {
    pub(crate) fn gen_key() -> anyhow::Result<String> {
        let mut secret = [0u8; KEY_SIZE];
        rand_bytes(&mut secret).with_context(|| "Could not generate secret")?;

        let mut salt = [0u8; SALT_SIZE];
        rand_bytes(&mut salt).with_context(|| "Could not generate salt")?;

        let mut key = [0u8; KEY_SIZE];
        pbkdf2_hmac(&secret, &salt, KEY_DERIVATION_ITERATIONS, MessageDigest::sha256(), &mut key)
            .with_context(|| "Could not generate AES key")?;

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

#[cfg(test)]
mod tests {
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
