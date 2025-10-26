use crate::common::crypto_handler::{CryptoHandler, KEY_ID_SIZE};
use openssl::rand::rand_bytes;

pub const MSG_SIZE: usize = 201;

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
        let max_cipher_size = 192;
        let ciphertext = self.crypto_handler.encrypt(data_to_encrypt)?;
        let ciphertext_len = ciphertext.len();
        if ciphertext_len > max_cipher_size {
            Err(format!(
                "Too much data, must be at most {max_cipher_size} bytes, \
                but was {ciphertext_len} bytes. Reduce command name length."
            ))?
        }

        let data_to_send_len = MSG_SIZE; // 1 zero byte prefix + 8 bytes id + 192 bytes encrypted data
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
                return Ok(i + 1);
            }
        }

        Err("Could not get index of zero byte")?
    }
}
