use crate::common::crypto::{CIPHERTEXT_SIZE, KEY_ID_SIZE};

pub(crate) mod client_data;
pub(crate) mod parser;
pub(crate) mod serialization;

pub(crate) const MSG_SIZE: usize = KEY_ID_SIZE + CIPHERTEXT_SIZE;
