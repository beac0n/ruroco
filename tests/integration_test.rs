#[cfg(test)]
mod tests {
    use std::{env, fs, thread};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;

    use ruroco::blocklist::Blocklist;
    use ruroco::client::{gen, send};
    use ruroco::commander::Commander;
    use ruroco::common::{get_blocklist_path, get_socket_path, init_logger};
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

        let curr_dir = &env::current_dir().unwrap();
        let test_filename = gen_file_name(".test");
        let socket_path = get_socket_path(curr_dir);
        let blocklist_path = get_blocklist_path(curr_dir);
        let public_pem_path = PathBuf::from(gen_file_name(".pem"));
        let mut private_pem_path = env::current_dir().unwrap();
        private_pem_path.push("tests");
        private_pem_path.push(gen_file_name(".pem"));

        gen(private_pem_path.clone(), public_pem_path.clone(), key_size).unwrap();

        let server_address_for_server = server_address.clone();

        thread::spawn(move || {
            Server::create(env::current_dir().unwrap(), server_address_for_server)
                .expect("could not create server")
                .run()
                .expect("server terminated")
        });

        let mut config = HashMap::new();
        config.insert(String::from("default"), format!("touch {}", &test_filename));

        thread::spawn(move || {
            Commander::create(
                config,
                String::from(""),
                String::from(""),
                env::current_dir().unwrap(),
            )
            .run()
            .expect("commander terminated")
        });

        send(private_pem_path.clone(), server_address.to_string(), String::from("default"), 1)
            .unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for commands to be executed
        let blocklist = Blocklist::create(curr_dir);
        let blocked_list_0 = blocklist.get();

        let _ = fs::remove_file(&test_filename);

        send(private_pem_path.clone(), server_address.to_string(), String::from("default"), 5)
            .unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for commands to be executed
        let blocklist = Blocklist::create(curr_dir);
        let blocked_list_1 = blocklist.get();

        let start_test_exists = Path::new(&test_filename).exists();
        let private_exists = private_pem_path.exists();
        let public_exists = public_pem_path.exists();
        let socket_exists = socket_path.exists();
        let blocklist_exists = blocklist_path.exists();

        let _ = fs::remove_file(&test_filename);
        let _ = fs::remove_file(&private_pem_path);
        let _ = fs::remove_file(&public_pem_path);
        let _ = fs::remove_file(socket_path);
        let _ = fs::remove_file(blocklist_path);

        assert!(start_test_exists);
        assert!(private_exists);
        assert!(public_exists);
        assert!(socket_exists);
        assert!(blocklist_exists);

        assert_eq!(blocked_list_0.len(), 1);
        assert_eq!(blocked_list_1.len(), 1);
        assert_ne!(blocked_list_0.first(), blocked_list_1.first());
    }
}
