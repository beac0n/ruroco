use crate::common;
use std::path::{Path, PathBuf};

pub fn get_commander_unix_socket_path(config_dir: &Path) -> PathBuf {
    common::resolve_path(config_dir).join("ruroco.socket")
}

#[cfg(test)]
mod tests {
    use crate::server::util::get_commander_unix_socket_path;
    use std::path::PathBuf;

    #[test]
    fn test_get_socket_path() {
        assert_eq!(
            get_commander_unix_socket_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/ruroco.socket")
        );
    }
}
