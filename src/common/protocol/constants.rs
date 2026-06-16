/// Wire protocol version, carried as the first byte of the authenticated plaintext.
/// Bump this whenever the plaintext layout or packet framing changes incompatibly.
pub(crate) const PROTOCOL_VERSION: u8 = 1;

pub(crate) const PLAINTEXT_SIZE: usize = 58;
pub(crate) const CIPHERTEXT_SIZE: usize = 86;
pub(crate) const KEY_ID_SIZE: usize = 8;
pub(crate) const MSG_SIZE: usize = KEY_ID_SIZE + CIPHERTEXT_SIZE;
