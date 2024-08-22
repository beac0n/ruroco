#[cfg(test)]
mod tests {
    use ruroco::common::{get_blocklist_path, get_socket_path, resolve_path};
    use std::path::PathBuf;
    use std::{env, fs};

    #[test]
    fn test_get_blocklist_path() {
        assert_eq!(
            get_blocklist_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/blocklist.toml")
        );
    }

    #[test]
    fn test_get_socket_path() {
        assert_eq!(
            get_socket_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/ruroco.socket")
        );
    }

    #[test]
    fn test_resolve_absolute_path() {
        assert_eq!(resolve_path(&PathBuf::from("/foo/bar/baz")), PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let _ = fs::create_dir_all(PathBuf::from("./tmp/foo"));
        assert_eq!(
            resolve_path(&PathBuf::from("./tmp/foo")),
            env::current_dir().unwrap().join("tmp/foo")
        );
    }
}
