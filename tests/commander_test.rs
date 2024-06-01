#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::time::Duration;
    use std::{fs, thread};

    use rand::distributions::{Alphanumeric, DistString};

    use ruroco::commander::{Commander, CommanderCommand};
    use ruroco::common::{init_logger, socket_file_path};

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        return format!("{rand_str}{suffix}");
    }

    #[test]
    fn test_run() {
        init_logger();
        let _ = fs::remove_file(socket_file_path());

        let start_test_filename = gen_file_name("_start.test");
        let stop_test_filename = gen_file_name("_stop.test");

        let start = format!("touch {}", &start_test_filename);
        let stop = format!("touch {}", &stop_test_filename);
        println!("{}", socket_file_path());

        assert!(!Path::new(&socket_file_path()).exists());

        let mut config = HashMap::new();
        config.insert("default".to_string(), CommanderCommand::create(start, stop, 0));
        thread::spawn(move || Commander::create(config).run().expect("commander terminated"));

        thread::sleep(Duration::from_secs(1));

        assert!(Path::new(&socket_file_path()).exists())
    }
}
