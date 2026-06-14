use anyhow::Context;
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;

#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) mod handler;
#[cfg(any(feature = "with-client", feature = "with-server"))]
mod handler_ops;

/// Verify a raw Ed25519 detached signature over `message` against `public_key_pem`.
///
/// Used by the self-update path to authenticate downloaded binaries before they are
/// written to disk. Returns an error if the key cannot be parsed or the signature is
/// invalid.
#[cfg(feature = "with-client")]
pub(crate) fn verify_ed25519(
    public_key_pem: &[u8],
    message: &[u8],
    signature: &[u8],
) -> anyhow::Result<()> {
    use openssl::pkey::PKey;
    use openssl::sign::Verifier;

    let pkey = PKey::public_key_from_pem(public_key_pem)
        .with_context(|| "Could not parse Ed25519 public key")?;
    let mut verifier =
        Verifier::new_without_digest(&pkey).with_context(|| "Could not create Ed25519 verifier")?;
    let valid = verifier
        .verify_oneshot(signature, message)
        .with_context(|| "Could not verify Ed25519 signature")?;
    if !valid {
        anyhow::bail!("Signature verification failed");
    }
    Ok(())
}

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

#[cfg(any(feature = "with-client", feature = "with-server"))]
pub fn get_random_range(from: u16, to: u16) -> anyhow::Result<u16> {
    use openssl::rand::rand_bytes;
    let mut buf = [0u8; 4];
    rand_bytes(&mut buf).with_context(|| "Could not generate number")?;
    let span = (to - from) as u32;
    Ok(from + (u32::from_be_bytes(buf) % span) as u16)
}

#[cfg(feature = "with-client")]
#[cfg(test)]
mod ed25519_tests {
    use super::verify_ed25519;
    use openssl::pkey::PKey;
    use openssl::sign::Signer;

    /// Generate a fresh Ed25519 keypair and return (public_key_pem, signer_key).
    fn keypair() -> (Vec<u8>, PKey<openssl::pkey::Private>) {
        let key = PKey::generate_ed25519().unwrap();
        let pub_pem = key.public_key_to_pem().unwrap();
        (pub_pem, key)
    }

    fn sign(key: &PKey<openssl::pkey::Private>, message: &[u8]) -> Vec<u8> {
        let mut signer = Signer::new_without_digest(key).unwrap();
        signer.sign_oneshot_to_vec(message).unwrap()
    }

    #[test]
    fn test_verify_valid_signature() {
        let (pub_pem, key) = keypair();
        let message = b"some binary contents";
        let sig = sign(&key, message);
        assert!(verify_ed25519(&pub_pem, message, &sig).is_ok());
    }

    #[test]
    fn test_verify_tampered_message_fails() {
        let (pub_pem, key) = keypair();
        let sig = sign(&key, b"original contents");
        let err = verify_ed25519(&pub_pem, b"tampered contents", &sig).unwrap_err();
        assert!(err.to_string().contains("Signature verification failed"));
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let (_pub_pem, signer_key) = keypair();
        let (other_pub_pem, _other_key) = keypair();
        let message = b"some binary contents";
        let sig = sign(&signer_key, message);
        assert!(verify_ed25519(&other_pub_pem, message, &sig).is_err());
    }

    #[test]
    fn test_verify_malformed_public_key_errors() {
        let (_pub_pem, key) = keypair();
        let message = b"some binary contents";
        let sig = sign(&key, message);
        let err = verify_ed25519(b"not a pem", message, &sig).unwrap_err();
        assert!(err.to_string().contains("Could not parse Ed25519 public key"));
    }
}
