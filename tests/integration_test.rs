#[cfg(test)]
mod tests {
    use ruroco::client::config::SendCommand;
    use ruroco::client::counter::Counter;
    use ruroco::client::gen::Generator;
    use ruroco::client::send::Sender;
    use ruroco::common::{get_random_range, get_random_string};
    use ruroco::server::blocklist::Blocklist;
    use ruroco::server::commander::Commander;
    use ruroco::server::config::ConfigServer;
    use ruroco::server::util::get_commander_unix_socket_path;
    use ruroco::server::Server;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::{env, fs, thread};

    const TEST_IP_V4: &str = "192.168.178.123";
    const TEST_IP_V6: &str = "dead:beef:dead:beef:dead:beef:dead:beef";

    struct TestData {
        test_file_path: PathBuf,
        socket_path: PathBuf,
        blocklist_path: PathBuf,
        key_path: PathBuf,
        server_address: String,
        config_dir: PathBuf,
        test_file_exists: bool,
        block_list_exists: bool,
        client_sent_ip: Option<String>,
        strict: bool,
    }

    impl TestData {
        fn create() -> TestData {
            let test_folder_path = tempfile::tempdir().unwrap().keep();
            env::set_var("RUROCO_CONF_DIR", &test_folder_path);

            TestData {
                config_dir: test_folder_path.clone(),
                test_file_path: test_folder_path.join(TestData::gen_file_name(".test")),
                socket_path: get_commander_unix_socket_path(&test_folder_path),
                blocklist_path: Blocklist::get_blocklist_path(&test_folder_path),
                key_path: test_folder_path.join(TestData::gen_file_name(".key")),
                server_address: Self::get_server_address("[::]"),
                test_file_exists: false,
                block_list_exists: false,
                client_sent_ip: None,
                strict: true,
            }
        }

        fn get_server_address(host: &str) -> String {
            let server_port = get_random_range(1024, 65535).unwrap();
            format!("{host}:{server_port}")
        }

        fn gen_file_name(suffix: &str) -> String {
            let rand_str = get_random_string(16).unwrap();
            format!("{rand_str}{suffix}")
        }

        fn run_client_gen(&self) {
            let key = Generator::create()
                .expect("could not create key generator")
                .gen()
                .expect("could not generate key");
            fs::write(&self.key_path, key).expect("failed to write key")
        }

        fn get_blocked_list(&self) -> HashMap<u64, u128> {
            let blocklist = Blocklist::create(&self.config_dir).unwrap();
            blocklist.get().clone()
        }

        fn run_client_send(&self) {
            let sender = Sender::create(SendCommand {
                address: self.server_address.to_string(),
                key: fs::read_to_string(&self.key_path).expect("failed to read key"),
                command: "default".to_string(),
                permissive: !self.strict,
                ip: self.client_sent_ip.clone(),
                ipv4: false,
                ipv6: false,
            })
            .expect("could not create sender");

            sender.send().expect("could not send command");
            thread::sleep(Duration::from_secs(10)); // wait for files to be written and blocklist to be updated
        }

        fn run_commander(&self) {
            let config_dir = self.config_dir.clone();
            let mut commands = HashMap::new();
            commands.insert(
                "default".to_string(),
                format!("echo -n $RUROCO_IP > {:?}", &self.test_file_path),
            );

            thread::spawn(move || {
                Commander::create(ConfigServer {
                    commands,
                    config_dir,
                    ..Default::default()
                })
                .unwrap()
                .run()
                .expect("commander terminated")
            });
            thread::sleep(Duration::from_secs(2));
        }

        fn run_server(&self) {
            let config_dir = self.config_dir.clone();
            let server_address = self.server_address.clone();
            thread::spawn(move || {
                Server::create(
                    ConfigServer {
                        config_dir,
                        ips: vec![
                            "127.0.0.1".parse().unwrap(),
                            "::1".parse().unwrap(),
                            "::".parse().unwrap(),
                        ],
                        ..Default::default()
                    },
                    Some(server_address),
                )
                .expect("could not create server")
                .run()
                .expect("server terminated")
            });
            thread::sleep(Duration::from_secs(2));
        }

