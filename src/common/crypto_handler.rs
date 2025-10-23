use openssl::hash::{Hasher, MessageDigest};
use openssl::pkcs5::pbkdf2_hmac;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::RngCore;
use std::fs;
use std::path::Path;

// see https://www.rfc-editor.org/rfc/rfc3447#section-7.2.1
pub const SHA256_DIGEST_LENGTH: usize = 32;

#[derive(Debug)]
pub struct CryptoHandler {
    pub key: Vec<u8>,
    pub id: Vec<u8>,
}

impl CryptoHandler {
    pub fn create(key_string: String) -> Result<Self, String> {
        let bytes = (0..key_string.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&key_string[i..i + 2], 16))
            .collect()
            .map_err(|e| format!("invalid hex: {e}"))?;

        let (id, key) = bytes.split_at(8);

        if key.len() != 32 {
            return Err("Key length must be 32")?;
        }

        Ok(CryptoHandler {
            key: key.to_vec(),
            id: id.to_vec(),
        })
    }

    pub fn gen_key() -> Result<String, String> {
        let mut secret = [0u8; 32];
        rand_bytes(&mut secret).map_err(|e| format!("Could not generate secret: {e}"))?;

        let mut salt = [0u8; 16];
        rand_bytes(&mut salt).map_err(|e| format!("Could not generate salt: {e}"))?;

        let iterations = 100_000;
        let mut key = [0u8; 32];
        pbkdf2_hmac(&secret, &salt, iterations, MessageDigest::sha256(), &mut key)
            .map_err(|e| format!("Could not generate AES key: {e}"))?;

        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 8];
        rng.fill_bytes(&mut random_bytes);

        let random_hex: String = random_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let key_hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();

        Ok(format!("{}{}", random_hex, key_hex))
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
        let cipher = Cipher::aes_256_gcm();
        let mut iv = vec![0u8; 12];
        rand_bytes(&mut iv).map_err(|e| e.to_string())?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &self.key, Some(&iv))
            .map_err(|e| format!("Could not create crypter: {}", e))?;

        let mut ciphertext = vec![0; plaintext.len() + cipher.block_size()];
        let mut count = crypter
            .update(plaintext, &mut ciphertext)
            .map_err(|e| format!("Could not update crypter: {}", e))?;

        count += crypter
            .finalize(&mut ciphertext[count..])
            .map_err(|e| format!("Could not finalize crypter: {}", e))?;

        ciphertext.truncate(count);

        let mut tag = vec![0u8; 16];
        crypter.get_tag(&mut tag).map_err(|e| format!("Could not get tag from crypter: {}", e))?;

        Ok((iv, ciphertext, tag))
    }

    pub fn decrypt(&self, iv: &[u8], ciphertext: &[u8], tag: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Cipher::aes_256_gcm();
        let mut decrypter = Crypter::new(cipher, Mode::Decrypt, &self.key, Some(iv))
            .map_err(|e| format!("Could not create decrypter: {}", e))?;

        let mut plaintext = vec![0; ciphertext.len() + cipher.block_size()];
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
    use crate::common::crypto_handler::CryptoHandler;

    #[test]
    fn test_encrypt() {
        let plaintext = b"Hello world!";

        let key = CryptoHandler::gen_key().unwrap();
        let handler = CryptoHandler::create(&key).unwrap();


        let (iv, cipher, tag) = handler.encrypt(plaintext).unwrap();
        let decrypted = handler.decrypt(&iv, &cipher, &tag).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
