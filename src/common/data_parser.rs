use crate::common::crypto_handler::{CryptoHandler, CIPHERTEXT_SIZE, KEY_ID_SIZE, PLAINTEXT_SIZE};

pub const MSG_SIZE: usize = KEY_ID_SIZE + CIPHERTEXT_SIZE;

#[derive(Debug)]
pub struct DataParser {
    crypto_handler: CryptoHandler,
}

impl DataParser {
    pub fn create(key_string: &str) -> Result<Self, String> {
        Ok(DataParser {
            crypto_handler: CryptoHandler::create(key_string)?,
        })
    }

    pub fn encode(&self, data: &[u8; PLAINTEXT_SIZE]) -> Result<[u8; MSG_SIZE], String> {
        let ciphertext = self.crypto_handler.encrypt(data)?;
        let mut data_encoded = [0u8; MSG_SIZE];
        data_encoded[0..KEY_ID_SIZE].copy_from_slice(&self.crypto_handler.id);
        data_encoded[KEY_ID_SIZE..].copy_from_slice(&ciphertext);
        Ok(data_encoded)
    }
    pub fn decode(
        data: &[u8; MSG_SIZE],
    ) -> Result<(&[u8; KEY_ID_SIZE], &[u8; CIPHERTEXT_SIZE]), String> {
        let data_decoded = <&[u8; CIPHERTEXT_SIZE]>::try_from(&data[KEY_ID_SIZE..CIPHERTEXT_SIZE])
            .map_err(|e| format!("Could not get decoded data for ciphertext: {e}"))?;
        let key_id = <&[u8; KEY_ID_SIZE]>::try_from(&data[0..KEY_ID_SIZE])
            .map_err(|e| format!("Could not get decoded data for key id: {e}"))?;
        Ok((key_id, data_decoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::crypto_handler::{IV_SIZE, TAG_SIZE};
    use openssl::rand::rand_bytes;

    fn parser() -> DataParser {
        let key = CryptoHandler::gen_key().expect("key generation failed");
        DataParser::create(&key).expect("parser creation failed")
    }

    #[test]
    fn decode_data_rejects_payload_without_ciphertext() {
        let parser = parser();
        let mut packet = [1u8; MSG_SIZE];
        let key_id_start = MSG_SIZE - TAG_SIZE - IV_SIZE - KEY_ID_SIZE;
        packet[key_id_start - 1] = 0;
        packet[key_id_start..key_id_start + KEY_ID_SIZE].copy_from_slice(&parser.crypto_handler.id);

        let err = DataParser::decode(&packet).unwrap_err();
        assert_eq!(err, "Encrypted payload shorter than IV + tag");
    }

    #[test]
    fn decode_data_accepts_valid_ciphertext() {
        let parser = parser();

        let mut payload = [0u8; 57];
        rand_bytes(&mut payload).unwrap();

        let encoded = parser.encode(&payload).expect("encode failed");

        let (key_id, ciphertext) = DataParser::decode(&encoded).expect("decode failed");
        assert_eq!(key_id.to_vec(), parser.crypto_handler.id.to_vec());
        assert_eq!(parser.crypto_handler.decrypt(ciphertext).expect("decrypt failed"), payload);
    }

    #[test]
    fn decode_data_rejects_payload_with_no_zero_delimiter() {
        let packet = [1u8; MSG_SIZE];
        let err = DataParser::decode(&packet).unwrap_err();
        assert_eq!(err, "Could not get index of zero byte");
    }

    #[test]
    fn decode_data_rejects_key_overlapping_boundary() {
        let mut packet = [1u8; MSG_SIZE];
        packet[MSG_SIZE - 1] = 0;
        let err = DataParser::decode(&packet).unwrap_err();
        assert_eq!(err, "Key id overlaps packet boundary");
    }
}
