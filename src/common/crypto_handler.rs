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
    pub(crate) fn from_key_path(key_path: &Path) -> Result<Self, String> {
        let key = fs::read_to_string(key_path).map_err(|e| format!("Could not read key: {e}"))?;
        Self::create(&key)
    }

    pub(crate) fn create(key_string: &str) -> Result<Self, String> {
        let key_string = key_string.trim();
        let bytes = general_purpose::STANDARD
            .decode(key_string)
            .map_err(|e| format!("Could not decode base64 key: {e}"))?;

        let (id, key) = bytes.split_at_checked(KEY_ID_SIZE).ok_or("Key too short".to_string())?;

        if key.len() != KEY_SIZE {
            return Err(&format!("Key length must be {KEY_SIZE} bytes"))?;
        }

        Ok(CryptoHandler {
            key: key.try_into().map_err(|e| format!("Could not convert key: {e}"))?,
            id: id.try_into().map_err(|e| format!("Could not convert key id: {e}"))?,
        })
    }

    pub(crate) fn gen_key() -> Result<String, String> {
        let mut secret = [0u8; KEY_SIZE];
        rand_bytes(&mut secret).map_err(|e| format!("Could not generate secret: {e}"))?;

        let mut salt = [0u8; SALT_SIZE];
        rand_bytes(&mut salt).map_err(|e| format!("Could not generate salt: {e}"))?;

        let mut key = [0u8; KEY_SIZE];
        pbkdf2_hmac(&secret, &salt, KEY_DERIVATION_ITERATIONS, MessageDigest::sha256(), &mut key)
            .map_err(|e| format!("Could not generate AES key: {e}"))?;

        let mut id = [0u8; KEY_ID_SIZE];
        rand_bytes(&mut id).map_err(|e| format!("Could not generate key id: {e}"))?;

        Ok(general_purpose::STANDARD.encode([id.as_slice(), key.as_slice()].concat()))
    }

    pub(crate) fn encrypt(
        &self,
        plaintext: &[u8; PLAINTEXT_SIZE],
    ) -> Result<[u8; CIPHERTEXT_SIZE], String> {
        let cipher = Cipher::aes_256_gcm();
        let mut iv = [0u8; IV_SIZE];
        rand_bytes(&mut iv).map_err(|e| e.to_string())?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &self.key, Some(&iv))
            .map_err(|e| format!("Could not create crypter: {e}"))?;

        let mut ciphertext = [0u8; PLAINTEXT_SIZE];
        let count = crypter
            .update(plaintext, &mut ciphertext)
            .map_err(|e| format!("Could not update crypter: {e}"))?;

        if count != PLAINTEXT_SIZE {
            return Err("ciphertext length mismatch".into());
        }

        if crypter.finalize(&mut []).map_err(|e| format!("Could not finalize crypter: {e}"))? != 0 {
            return Err("GCM finalize returned unexpected bytes".into());
        }

        let mut tag = [0u8; TAG_SIZE];
        crypter.get_tag(&mut tag).map_err(|e| format!("Could not get tag from crypter: {e}"))?;

        let mut out = [0u8; CIPHERTEXT_SIZE];
        out[0..IV_SIZE].copy_from_slice(&iv);
        out[IV_SIZE..IV_SIZE + TAG_SIZE].copy_from_slice(&tag);
        out[IV_SIZE + TAG_SIZE..].copy_from_slice(&ciphertext);

        Ok(out)
    }

    pub(crate) fn decrypt(
        &self,
        iv_tag_ciphertext: &[u8; CIPHERTEXT_SIZE],
    ) -> Result<[u8; PLAINTEXT_SIZE], String> {
        let iv = &iv_tag_ciphertext[..IV_SIZE];
        let tag = &iv_tag_ciphertext[IV_SIZE..IV_SIZE + TAG_SIZE];
        let ciphertext = &iv_tag_ciphertext[IV_SIZE + TAG_SIZE..];

        let cipher = Cipher::aes_256_gcm();
        let mut decrypter = Crypter::new(cipher, Mode::Decrypt, &self.key, Some(iv))
            .map_err(|e| format!("Could not create decrypter: {e}"))?;

        let mut plaintext = [0u8; PLAINTEXT_SIZE];
        let written = decrypter
            .update(ciphertext, &mut plaintext)
            .map_err(|e| format!("Could not update decrypter: {e}"))?;

        if written != PLAINTEXT_SIZE {
            return Err("Plaintext length mismatch".into());
        }

        decrypter.set_tag(tag).map_err(|e| format!("Could not set tag: {e}"))?;

        if decrypter.finalize(&mut []).map_err(|e| format!("Could not finalize decrypter: {e}"))?
            != 0
        {
            return Err("GCM finalize returned unexpected bytes".into());
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
