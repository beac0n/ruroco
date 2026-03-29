pub(crate) mod client_data;
#[cfg(feature = "with-client")]
mod client_data_client;
#[cfg(feature = "with-server")]
pub(crate) mod client_data_server;
pub(crate) mod constants;
pub(crate) mod parser;
pub(crate) mod serialization;
#[cfg(feature = "with-server")]
pub(crate) mod serialization_server;

pub(crate) use constants::{CIPHERTEXT_SIZE, KEY_ID_SIZE, MSG_SIZE, PLAINTEXT_SIZE};
