#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use ruroco::config_server::ConfigServer;
    use ruroco::server::Server;

    #[test]
    fn test_create_from_path() {
        let path =
            env::current_dir().unwrap_or(PathBuf::from("/tmp")).join("tests").join("config.toml");

        assert_eq!(
            Server::create_from_path(path),
            Server::create(ConfigServer {
                address: String::from("127.0.0.1:8080"),
                config_dir: PathBuf::from("/etc/ruroco/"),
                ..Default::default()
            })
        );
    }
}
