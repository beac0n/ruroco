/// persists the blocked list of deadlines
pub mod blocklist;
/// responsible for executing the commands that are defined in the config file
pub mod commander;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::data::{ClientData, CommanderData};
use crate::common::data_parser::{DataParser, MSG_SIZE};
use crate::common::{error, info, time_from_ntp};
use crate::config::config_server::{CliServer, ConfigServer};
use crate::server::blocklist::Blocklist;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Server {
    config: ConfigServer,
    crypto_handlers: HashMap<Vec<u8>, CryptoHandler>,
    socket: UdpSocket,
    client_recv_data: [u8; MSG_SIZE],
    socket_path: PathBuf,
    blocklist: Blocklist,
}

impl Server {
    pub fn create_from_path(path: &Path) -> Result<Server, String> {
        match fs::read_to_string(path) {
            Ok(config) => Server::create(ConfigServer::deserialize(&config)?, None),
            Err(e) => Err(format!("Could not read {path:?}: {e}")),
        }
    }

    pub fn create(config: ConfigServer, address: Option<String>) -> Result<Server, String> {
        config.validate_ips()?;

        Ok(Server {
            crypto_handlers: config.create_crypto_handlers()?,
            socket: config.create_server_udp_socket(address)?,
            client_recv_data: [0u8; MSG_SIZE],
            socket_path: config.get_commander_unix_socket_path(),
            blocklist: config.create_blocklist(),
            config,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        info(&format!("Running server on {:?}", self.socket));
        loop {
            let data = self.socket.recv_from(&mut self.client_recv_data);
            self.run_loop_iteration(data);
        }
    }

    fn run_loop_iteration(&mut self, data: std::io::Result<(usize, SocketAddr)>) -> Option<String> {
        let error_msg = match data {
            Ok((count, src)) if count != MSG_SIZE => {
                Some(format!("Invalid read count {count}, expected {MSG_SIZE} from {src}"))
            }
            Ok((count, src)) => {
                info(&format!("Successfully received {count} bytes from {src}"));
                self.decrypt().map_or_else(Some, |d| self.validate_and_send_command(&d, src.ip()))
            }
            Err(e) => Some(format!("Could not receive bytes from socket: {e}")),
        };

        error_msg.inspect(|s| error(s))
    }

    fn decrypt(&mut self) -> Result<Vec<u8>, String> {
        let (key_id, encrypted_data) = DataParser::decode_data(&self.client_recv_data)?;

        self.crypto_handlers
            .get(key_id)
            .map(|crypto_handler| crypto_handler.decrypt(encrypted_data))
            .unwrap_or_else(|| Err(format!("Could not find key for id {key_id:X?}")))
    }

    fn validate_and_send_command(
        &mut self,
        decrypted_data: &[u8],
        src_ip_addr: IpAddr,
    ) -> Option<String> {
        let src_ip = match src_ip_addr.to_string() {
            // see https://datatracker.ietf.org/doc/html/rfc5156#section-2.2
            ip if ip.starts_with("::ffff:") => ip.replacen("::ffff:", "", 1),
            ip => ip,
        };

        match self.decode(decrypted_data) {
            Ok((now_ns, client_data)) if now_ns > client_data.deadline() => {
                let deadline = client_data.deadline();
                Some(format!("Invalid deadline - now {now_ns} is after {deadline}"))
            }
            Ok((_, client_data)) if !self.config.ips.contains(&client_data.destination_ip()) => {
                let destination_ip = client_data.destination_ip();
                let ips = &self.config.ips;
                Some(format!("Invalid host IP - expected {ips:?} to contain {destination_ip}"))
            }
            Ok((_, client_data)) if self.blocklist.is_blocked(client_data.deadline()) => {
                Some(format!("Invalid deadline - {} is on blocklist", client_data.deadline()))
            }
            Ok((_, client_data)) if client_data.validate_source_ip(&src_ip) => {
                let client_src_ip_str = client_data.source_ip().unwrap();
                Some(format!("Invalid source IP - expected {client_src_ip_str}, actual {src_ip}"))
            }
            Ok((now_ns, client_data)) => {
                let command_name = client_data.c.to_string();
                let ip = client_data.source_ip().unwrap_or(src_ip);
                info(&format!("Valid data - trying {command_name} with {ip}"));

                self.send_command(CommanderData { command_name, ip });
                self.update_block_list(now_ns, client_data.deadline());
                None
            }
            Err(e) => Some(format!("Could not decode data: {e}")),
        }
    }

    fn update_block_list(&mut self, now_ns: u128, deadline_ns: u128) {
        self.blocklist.clean(now_ns);
        self.blocklist.add(deadline_ns);
        self.blocklist.save();
    }

    fn send_command(&self, data: CommanderData) {
        match self.write_to_socket(data) {
            Ok(_) => info("Successfully sent data to commander"),
            Err(e) => error(&format!(
                "Could not send data to commander via socket {:?}: {e}",
                &self.socket_path
            )),
        }
    }

    fn write_to_socket(&self, data: CommanderData) -> Result<(), String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| format!("Could not connect to socket {:?}: {e}", self.socket_path))?;

        let data_to_send = data.serialize()?;

        stream.write_all(data_to_send.as_bytes()).map_err(|e| {
            format!("Could not write {data_to_send} to socket {:?}: {e}", self.socket_path)
        })?;

        stream
            .flush()
            .map_err(|e| format!("Could not flush stream for {:?}: {e}", self.socket_path))?;
        Ok(())
    }

