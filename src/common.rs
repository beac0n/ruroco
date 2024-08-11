use log::error;
use openssl::rsa::Padding;
use std::path::PathBuf;
use std::time::SystemTime;
use std::{env, fs};

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

pub fn get_socket_path(config_dir: &PathBuf) -> PathBuf {
    resolve_path(config_dir).join("ruroco.socket")
}

pub fn get_blocklist_path(config_dir: &PathBuf) -> PathBuf {
    resolve_path(config_dir).join("blocklist.toml")
}

pub fn resolve_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let mut full_path = match env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                error!("Could not get current directory: {e}");
                return path.to_path_buf();
            }
        };
        full_path.push(path);
        match fs::canonicalize(&full_path) {
            Ok(p) => p,
            Err(e) => {
                error!("Could not canonicalize {:?}: {e}", &full_path);
                full_path
            }
        }
    }
}
