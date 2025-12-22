use anyhow::{anyhow, Context};
use base64::{engine::general_purpose, Engine};
use openssl::hash::MessageDigest;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs;
use std::path::Path;

pub(crate) const PLAINTEXT_SIZE: usize = 57;
pub(crate) const CIPHERTEXT_SIZE: usize = 85;
pub(crate) const KEY_ID_SIZE: usize = 8;
pub(crate) const IV_SIZE: usize = 12;
pub(crate) const TAG_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 16;
const KEY_DERIVATION_ITERATIONS: usize = 100_000;

#[derive(Debug)]
pub(crate) struct CryptoHandler {
    pub(crate) key: [u8; KEY_SIZE],
    pub(crate) id: [u8; KEY_ID_SIZE],
}

impl CryptoHandler {
    pub(crate) fn from_key_path(key_path: &Path) -> anyhow::Result<Self> {
        let key = fs::read_to_string(key_path).with_context(|| "Could not read key")?;
        Self::create(&key)
    }

    pub(crate) fn create(key_string: &str) -> anyhow::Result<Self> {
        let key_string = key_string.trim();
        let bytes = general_purpose::STANDARD
            .decode(key_string)
            .with_context(|| "Could not decode base64 key")?;

        let (id, key) =
            bytes.split_at_checked(KEY_ID_SIZE).ok_or_else(|| anyhow!("Key too short"))?;

        if key.len() != KEY_SIZE {
            return Err(anyhow!("Key length must be {KEY_SIZE} bytes"));
        }

        Ok(CryptoHandler {
            key: key.try_into().with_context(|| "Could not convert key")?,
            id: id.try_into().with_context(|| "Could not convert key id")?,
        })
    }

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
            return Err(anyhow!("ciphertext length mismatch"));
        }

        if crypter.finalize(&mut []).with_context(|| "Could not finalize crypter")? != 0 {
            return Err(anyhow!("GCM finalize returned unexpected bytes"));
        }

        let mut tag = [0u8; TAG_SIZE];
        crypter.get_tag(&mut tag).with_context(|| "Could not get tag from crypter")?;

        let mut out = [0u8; CIPHERTEXT_SIZE];
        out[0..IV_SIZE].copy_from_slice(&iv);
        out[IV_SIZE..IV_SIZE + TAG_SIZE].copy_from_slice(&tag);
        out[IV_SIZE + TAG_SIZE..].copy_from_slice(&ciphertext);

        Ok(out)
    }

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
            return Err(anyhow!("Plaintext length mismatch"));
        }

        decrypter.set_tag(tag).with_context(|| "Could not set tag")?;

        if decrypter.finalize(&mut []).with_context(|| "Could not finalize decrypter")? != 0 {
            return Err(anyhow!("GCM finalize returned unexpected bytes"));
        }

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::crypto_handler::{CryptoHandler, PLAINTEXT_SIZE};
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
}
