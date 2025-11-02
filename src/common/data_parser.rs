use crate::common::crypto_handler::{CryptoHandler, IV_SIZE, KEY_ID_SIZE, TAG_SIZE};
use openssl::rand::rand_bytes;

pub const MSG_SIZE: usize = 500;

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

    pub fn encode_data(&self, data_to_encrypt: &[u8]) -> Result<[u8; MSG_SIZE], String> {
        let max_cipher_size = MSG_SIZE - 1 - KEY_ID_SIZE;
        let ciphertext = self.crypto_handler.encrypt(data_to_encrypt)?;
        let ciphertext_len = ciphertext.len();
        if ciphertext_len > max_cipher_size {
            Err(format!(
                "Too much data, must be at most {max_cipher_size} bytes, \
                but was {ciphertext_len} bytes. Reduce command name length."
            ))?
        }

        let data_to_send_len = MSG_SIZE; // 1 zero byte prefix + 8 bytes id + remaining encrypted data
        let mut data_to_send = [0u8; MSG_SIZE];
        let ciphertext_start = data_to_send_len - ciphertext_len;
        let key_id_start = ciphertext_start - KEY_ID_SIZE;
        data_to_send[ciphertext_start..].copy_from_slice(&ciphertext);
        data_to_send[key_id_start..ciphertext_start].copy_from_slice(&self.crypto_handler.id);

        if key_id_start > 1 {
            rand_bytes(&mut data_to_send[..key_id_start - 1])
                .map_err(|e| format!("Could not generate random bytes: {e}"))?;
            for b in &mut data_to_send[..key_id_start - 1] {
                if *b == 0 {
                    *b = 1;
                }
            }
        }

        Ok(data_to_send)
    }
    pub fn decode_data(data: &[u8]) -> Result<(&[u8], &[u8]), String> {
        let key_id_start = DataParser::get_key_id_start_index(data)?;
        let key_id = &data[key_id_start..key_id_start + KEY_ID_SIZE];
        let encrypted_data = &data[key_id_start + KEY_ID_SIZE..];
        Ok((key_id, encrypted_data))
    }

    fn get_key_id_start_index(data: &[u8]) -> Result<usize, String> {
        for (i, &b) in data.iter().enumerate() {
            if b == 0 {
                let key_id_start = i + 1;
                let encrypted_data_start = key_id_start + KEY_ID_SIZE;
                let data_len = data.len();
                return if encrypted_data_start >= data_len {
                    Err("Key id overlaps packet boundary".to_string())
                } else if data_len - encrypted_data_start <= (IV_SIZE + TAG_SIZE) {
                    Err("Encrypted payload shorter than IV + tag".to_string())
                } else {
                    Ok(key_id_start)
                };
            }
        }

        Err("Could not get index of zero byte")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let err = DataParser::decode_data(&packet).unwrap_err();
        assert_eq!(err, "Encrypted payload shorter than IV + tag");
    }

    #[test]
    fn decode_data_accepts_valid_ciphertext() {
        let parser = parser();
        let payload = b"hello world";
        let encoded = parser.encode_data(payload).expect("encode failed");

        let (key_id, ciphertext) = DataParser::decode_data(&encoded).expect("decode failed");
        assert_eq!(key_id, parser.crypto_handler.id);
        assert_eq!(parser.crypto_handler.decrypt(ciphertext).expect("decrypt failed"), payload);
    }

    #[test]
    fn decode_data_rejects_payload_with_no_zero_delimiter() {
        let packet = [1u8; MSG_SIZE];
        let err = DataParser::decode_data(&packet).unwrap_err();
        assert_eq!(err, "Could not get index of zero byte");
    }

    #[test]
    fn decode_data_rejects_key_overlapping_boundary() {
        let mut packet = [1u8; MSG_SIZE];
        packet[MSG_SIZE - 1] = 0;
        let err = DataParser::decode_data(&packet).unwrap_err();
        assert_eq!(err, "Key id overlaps packet boundary");
    }
}
