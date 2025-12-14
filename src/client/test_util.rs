#![cfg(test)]
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::TempDir;

static TEST_CONF_DIR: OnceLock<TempDir> = OnceLock::new();
pub fn get_conf_dir() -> Result<PathBuf, String> {
    Ok(TEST_CONF_DIR.get_or_init(|| TempDir::new().expect("temp conf dir")).path().to_path_buf())
}
