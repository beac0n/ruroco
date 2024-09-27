#[cfg(test)]
mod tests {
    use ruroco::common::{get_blocklist_path, get_socket_path, resolve_path, time, time_from_ntp};
    use std::path::PathBuf;
    use std::{env, fs};

    #[test]
    fn test_time_from_ntp_server() {
        let now = time().unwrap();
        let time = time_from_ntp("pool.ntp.org:123").unwrap();
        let diff = time - now;

        println!("{now} || {time}");

        assert!(diff > 0);
        let one_second = 1000000000;
        assert!(diff < one_second);
    }

    #[test]
    fn test_time_from_ntp_system() {
        let now = time().unwrap();
        let time = time_from_ntp("system").unwrap();
        let diff = time - now;

        println!("{diff}");

        assert!(diff > 0);
        let one_milli_second = 1000000;
        assert!(diff < one_milli_second);
    }

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
