use std::path::PathBuf;
use std::time::SystemTime;

use openssl::rsa::Padding;

pub const RSA_PADDING: Padding = Padding::PKCS1;
pub const PADDING_SIZE: usize = 11; // see https://www.rfc-editor.org/rfc/rfc3447#section-7.2.1

pub fn init_logger() {
    let _ = env_logger::builder().filter_level(log::LevelFilter::Info).try_init();
}

pub fn time() -> Result<u128, String> {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Could not get duration since: {e}"))?;
    Ok(duration.as_nanos())
}

pub fn get_socket_path(config_dir: PathBuf) -> PathBuf {
    let mut config_dir_clone = config_dir.clone();
    config_dir_clone.push("ruroco.socket");
    config_dir_clone
}
