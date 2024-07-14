#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{fs, thread};

    use rand::distributions::{Alphanumeric, DistString};

    use ruroco::commander::Commander;
    use ruroco::common::init_logger;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }

    #[test]
    fn test_run() {
        init_logger();
        let socket_file_path = Path::new("/tmp/ruroco/ruroco.socket");
        let _ = fs::remove_file(socket_file_path);
        assert!(!socket_file_path.exists());

        let mut config = HashMap::new();
        config.insert(String::from("default"), format!("touch {}", gen_file_name(".test")));
        thread::spawn(move || {
            Commander::create(
                config,
                String::from(""),
                String::from(""),
                PathBuf::from("/tmp/ruroco"),
            )
            .run()
            .expect("commander terminated")
        });

        thread::sleep(Duration::from_secs(1));

        assert!(socket_file_path.exists());
    }
}
