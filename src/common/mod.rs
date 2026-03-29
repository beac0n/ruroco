#[cfg(target_os = "android")]
pub mod android_util;
pub(crate) mod crypto;
pub(crate) mod fs;
pub(crate) mod logging;
pub(crate) mod protocol;

pub(crate) use crypto::blake2b_u64;
pub use crypto::get_random_range;
pub use crypto::get_random_string;
pub(crate) use crypto::handler as crypto_handler;
pub(crate) use fs::change_file_ownership;
pub(crate) use fs::resolve_path;
#[cfg(any(feature = "with-server", feature = "with-gui"))]
pub(crate) use logging::error;
pub(crate) use logging::info;
pub(crate) use protocol::client_data;
pub(crate) use protocol::parser as data_parser;
#[cfg(feature = "with-client")]
pub(crate) use protocol::serialization::serialize_ip;
