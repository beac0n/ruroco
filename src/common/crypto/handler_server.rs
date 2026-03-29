use crate::common::crypto::handler::CryptoHandler;
use crate::common::protocol::{CIPHERTEXT_SIZE, PLAINTEXT_SIZE};
use anyhow::Context;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs;
use std::path::Path;

const IV_SIZE: usize = 12;
const TAG_SIZE: usize = 16;

impl CryptoHandler {
    pub(crate) fn from_key_path(key_path: &Path) -> anyhow::Result<Self> {
        let key = fs::read_to_string(key_path).with_context(|| "Could not read key")?;
        Self::create(&key)
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
            anyhow::bail!("Plaintext length mismatch");
        }

        decrypter.set_tag(tag).with_context(|| "Could not set tag")?;

        if decrypter.finalize(&mut []).with_context(|| "Could not finalize decrypter")? != 0 {
            anyhow::bail!("GCM finalize returned unexpected bytes");
        }

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::CryptoHandler;

    #[test]
    fn test_from_key_path_nonexistent() {
        let result = CryptoHandler::from_key_path(std::path::Path::new("/tmp/no_such_key.key"));
        assert!(result.is_err());
    }
}
