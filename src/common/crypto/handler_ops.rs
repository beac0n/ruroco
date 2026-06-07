#[cfg(any(feature = "with-client", feature = "with-server"))]
use anyhow::Context;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use openssl::cipher::Cipher;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use openssl::cipher_ctx::CipherCtx;
#[cfg(feature = "with-client")]
use openssl::rand::rand_bytes;

use super::handler::CryptoHandler;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use crate::common::protocol::{CIPHERTEXT_SIZE, PLAINTEXT_SIZE};

#[cfg(any(feature = "with-client", feature = "with-server"))]
const IV_SIZE: usize = 12;
#[cfg(any(feature = "with-client", feature = "with-server"))]
const TAG_SIZE: usize = 16;

// AES-256-GCM-SIV (RFC 8452): nonce-misuse-resistant AEAD. A repeated nonce only ever leaks
// whether two plaintexts were identical (which the replay check already rejects), never the
// catastrophic key recovery of plain GCM. Requires OpenSSL >= 3.2 (fetched from the default
// provider). The counter lives inside the authenticated plaintext, so replay protection is bound
// to the message; the 12-byte nonce stays random to keep packets indistinguishable on the wire.
#[cfg(any(feature = "with-client", feature = "with-server"))]
fn gcm_siv() -> anyhow::Result<Cipher> {
    Cipher::fetch(None, "AES-256-GCM-SIV", None)
        .with_context(|| "Could not fetch AES-256-GCM-SIV cipher (requires OpenSSL >= 3.2)")
}

#[cfg(feature = "with-client")]
impl CryptoHandler {
    pub(crate) fn encrypt(
        &self,
        plaintext: &[u8; PLAINTEXT_SIZE],
    ) -> anyhow::Result<[u8; CIPHERTEXT_SIZE]> {
        let mut iv = [0u8; IV_SIZE];
        rand_bytes(&mut iv).with_context(|| "Could not generate IV")?;

        let cipher = gcm_siv()?;
        let mut ctx = CipherCtx::new().with_context(|| "Could not create cipher context")?;
        ctx.encrypt_init(Some(&cipher), Some(&self.key), Some(&iv))
            .with_context(|| "Could not init encryption")?;

        let mut ciphertext = Vec::with_capacity(PLAINTEXT_SIZE);
        ctx.cipher_update_vec(plaintext, &mut ciphertext)
            .with_context(|| "Could not update encryption")?;
        ctx.cipher_final_vec(&mut ciphertext).with_context(|| "Could not finalize encryption")?;

        if ciphertext.len() != PLAINTEXT_SIZE {
            anyhow::bail!("ciphertext length mismatch");
        }

        let mut tag = [0u8; TAG_SIZE];
        ctx.tag(&mut tag).with_context(|| "Could not get tag")?;

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

        let cipher = gcm_siv()?;
        let mut ctx = CipherCtx::new().with_context(|| "Could not create cipher context")?;
        ctx.decrypt_init(Some(&cipher), Some(&self.key), Some(iv))
            .with_context(|| "Could not init decryption")?;
        ctx.set_tag(tag).with_context(|| "Could not set tag")?;

        let mut plaintext = Vec::with_capacity(PLAINTEXT_SIZE);
        ctx.cipher_update_vec(ciphertext, &mut plaintext)
            .with_context(|| "Could not update decryption")?;
        ctx.cipher_final_vec(&mut plaintext)
            .with_context(|| "Could not finalize decryption (tag mismatch)")?;

        if plaintext.len() != PLAINTEXT_SIZE {
            anyhow::bail!("Plaintext length mismatch");
        }

        plaintext.as_slice().try_into().with_context(|| "Could not convert plaintext")
    }
}
