#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{fs, thread};

    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;

    use ruroco::client::{gen, send};
    use ruroco::commander::Commander;
    use ruroco::common::init_logger;
    use ruroco::config::CommanderCommand;
    use ruroco::server::Server;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        return format!("{rand_str}{suffix}");
    }

    #[test]
    fn test_integration_key_size_1024() {
        run_integration_test(1024);
    }

    #[test]
    fn test_integration_key_size_2048() {
        run_integration_test(2048);
    }

    #[test]
    fn test_integration_key_size_4096() {
        run_integration_test(4096);
    }

    #[test]
    fn test_integration_key_size_8192() {
        run_integration_test(8192);
    }

    fn run_integration_test(key_size: u32) {
        init_logger();

        let server_address = format!("127.0.0.1:{}", rand::thread_rng().gen_range(1024..65535));

        let start_test_filename = gen_file_name("_start.test");
        let stop_test_filename = gen_file_name("_stop.test");

        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(private_pem_path.clone(), public_pem_path.clone(), key_size).unwrap();

        let server_address_for_server = server_address.clone();

        thread::spawn(move || {
            Server::create(
                public_pem_path,
                server_address_for_server,
                1,
                PathBuf::from("/tmp/ruroco/ruroco.socket"),
            )
            .expect("could not create server")
            .run()
            .expect("server terminated")
        });

        let start = format!("touch {}", &start_test_filename);
        let stop = format!("touch {}", &stop_test_filename);

        let mut config = HashMap::new();
        config.insert(String::from("default"), CommanderCommand::create(start, stop, 0));

        thread::spawn(move || {
            Commander::create(
                config,
                String::from(""),
                String::from(""),
                PathBuf::from("/tmp/ruroco/ruroco.socket"),
            )
            .run()
            .expect("commander terminated")
        });

        send(private_pem_path.clone(), server_address.to_string(), String::from("default")).unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for commands to be executed

        let _ = fs::remove_file(&start_test_filename);
        let _ = fs::remove_file(&stop_test_filename);

        send(private_pem_path.clone(), server_address.to_string(), String::from("default")).unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for commands to be executed

        let start_test_exists = Path::new(&start_test_filename).exists();
        let stop_test_exists = Path::new(&stop_test_filename).exists();
        let private_exists = Path::new(&private_file).exists();
        let public_exists = Path::new(&public_file).exists();

        let _ = fs::remove_file(&start_test_filename);
        let _ = fs::remove_file(&stop_test_filename);
        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert!(start_test_exists);
        assert!(stop_test_exists);
        assert!(private_exists);
        assert!(public_exists);
    }
}
