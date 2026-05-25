#[cfg(any(feature = "with-client", feature = "with-server"))]
use anyhow::Context;
#[cfg(feature = "with-client")]
use openssl::rand::rand_bytes;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use openssl::symm::{Cipher, Crypter, Mode};

use super::handler::CryptoHandler;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use crate::common::protocol::{CIPHERTEXT_SIZE, PLAINTEXT_SIZE};

#[cfg(any(feature = "with-client", feature = "with-server"))]
const IV_SIZE: usize = 12;
#[cfg(any(feature = "with-client", feature = "with-server"))]
const TAG_SIZE: usize = 16;

#[cfg(feature = "with-client")]
impl CryptoHandler {
    pub(crate) fn encrypt(
        &self,
        plaintext: &[u8; PLAINTEXT_SIZE],
    ) -> anyhow::Result<[u8; CIPHERTEXT_SIZE]> {
        let cipher = Cipher::aes_256_gcm();
        let mut iv = [0u8; IV_SIZE];
        rand_bytes(&mut iv).with_context(|| "Could not generate IV")?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &self.key, Some(&iv))
            .with_context(|| "Could not create crypter")?;

        let mut ciphertext = [0u8; PLAINTEXT_SIZE];
        let count = crypter
            .update(plaintext, &mut ciphertext)
            .with_context(|| "Could not update crypter")?;

        if count != PLAINTEXT_SIZE {
            anyhow::bail!("ciphertext length mismatch");
        }

        if crypter.finalize(&mut []).with_context(|| "Could not finalize crypter")? != 0 {
            anyhow::bail!("GCM finalize returned unexpected bytes");
        }

        let mut tag = [0u8; TAG_SIZE];
        crypter.get_tag(&mut tag).with_context(|| "Could not get tag from crypter")?;

        let mut out = [0u8; CIPHERTEXT_SIZE];
        out[0..IV_SIZE].copy_from_slice(&iv);
        out[IV_SIZE..IV_SIZE + TAG_SIZE].copy_from_slice(&tag);
        out[IV_SIZE + TAG_SIZE..].copy_from_slice(&ciphertext);

        Ok(out)
    }
}

#[cfg(feature = "with-server")]
impl CryptoHandler {
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
