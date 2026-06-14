#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) mod client_data;
#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) mod constants;
pub(crate) mod parser;
pub(crate) mod serialization;

#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) use constants::{CIPHERTEXT_SIZE, KEY_ID_SIZE, MSG_SIZE, PLAINTEXT_SIZE};
