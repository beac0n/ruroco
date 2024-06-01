use std::time::{SystemTime, SystemTimeError};

use openssl::rsa::Padding;

pub const RSA_PADDING: Padding = Padding::PKCS1;

pub fn init_logger() {
    let _ = env_logger::builder().filter_level(log::LevelFilter::Info).try_init();
}

pub fn time() -> Result<u128, SystemTimeError> {
    Ok(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_nanos())
}
