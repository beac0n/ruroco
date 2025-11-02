use openssl::hash::MessageDigest;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs;
use std::num::ParseIntError;
use std::path::Path;

pub const KEY_ID_SIZE: usize = 8;
pub const IV_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;
pub const BLOCK_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 16;
const KEY_DERIVATION_ITERATIONS: usize = 100_000;

#[derive(Debug)]
pub struct CryptoHandler {
    pub key: [u8; KEY_SIZE],
    pub id: [u8; KEY_ID_SIZE],
}

impl CryptoHandler {
    pub fn from_key_path(key_path: &Path) -> Result<Self, String> {
        let key = fs::read_to_string(key_path).map_err(|e| format!("Could not read key: {e}"))?;
        Self::create(&key)
    }

    pub fn create(key_string: &str) -> Result<Self, String> {
        let key_string = key_string.trim();
        let key_string_len = KEY_ID_SIZE + KEY_SIZE;
        let key_string_len_hex = key_string_len * 2;
        if key_string.len() != key_string_len_hex {
            return Err(&format!(
                "Key length must be {key_string_len_hex} hex characters ({key_string_len} bytes)"
            ))?;
        }

        let bytes = (0..key_string.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&key_string[i..i + 2], 16))
            .collect::<Result<Vec<u8>, ParseIntError>>()
            .map_err(|e| format!("invalid hex: {e}"))?;

        let (id, key) = bytes.split_at(KEY_ID_SIZE);

        if key.len() != KEY_SIZE {
            return Err(&format!("Key length must be {KEY_SIZE} bytes"))?;
        }

        Ok(CryptoHandler {
            key: key.try_into().map_err(|e| format!("Could not convert key: {e}"))?,
            id: id.try_into().map_err(|e| format!("Could not convert key id: {e}"))?,
        })
    }

    pub fn gen_key() -> Result<String, String> {
        let mut secret = [0u8; KEY_SIZE];
        rand_bytes(&mut secret).map_err(|e| format!("Could not generate secret: {e}"))?;

        let mut salt = [0u8; SALT_SIZE];
        rand_bytes(&mut salt).map_err(|e| format!("Could not generate salt: {e}"))?;

        let mut key = [0u8; KEY_SIZE];
        pbkdf2_hmac(&secret, &salt, KEY_DERIVATION_ITERATIONS, MessageDigest::sha256(), &mut key)
            .map_err(|e| format!("Could not generate AES key: {e}"))?;

        let mut id = [0u8; KEY_ID_SIZE];
        rand_bytes(&mut id).map_err(|e| format!("Could not generate key id: {e}"))?;

        let id_hex: String = id.iter().map(|b| format!("{:02x}", b)).collect();
        let key_hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();

        Ok(format!("{id_hex}{key_hex}"))
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Cipher::aes_256_gcm();
        let mut iv = vec![0u8; IV_SIZE];
        rand_bytes(&mut iv).map_err(|e| e.to_string())?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &self.key, Some(&iv))
            .map_err(|e| format!("Could not create crypter: {}", e))?;

        let mut ciphertext = vec![0; plaintext.len() + BLOCK_SIZE];
        let mut count = crypter
            .update(plaintext, &mut ciphertext)
            .map_err(|e| format!("Could not update crypter: {}", e))?;

        count += crypter
            .finalize(&mut ciphertext[count..])
            .map_err(|e| format!("Could not finalize crypter: {}", e))?;

        ciphertext.truncate(count);

        let mut tag = vec![0u8; TAG_SIZE];
        crypter.get_tag(&mut tag).map_err(|e| format!("Could not get tag from crypter: {}", e))?;

        Ok([iv, tag, ciphertext].concat())
    }

    pub fn decrypt(&self, iv_tag_ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let iv = &iv_tag_ciphertext[..IV_SIZE];
        let tag = &iv_tag_ciphertext[IV_SIZE..IV_SIZE + TAG_SIZE];
        let ciphertext = &iv_tag_ciphertext[IV_SIZE + TAG_SIZE..];

        let cipher = Cipher::aes_256_gcm();
        let mut decrypter = Crypter::new(cipher, Mode::Decrypt, &self.key, Some(iv))
            .map_err(|e| format!("Could not create decrypter: {}", e))?;

        let mut plaintext = vec![0; ciphertext.len() + BLOCK_SIZE];
        let mut count = decrypter
            .update(ciphertext, &mut plaintext)
            .map_err(|e| format!("Could not update decrypter: {}", e))?;

        // supply the tag before finalize so verification can occur
        decrypter.set_tag(tag).map_err(|e| format!("Could not set tag: {}", e))?;

        count += decrypter
            .finalize(&mut plaintext[count..])
            .map_err(|e| format!("Could not finalize decrypter (auth failed?): {}", e))?;
        plaintext.truncate(count);

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::crypto_handler::{CryptoHandler, IV_SIZE, TAG_SIZE};

    #[test]
    fn test_encrypt() {
        let plaintext = b"Hello world!";

        let key = CryptoHandler::gen_key().unwrap();
        let handler = CryptoHandler::create(&key).unwrap();

        let ciphertext = handler.encrypt(plaintext).unwrap();
        let decrypted = handler.decrypt(&ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn ciphertext_length_matches_plaintext_length() {
        let key = CryptoHandler::gen_key().unwrap();
        let handler = CryptoHandler::create(&key).unwrap();

        for size in [0usize, 1, 5, 16, 31, 64, 255] {
            let plaintext = vec![b'a'; size];
            let ciphertext = handler.encrypt(&plaintext).unwrap();

            let encrypted_section = &ciphertext[IV_SIZE + TAG_SIZE..];
            assert_eq!(
                encrypted_section.len(),
                plaintext.len(),
                "mismatched length for size {size}"
            );
        }
    }
}
