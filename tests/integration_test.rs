#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::{fs, thread};

    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;

    use ruroco::blocklist::Blocklist;
    use ruroco::client::{gen, send};
    use ruroco::commander::Commander;
    use ruroco::common::{get_blocklist_path, get_socket_path, time};
    use ruroco::config_server::ConfigServer;
    use ruroco::server::Server;

    const TEST_IP: &str = "192.168.178.123";

    struct TestData {
        test_file_path: PathBuf,
        socket_path: PathBuf,
        blocklist_path: PathBuf,
        public_pem_path: PathBuf,
        private_pem_path: PathBuf,
        server_address: String,
        config_dir: PathBuf,
    }

    impl TestData {
        fn create() -> TestData {
            let test_folder_path = PathBuf::from("/dev/shm").join(gen_file_name(""));
            let private_pem_dir = test_folder_path.join("private");
            let _ = fs::create_dir_all(&test_folder_path);
            let _ = fs::create_dir_all(&private_pem_dir);

            TestData {
                config_dir: test_folder_path.clone(),
                test_file_path: test_folder_path.join(gen_file_name(".test")),
                socket_path: get_socket_path(&test_folder_path),
                blocklist_path: get_blocklist_path(&test_folder_path),
                public_pem_path: test_folder_path.join(gen_file_name(".pem")),
                private_pem_path: private_pem_dir.join(gen_file_name(".pem")),
                server_address: format!("127.0.0.1:{}", rand::thread_rng().gen_range(1024..65535)),
            }
        }
    }

    #[test]
    fn test_too_late() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 0, time().unwrap(), None, true);
        assert_file_paths(&test_data, false, false);
    }

    #[test]
    fn test_is_blocked() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        let now = time().unwrap();
        run_client_send(&test_data, 5, now, None, true);
        let _ = fs::remove_file(&test_data.test_file_path);

        run_client_send(&test_data, 5, now, None, true);
        assert_file_paths(&test_data, false, true);
    }

    #[test]
    fn test_ip_mismatch() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 1, time().unwrap(), Some(String::from(TEST_IP)), true);
        assert_file_paths(&test_data, false, false);
    }

    #[test]
    fn test_ip_mismatch_not_strict() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 1, time().unwrap(), Some(String::from(TEST_IP)), false);
        assert_file_paths(&test_data, true, true);
    }

    #[test]
    fn test_ip_match() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 1, time().unwrap(), Some(String::from("127.0.0.1")), true);
        assert_file_paths(&test_data, true, true);
    }

    #[test]
    fn test_is_valid() {
        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 1, time().unwrap(), None, true);
        let blocked_list_0 = get_blocked_list(&test_data);

        run_client_send(&test_data, 5, time().unwrap(), None, true);
        let blocked_list_1 = get_blocked_list(&test_data);

        assert_file_paths(&test_data, true, true);

        assert_eq!(blocked_list_0.len(), 1);
        assert_eq!(blocked_list_1.len(), 1);
        assert_ne!(blocked_list_0.first(), blocked_list_1.first());
    }

    fn run_client_gen(file_paths: &TestData) {
        let key_size = 1024;

        gen(file_paths.private_pem_path.clone(), file_paths.public_pem_path.clone(), key_size)
            .unwrap();
    }

    fn assert_file_paths(
        test_data: &TestData,
        with_test_file_exists: bool,
        with_block_list_exists: bool,
    ) {
        let test_file_exists = test_data.test_file_path.exists();
        let private_exists = test_data.private_pem_path.exists();
        let public_exists = test_data.public_pem_path.exists();
        let socket_exists = test_data.socket_path.exists();
        let blocklist_exists = test_data.blocklist_path.exists();

        let _ = fs::remove_dir_all(&test_data.config_dir);

        assert_eq!(test_file_exists, with_test_file_exists);
        assert_eq!(blocklist_exists, with_block_list_exists);
        assert!(private_exists);
        assert!(public_exists);
        assert!(socket_exists);
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }

    fn get_blocked_list(test_data: &TestData) -> Vec<u128> {
        let blocklist = Blocklist::create(&test_data.config_dir);
        blocklist.get().clone()
    }

    fn run_client_send(
        test_data: &TestData,
        deadline: u16,
        now: u128,
        ip: Option<String>,
        strict: bool,
    ) {
        let pem_path = test_data.private_pem_path.clone();
        let address = test_data.server_address.to_string();
        let command = String::from("default");
        send(pem_path, address, command, deadline, strict, ip, now).unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for files to be written and blocklist to be updated
    }

    fn run_commander(test_data: &TestData) {
        let config_dir = test_data.config_dir.clone();
        let mut commands = HashMap::new();
        commands.insert(String::from("default"), format!("touch {:?}", &test_data.test_file_path));

        thread::spawn(move || {
            Commander::create(ConfigServer {
                commands,
                config_dir,
                ..Default::default()
            })
            .run()
            .expect("commander terminated")
        });
    }

    fn run_server(test_data: &TestData) {
        let address = test_data.server_address.clone();
        let config_dir = test_data.config_dir.clone();

        thread::spawn(move || {
            Server::create(ConfigServer {
                address,
                config_dir,
                ..Default::default()
            })
            .expect("could not create server")
            .run()
            .expect("server terminated")
        });
    }
}
