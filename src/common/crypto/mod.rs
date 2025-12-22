use anyhow::Context;
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use openssl::rand::rand_bytes;

pub(crate) mod handler;

pub(crate) use handler::{CryptoHandler, CIPHERTEXT_SIZE, KEY_ID_SIZE, PLAINTEXT_SIZE};

pub(crate) fn blake2b_u64(s: &str) -> anyhow::Result<u64> {
    let mut hasher = Blake2bVar::new(8)
        .with_context(|| format!("Could not create Blake2b hasher for string {s}"))?;
    hasher.update(s.as_bytes());
    let mut out = [0u8; 8];
    hasher
        .finalize_variable(&mut out)
        .with_context(|| format!("Could not finalize Blake2b hash for string {s}"))?;
    Ok(u64::from_be_bytes(out))
}

pub fn get_random_string(len: usize) -> anyhow::Result<String> {
    let mut buf = vec![0u8; len];
    rand_bytes(&mut buf).with_context(|| "Could not generate random")?;
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    Ok(buf
        .iter()
        .map(|b| chars.as_bytes()[(*b as usize) % chars.len()] as char)
        .collect())
}

pub fn get_random_range(from: u16, to: u16) -> anyhow::Result<u16> {
    let mut buf = [0u8; 2];
    rand_bytes(&mut buf).with_context(|| "Could not generate number")?;

    let span = to - from;
    let v = u16::from_be_bytes(buf) % span;
    Ok(from + v)
}