    fn decode(&self, decrypted_data: &[u8]) -> Result<(u128, ClientData), String> {
        match ClientData::deserialize(decrypted_data) {
            Ok(data) => Ok((time_from_ntp(&self.config.ntp)?, data)),
            Err(e) => Err(e),
        }
    }
}

pub fn run_server(server: CliServer) -> Result<(), String> {
    Server::create_from_path(&server.config)?.run()
}

#[cfg(test)]
mod tests {
    use crate::client::gen::Generator;
    use crate::common::data_parser::MSG_SIZE;
    use crate::config::config_server::{CliServer, ConfigServer};
    use crate::server::Server;
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};
    use rand::Rng;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::{env, fs, io};

    impl PartialEq for Server {
        fn eq(&self, other: &Self) -> bool {
            self.client_recv_data == other.client_recv_data
                && self.socket_path == other.socket_path
                && self.blocklist == other.blocklist
        }
    }

    #[test]
    fn test_print_help() {
        let result = CliServer::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_create_server_udp_socket() {
        env::remove_var("LISTEN_PID");
        env::remove_var("RUROCO_LISTEN_ADDRESS");
        let socket = ConfigServer::default().create_server_udp_socket(None).unwrap();
        let result = socket.local_addr().unwrap();
        assert_eq!(format!("{result:?}"), "[::]:34020");
    }

    #[test]
    fn test_create_invalid_pid() {
        env::set_var("LISTEN_PID", "12345");

        let config_dir =
            env::current_dir().unwrap_or(PathBuf::from("/tmp")).join("tests").join("conf_dir");

        let result = Server::create(
            ConfigServer {
                config_dir,
                ..Default::default()
            },
            None,
        );

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "LISTEN_PID was set, but not to our PID");
    }

    #[test]
    fn test_create_from_invalid_path() {
        let path = env::current_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("tests")
            .join("files")
            .join("config_invalid.toml");

        let result = Server::create_from_path(&path);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("TOML parse error at line 1, column 1"));
    }

    #[test]
    fn test_create_from_invalid_toml_path() {
        let result = Server::create_from_path(&PathBuf::from("/tmp/path/does/not/exist"));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            r#"Could not read "/tmp/path/does/not/exist": No such file or directory (os error 2)"#
        );
    }

    #[test]
    fn test_create_from_path() {
        let server_port = rand::rng().random_range(1024..65535);
        env::set_var("RUROCO_LISTEN_ADDRESS", format!("[::]:{server_port}"));

        let tests_dir_path = env::current_dir().unwrap_or(PathBuf::from("/tmp")).join("tests");
        let conf_path = tests_dir_path.join("files").join("config.toml");
        let config_dir = tests_dir_path.join("conf_dir");

        let res_path = Server::create_from_path(&conf_path).unwrap();
        let res_create = Server::create(
            ConfigServer {
                config_dir,
                ..Default::default()
            },
            Some("127.0.0.1:8081".to_string()),
        )
        .unwrap();

        assert_eq!(res_path, res_create);
    }

    #[test]
    fn test_loop_iteration_invalid_read_count() {
        let mut server = create_server();
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));

        assert_eq!(
            server.run_loop_iteration(success_data).unwrap(),
            format!("Invalid read count 0, expected {MSG_SIZE} from 127.0.0.1:8080")
        );
    }

    #[test]
    fn test_loop_iteration_decrypt_error() {
        let mut server = create_server();
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((MSG_SIZE, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));
        assert_eq!(
            server.run_loop_iteration(success_data).unwrap(),
            "Could not find key for id [0, 0, 0, 0, 0, 0, 0, 0]"
        );
    }

    #[test]
    fn test_loop_iteration_error() {
        let mut server = create_server();
        let error_data: io::Result<(usize, SocketAddr)> =
            Err(io::Error::other("An error occurred"));

        assert_eq!(
            server.run_loop_iteration(error_data).unwrap(),
            "Could not receive bytes from socket: An error occurred"
        );
    }

    fn create_server() -> Server {
        let test_folder_path = PathBuf::from("/dev/shm").join(gen_file_name(""));
        let _ = fs::create_dir_all(&test_folder_path);

        let key_path = test_folder_path.join(gen_file_name(".key"));
        let key = Generator::create().unwrap().gen().expect("could not generate key");
        fs::write(&key_path, key).expect("failed to write key");

        Server::create(
            ConfigServer {
                config_dir: test_folder_path.clone(),
                ..Default::default()
            },
            Some(format!("127.0.0.1:{}", rand::rng().random_range(1024..65535))),
        )
        .expect("could not create server")
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
