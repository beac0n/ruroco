#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::{env, fs, thread};

    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;

    use ruroco::blocklist::Blocklist;
    use ruroco::client::{gen, send};
    use ruroco::commander::Commander;
    use ruroco::common::{get_blocklist_path, get_socket_path, init_logger, time};
    use ruroco::server::Server;

    struct TestData {
        test_file_path: PathBuf,
        socket_path: PathBuf,
        blocklist_path: PathBuf,
        public_pem_path: PathBuf,
        private_pem_path: PathBuf,
        server_address: String,
    }

    impl TestData {
        fn create() -> TestData {
            TestData {
                test_file_path: PathBuf::from(gen_file_name(".test")),
                socket_path: get_socket_path(&env::current_dir().unwrap()),
                blocklist_path: get_blocklist_path(&env::current_dir().unwrap()),
                public_pem_path: PathBuf::from(gen_file_name(".pem")),
                private_pem_path: get_private_pem_path(),
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

        run_client_send(&test_data, 0, time().unwrap());
        assert_file_paths(&test_data, false, false);
    }

    #[test]
    fn test_is_blocked() {
        init_logger();

        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        let now = time().unwrap();
        run_client_send(&test_data, 5, now);
        let _ = fs::remove_file(&test_data.test_file_path);

        run_client_send(&test_data, 5, now);
        assert_file_paths(&test_data, false, true);
    }

    #[test]
    fn test_integration_test() {
        init_logger();

        let test_data: TestData = TestData::create();

        run_client_gen(&test_data);
        run_server(&test_data);
        run_commander(&test_data);

        run_client_send(&test_data, 1, time().unwrap());
        let blocked_list_0 = get_blocked_list();

        run_client_send(&test_data, 5, time().unwrap());
        let blocked_list_1 = get_blocked_list();

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

        let _ = fs::remove_file(&test_data.test_file_path);
        let _ = fs::remove_file(&test_data.private_pem_path);
        let _ = fs::remove_file(&test_data.public_pem_path);
        let _ = fs::remove_file(&test_data.socket_path);
        let _ = fs::remove_file(&test_data.blocklist_path);

        assert_eq!(test_file_exists, with_test_file_exists);
        assert!(private_exists);
        assert!(public_exists);
        assert!(socket_exists);
        assert_eq!(blocklist_exists, with_block_list_exists);
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        return format!("{rand_str}{suffix}");
    }

    fn get_blocked_list() -> Vec<u128> {
        let blocklist = Blocklist::create(&env::current_dir().unwrap());
        blocklist.get().clone()
    }

    fn run_client_send(test_data: &TestData, deadline: u16, now: u128) {
        send(
            test_data.private_pem_path.clone(),
            test_data.server_address.to_string(),
            String::from("default"),
            deadline,
            now,
        )
        .unwrap();
        thread::sleep(Duration::from_secs(1)); // wait for commands to be executed
    }

    fn run_commander(test_data: &TestData) {
        let mut config = HashMap::new();
        config.insert(String::from("default"), format!("touch {:?}", &test_data.test_file_path));

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
    }

    fn run_server(test_data: &TestData) {
        let server_address_clone = test_data.server_address.clone();

        thread::spawn(move || {
            Server::create(env::current_dir().unwrap(), server_address_clone)
                .expect("could not create server")
                .run()
                .expect("server terminated")
        });
    }

    fn get_private_pem_path() -> PathBuf {
        let mut private_pem_path = env::current_dir().unwrap();
        private_pem_path.push("tests");
        private_pem_path.push(gen_file_name(".pem"));
        private_pem_path
    }
}
