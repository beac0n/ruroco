#[cfg(test)]
mod tests {
    use ruroco::config_server::ConfigServer;
    use ruroco::server::Server;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_create_server_udp_socket() {
        env::remove_var("LISTEN_PID");
        let result = ConfigServer::default().create_server_udp_socket(None).unwrap();
        assert_eq!(format!("{result:?}"), "UdpSocket { addr: [::]:34020, fd: 3 }");
    }

    #[test]
    fn test_create_invalid_pid() {
        env::set_var("LISTEN_PID", "12345");

        let config_dir =
            env::current_dir().unwrap_or(PathBuf::from("/tmp")).join("tests").join("conf_dir");

        let result = Server::create(
            ConfigServer {
                config_dir,
                ..Default::default()
            },
            None,
        );

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "LISTEN_PID was set, but not to our PID");
    }

    #[test]
    fn test_create_from_invalid_path() {
        let path = env::current_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("tests")
            .join("files")
            .join("config_invalid.toml");

        let result = Server::create_from_path(path);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("TOML parse error at line 1, column 1"));
    }

    #[test]
    fn test_create_from_invalid_toml_path() {
        let result = Server::create_from_path(PathBuf::from("/tmp/path/does/not/exist"));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            r#"Could not read "/tmp/path/does/not/exist": No such file or directory (os error 2)"#
        );
    }
}