        fn with_ipv6(&mut self) -> &mut TestData {
            self.server_address = Self::get_server_address("[::]");
            self
        }

        fn with_ipv4(&mut self) -> &mut TestData {
            self.server_address = Self::get_server_address("127.0.0.1");
            self
        }

        fn with_ip(&mut self, ip: &str) -> &mut TestData {
            self.client_sent_ip = Some(ip.to_string());
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
            let key_exists = self.key_path.exists();
            let socket_exists = self.socket_path.exists();
            let blocklist_exists = self.blocklist_path.exists();

            let _ = fs::remove_dir_all(&self.config_dir);

            assert_eq!(test_file_exists, self.test_file_exists);
            assert_eq!(blocklist_exists, self.block_list_exists);
            assert!(key_exists);
            assert!(socket_exists);
        }
    }

    #[test]
    fn test_is_blocked() {
        let mut test_data: TestData = TestData::create();

        test_data.run_client_gen();
        test_data.run_commander();
        test_data.run_server();

        test_data.run_client_send();
        let _ = fs::remove_file(&test_data.test_file_path);
        let mut counter = Counter::create(Sender::get_counter_path().unwrap(), 0).unwrap();
        counter.dec().unwrap();

        test_data.run_client_send();
        test_data.with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_ip_mismatch_v4() {
        ip_mismatch_test(TestData::create(), TEST_IP_V4);
    }

    #[test]
    fn test_ip_mismatch_v6() {
        let mut test_data = TestData::create();
        test_data.with_ipv6();
        ip_mismatch_test(test_data, TEST_IP_V6);
    }

    fn ip_mismatch_test(mut test_data: TestData, ip: &str) {
        test_data.run_client_gen();
        test_data.run_commander();
        test_data.run_server();

        test_data.with_ip(ip).run_client_send();
        test_data.assert_file_paths();
    }

    #[test]
    fn test_ip_mismatch_not_strict_ipv4() {
        ip_mismatch_not_strict_test(TestData::create(), TEST_IP_V4);
    }

    #[test]
    fn test_ip_mismatch_not_strict_ipv6() {
        let mut test_data = TestData::create();
        test_data.with_ipv6();
        ip_mismatch_not_strict_test(test_data, TEST_IP_V6);
    }

    fn ip_mismatch_not_strict_test(mut test_data: TestData, ip: &str) {
        test_data.run_client_gen();
        test_data.run_commander();
        test_data.run_server();

        test_data.with_ip(ip).with_strict(false).run_client_send();

        assert_eq!(
            fs::read_to_string(&test_data.test_file_path).expect("could not read file"),
            ip.to_string()
        );
        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_ip_match_v4() {
        let mut test_data = TestData::create();
        test_data.with_ipv4();
        ip_match_test(test_data, "127.0.0.1");
    }

    #[test]
    fn test_ip_match_v6() {
        let mut test_data = TestData::create();
        test_data.with_ipv6();
        ip_match_test(test_data, "::1");
    }

    fn ip_match_test(mut test_data: TestData, ip: &str) {
        test_data.run_client_gen();
        test_data.run_commander();
        test_data.run_server();

        test_data.with_ip(ip).run_client_send();

        assert_eq!(
            fs::read_to_string(&test_data.test_file_path).expect("could not read file"),
            ip.to_string()
        );
        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();
    }

    #[test]
    fn test_is_valid_ipv4() {
        is_valid_test(TestData::create());
    }

    #[test]
    fn test_is_valid_ipv6() {
        let mut test_data = TestData::create();
        test_data.with_ipv6();
        is_valid_test(test_data);
    }

    fn is_valid_test(mut test_data: TestData) {
        test_data.run_client_gen();
        test_data.run_commander();
        test_data.run_server();

        test_data.run_client_send();
        let blocked_list_0 = test_data.get_blocked_list();

        test_data.run_client_send();
        let blocked_list_1 = test_data.get_blocked_list();

        test_data.with_test_file_exists().with_block_list_exists().assert_file_paths();

        assert_eq!(blocked_list_0.len(), 1);
        assert_eq!(blocked_list_1.len(), 1);
        assert_ne!(blocked_list_0.values().last(), blocked_list_1.values().last());
    }

}
