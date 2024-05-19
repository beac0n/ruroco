pub mod lib {
    use std::env;
    use std::path::PathBuf;
    use std::time::{SystemTime, SystemTimeError};

    pub const SOCKET_DIR: &str = "/tmp/ruroco/";
    pub const SOCKET_FILE_PATH: &str = "/tmp/ruroco/ruroco.socket";

    pub fn init_logger() {
        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    pub fn time() -> Result<u128, SystemTimeError> {
        Ok(SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos())
    }

    pub fn get_path(file_name: &str) -> PathBuf {
        return match env::current_dir() {
            Ok(dir) => dir.join(file_name),
            Err(_) => PathBuf::from(file_name),
        };
    }
}
