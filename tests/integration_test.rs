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
        test_file_exists: bool,
        block_list_exists: bool,
        deadline: u16,
        now: Option<u128>,
        ip: Option<String>,
        strict: bool,
    }

    impl TestData {
        fn create() -> TestData {
            let test_folder_path = PathBuf::from("/dev/shm").join(TestData::gen_file_name(""));
            let private_pem_dir = test_folder_path.join("private");
            let _ = fs::create_dir_all(&test_folder_path);
            let _ = fs::create_dir_all(&private_pem_dir);

            TestData {
                config_dir: test_folder_path.clone(),
                test_file_path: test_folder_path.join(TestData::gen_file_name(".test")),
                socket_path: get_socket_path(&test_folder_path),
                blocklist_path: get_blocklist_path(&test_folder_path),
                public_pem_path: test_folder_path.join(TestData::gen_file_name(".pem")),
                private_pem_path: private_pem_dir.join(TestData::gen_file_name(".pem")),
                server_address: format!("127.0.0.1:{}", rand::thread_rng().gen_range(1024..65535)),
                test_file_exists: false,
                block_list_exists: false,
                deadline: 1,
                now: None,
                ip: None,
                strict: true,
            }
        }

        fn gen_file_name(suffix: &str) -> String {
            let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            format!("{rand_str}{suffix}")
        }

        fn run_client_gen(&self) {
            let key_size = 1024;

            gen(self.private_pem_path.clone(), self.public_pem_path.clone(), key_size).unwrap();
        }

        fn get_blocked_list(&self) -> Vec<u128> {
            let blocklist = Blocklist::create(&self.config_dir);
            blocklist.get().clone()
        }

        fn run_client_send(&self) {
            let now = if self.now.is_none() {
                time().unwrap()
            } else {
                self.now.unwrap()
            };

            let pem_path = self.private_pem_path.clone();
            let address = self.server_address.to_string();
            let command = String::from("default");
            send(pem_path, address, command, self.deadline, self.strict, self.ip.clone(), now)
                .unwrap();
            thread::sleep(Duration::from_secs(2)); // wait for files to be written and blocklist to be updated
        }

        fn run_commander(&self) {
            let config_dir = self.config_dir.clone();
            let mut commands = HashMap::new();
            commands.insert(
                String::from("default"),
                format!("echo -n $RUROCO_IP > {:?}", &self.test_file_path),
            );

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

        fn run_server(&self) {
            let address = self.server_address.clone();
            let config_dir = self.config_dir.clone();

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

        fn with_deadline(&mut self, deadline: u16) -> &mut TestData {
            self.deadline = deadline;
            self
        }

        fn with_now(&mut self, now: u128) -> &mut TestData {
            self.now = Some(now);
            self
        }

        fn with_ip(&mut self, ip: &str) -> &mut TestData {
            self.ip = Some(String::from(ip));
            self
        }

        fn with_strict(&mut self, strict: bool) -> &mut TestData {
            self.strict = strict;
            self
        }

        fn with_test_file_exists(&mut self) -> &mut TestData {
            self.test_file_exists = true;
            self
        }

        fn with_block_list_exists(&mut self) -> &mut TestData {
            self.block_list_exists = true;
            self
        }

        fn assert_file_paths(&self) {
            let test_file_exists = self.test_file_path.exists();
            let private_exists = self.private_pem_path.exists();
            let public_exists = self.public_pem_path.exists();
            let socket_exists = self.socket_path.exists();
            let blocklist_exists = self.blocklist_path.exists();

            let _ = fs::remove_dir_all(&self.config_dir);

            assert_eq!(test_file_exists, self.test_file_exists);
            assert_eq!(blocklist_exists, self.block_list_exists);
            assert!(private_exists);
            assert!(public_exists);
            assert!(socket_exists);
        }
    }

    #[test]
    fn test_too_late() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        test_data.with_deadline(0).run_client_send();

        test_data.assert_file_paths();
    }

    #[test]
    fn test_is_blocked() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        let now = time().unwrap();
        test_data.with_deadline(5).with_now(now).run_client_send();
        let _ = fs::remove_file(&test_data.test_file_path);

        test_data.with_deadline(5).with_now(now).run_client_send();
        test_data.with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_ip_mismatch() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        test_data.with_ip(TEST_IP).run_client_send();
        test_data.assert_file_paths();
    }

    #[test]
    fn test_ip_mismatch_not_strict() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        test_data.with_ip(TEST_IP).with_strict(false).run_client_send();

        assert_eq!(fs::read_to_string(&test_data.test_file_path).unwrap(), String::from(TEST_IP));
        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_ip_match() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        test_data.with_ip("127.0.0.1").run_client_send();

        assert_eq!(
            fs::read_to_string(&test_data.test_file_path).unwrap(),
            String::from("127.0.0.1")
        );
        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_is_valid() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_server();
        test_data.run_commander();

        test_data.run_client_send();
        let blocked_list_0 = test_data.get_blocked_list();

        test_data.with_deadline(5).run_client_send();
        let blocked_list_1 = test_data.get_blocked_list();

        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();

        assert_eq!(blocked_list_0.len(), 1);
        assert_eq!(blocked_list_1.len(), 1);
        assert_ne!(blocked_list_0.first(), blocked_list_1.first());
    }
}
