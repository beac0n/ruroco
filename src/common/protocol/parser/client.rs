use crate::common::crypto::handler::CryptoHandler;
use crate::common::protocol::{KEY_ID_SIZE, MSG_SIZE, PLAINTEXT_SIZE};

#[derive(Debug)]
pub(crate) struct DataParser {
    pub(super) crypto_handler: CryptoHandler,
}

impl DataParser {
    pub(crate) fn create(key_string: &str) -> anyhow::Result<Self> {
        Ok(DataParser {
            crypto_handler: CryptoHandler::create(key_string)?,
        })
    }

    pub(crate) fn encode(&self, data: &[u8; PLAINTEXT_SIZE]) -> anyhow::Result<[u8; MSG_SIZE]> {
        let ciphertext = self.crypto_handler.encrypt(data)?;
        let mut data_encoded = [0u8; MSG_SIZE];
        data_encoded[0..KEY_ID_SIZE].copy_from_slice(&self.crypto_handler.id);
        data_encoded[KEY_ID_SIZE..].copy_from_slice(&ciphertext);
        Ok(data_encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::protocol::PLAINTEXT_SIZE;
    use openssl::rand::rand_bytes;

    fn make_parser() -> DataParser {
        let key = CryptoHandler::gen_key().expect("key generation failed");
        DataParser::create(&key).expect("parser creation failed")
    }

    #[test]
    fn decode_data_accepts_valid_ciphertext() {
        let parser = make_parser();

        let mut payload = [0u8; PLAINTEXT_SIZE];
        rand_bytes(&mut payload).unwrap();

        let encoded = parser.encode(&payload).expect("encode failed");

        let (key_id, ciphertext) = super::super::decode(&encoded).expect("decode failed");
        assert_eq!(key_id.to_vec(), parser.crypto_handler.id.to_vec());
        assert_eq!(parser.crypto_handler.decrypt(ciphertext).expect("decrypt failed"), payload);
    }
}
