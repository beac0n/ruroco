#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{env, fs, thread};

    use rand::distributions::{Alphanumeric, DistString};

    use ruroco::commander::Commander;
    use ruroco::common::init_logger;
    use ruroco::config_server::ConfigServer;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }

    #[test]
    fn test_create_from_path() {
        let mut commands = HashMap::new();
        commands.insert(
            String::from("default"),
            String::from("touch /tmp/ruroco_test/start.test /tmp/ruroco_test/stop.test"),
        );

        let path =
            env::current_dir().unwrap_or(PathBuf::from("/tmp")).join("tests").join("config.toml");

        assert_eq!(
            Commander::create_from_path(path),
            Ok(Commander::create(ConfigServer {
                address: String::from("127.0.0.1:8080"),
                config_dir: PathBuf::from("/etc/ruroco/"),
                socket_user: String::from("ruroco"),
                socket_group: String::from("ruroco"),
                commands,
            }))
        );
    }

    #[test]
    fn test_run() {
        init_logger();
        let socket_file_path = Path::new("/tmp/ruroco/ruroco.socket");
        let _ = fs::remove_file(socket_file_path);
        assert!(!socket_file_path.exists());

        let mut commands = HashMap::new();
        commands.insert(String::from("default"), format!("touch {}", gen_file_name(".test")));
        thread::spawn(move || {
            Commander::create(ConfigServer {
                commands,
                config_dir: PathBuf::from("/tmp/ruroco"),
                ..Default::default()
            })
            .run()
            .expect("commander terminated")
        });

        thread::sleep(Duration::from_secs(1));

        assert!(socket_file_path.exists());
    }
}
