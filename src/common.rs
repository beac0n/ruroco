use std::time::{SystemTime, SystemTimeError};

pub const SOCKET_DIR: &str = "/etc/ruroco/";
pub const SOCKET_FILE_PATH: &str = "/etc/ruroco/ruroco.socket";

pub fn init_logger() {
    let _ = env_logger::builder().filter_level(log::LevelFilter::Info).try_init();
}

pub fn time() -> Result<u128, SystemTimeError> {
    Ok(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_nanos())
}
