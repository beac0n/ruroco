use crate::common::crypto_handler::{CryptoHandler, CIPHERTEXT_SIZE, KEY_ID_SIZE, PLAINTEXT_SIZE};
use anyhow::Context;

pub(crate) const MSG_SIZE: usize = KEY_ID_SIZE + CIPHERTEXT_SIZE;

#[derive(Debug)]
pub(crate) struct DataParser {
    crypto_handler: CryptoHandler,
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
    pub(crate) fn decode(
        data: &[u8; MSG_SIZE],
    ) -> anyhow::Result<(&[u8; KEY_ID_SIZE], &[u8; CIPHERTEXT_SIZE])> {
        let data_decoded = <&[u8; CIPHERTEXT_SIZE]>::try_from(&data[KEY_ID_SIZE..])
            .with_context(|| "Could not get decoded data for ciphertext")?;
        let key_id = <&[u8; KEY_ID_SIZE]>::try_from(&data[0..KEY_ID_SIZE])
            .with_context(|| "Could not get decoded data for key id")?;
        Ok((key_id, data_decoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::rand::rand_bytes;

    fn parser() -> DataParser {
        let key = CryptoHandler::gen_key().expect("key generation failed");
        DataParser::create(&key).expect("parser creation failed")
    }

    #[test]
    fn decode_data_accepts_valid_ciphertext() {
        let parser = parser();

        let mut payload = [0u8; PLAINTEXT_SIZE];
        rand_bytes(&mut payload).unwrap();

        let encoded = parser.encode(&payload).expect("encode failed");

        let (key_id, ciphertext) = DataParser::decode(&encoded).expect("decode failed");
        assert_eq!(key_id.to_vec(), parser.crypto_handler.id.to_vec());
        assert_eq!(parser.crypto_handler.decrypt(ciphertext).expect("decrypt failed"), payload);
    }
}
