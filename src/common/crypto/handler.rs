use anyhow::{anyhow, bail, Context};
use base64::{engine::general_purpose, Engine};

use crate::common::protocol::KEY_ID_SIZE;

const KEY_SIZE: usize = 32;

#[derive(Debug)]
pub(crate) struct CryptoHandler {
    pub(crate) key: [u8; KEY_SIZE],
    pub(crate) id: [u8; KEY_ID_SIZE],
}

impl CryptoHandler {
    pub(crate) fn create(key_string: &str) -> anyhow::Result<Self> {
        let key_string = key_string.trim();
        let bytes = general_purpose::STANDARD
            .decode(key_string)
            .with_context(|| "Could not decode base64 key")?;

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
}

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

    #[test]
    fn test_from_key_path() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        let key = CryptoHandler::gen_key().unwrap();
        std::fs::write(&key_path, &key).unwrap();

        let handler = CryptoHandler::from_key_path(&key_path).unwrap();
        let from_str = CryptoHandler::create(&key).unwrap();
        assert_eq!(handler.key, from_str.key);
        assert_eq!(handler.id, from_str.id);
    }
}
