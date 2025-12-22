/// persists the blocked list of deadlines
pub mod blocklist;
mod blocklist_serialization;
/// responsible for executing the commands that are defined in the config file
pub mod commander;
mod commander_data;
/// data structures for loading configuration files and using CLI arguments for server services
pub mod config;
pub mod util;

use crate::common::client_data::ClientData;
use crate::common::crypto_handler::{CryptoHandler, KEY_ID_SIZE, PLAINTEXT_SIZE};
use crate::common::data_parser::{DataParser, MSG_SIZE};
use crate::common::{error, info};
use crate::server::blocklist::Blocklist;
use crate::server::config::{CliServer, ConfigServer};
use anyhow::{anyhow, Context};
use commander_data::CommanderData;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Server {
    config: ConfigServer,
    crypto_handlers: HashMap<[u8; KEY_ID_SIZE], CryptoHandler>,
    socket: UdpSocket,
    client_recv_data: [u8; MSG_SIZE],
    socket_path: PathBuf,
    blocklist: Blocklist,
}

impl Server {
    fn create_from_path(path: &Path) -> anyhow::Result<Server> {
        match fs::read_to_string(path) {
            Ok(config) => Server::create(ConfigServer::deserialize(&config)?, None),
            Err(e) => Err(anyhow!("Could not read {path:?}: {e}")),
        }
    }

    pub fn create(config: ConfigServer, address: Option<String>) -> anyhow::Result<Server> {
        Ok(Server {
            crypto_handlers: config.create_crypto_handlers()?,
            socket: config.create_server_udp_socket(address)?,
            client_recv_data: [0u8; MSG_SIZE],
            socket_path: config.get_commander_unix_socket_path(),
            blocklist: config.create_blocklist()?,
            config,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        info(&format!("Running server on {:?}", self.socket));
        loop {
            let data = self.socket.recv_from(&mut self.client_recv_data);
            if let Err(e) = self.run_loop_iteration(data) {
                error(format!("{e}"));
            }
        }
    }

    fn run_loop_iteration(
        &mut self,
        data: std::io::Result<(usize, SocketAddr)>,
    ) -> anyhow::Result<()> {
        match data {
            Ok((count, src)) if count != MSG_SIZE => {
                Err(anyhow!("Invalid read count {count}, expected {MSG_SIZE} from {src}"))
            }
            Ok((count, src)) => {
                info(&format!("Successfully received {count} bytes from {src}"));
                let (key_id, plaintext) = self.decrypt()?;
                self.validate_and_send_command(key_id, plaintext, src.ip())
            }
            Err(e) => Err(anyhow!("Could not receive bytes from socket: {e}")),
        }
    }

    fn decrypt(&mut self) -> anyhow::Result<([u8; 8], [u8; PLAINTEXT_SIZE])> {
        let (key_id, encrypted_data) = DataParser::decode(&self.client_recv_data)?;
        let plaintext = self
            .crypto_handlers
            .get(key_id)
            .map(|crypto_handler| crypto_handler.decrypt(encrypted_data))
            .unwrap_or_else(|| Err(anyhow!("Could not find key for id {key_id:X?}")))?;
        Ok((*key_id, plaintext))
    }

    fn validate_and_send_command(
        &mut self,
        key_id: [u8; 8],
        plaintext_data: [u8; PLAINTEXT_SIZE],
        src_ip: IpAddr,
    ) -> anyhow::Result<()> {
        let src_ip = match src_ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
            _ => src_ip,
        };

        match ClientData::deserialize(plaintext_data) {
            Ok(client_data) if self.blocklist.is_blocked(key_id, client_data.counter) => {
                Err(anyhow!("Invalid counter - {} is on blocklist", client_data.counter))
            }
            Ok(client_data) if !self.config.ips.contains(&client_data.dst_ip) => {
                let destination_ip = &client_data.dst_ip;
                let ips = &self.config.ips;
                Err(anyhow!("Invalid host IP - expected {ips:?} to contain {destination_ip}"))
            }
            Ok(client_data) if client_data.is_source_ip_invalid(src_ip) => {
                let client_src_ip_str =
                    client_data.src_ip.map(|i| i.to_string()).unwrap_or("none".to_string());
                Err(anyhow!("Invalid source IP - expected {client_src_ip_str}, actual {src_ip}"))
            }
            Ok(client_data) => {
                let cmd = client_data.cmd_hash;
                let server_counter = self.blocklist.get_counter(key_id);
                let client_counter = client_data.counter;
                let ip = client_data.src_ip.unwrap_or(src_ip);
                info(&format!("Valid data - trying cmd {cmd} and counter {client_counter}|{server_counter:?} with {ip}"));

                self.send_command(CommanderData { cmd_hash: cmd, ip });
                self.update_block_list(key_id, client_data.counter);
                Ok(())
            }
            Err(e) => Err(anyhow!("Could not decode data: {e}")),
        }
    }

    fn update_block_list(&mut self, key_id: [u8; 8], counter: u128) {
        self.blocklist.add(key_id, counter);
        if let Err(e) = self.blocklist.save() {
            error(format!("Could not update block list: {e}"))
        }
    }

    fn send_command(&self, data: CommanderData) {
        match self.write_to_socket(data) {
            Ok(_) => info("Successfully sent data to commander"),
            Err(e) => error(format!(
                "Could not send data to commander via socket {:?}: {e}",
                &self.socket_path
            )),
        }
    }

    fn write_to_socket(&self, data: CommanderData) -> anyhow::Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("Could not connect to socket {:?}", self.socket_path))?;

        let data_to_send = data.serialize();
        stream.write_all(&data_to_send).with_context(|| {
            format!("Could not write {data_to_send:?} to socket {:?}", self.socket_path)
        })?;

        stream
            .flush()
            .with_context(|| format!("Could not flush stream for {:?}", self.socket_path))?;
        Ok(())
    }
}

pub fn run_server(server: CliServer) -> anyhow::Result<()> {
    Server::create_from_path(&server.config)?.run()
}

#[cfg(test)]
mod tests {
    use crate::client::gen::Generator;
    use crate::common::data_parser::MSG_SIZE;
    use crate::common::{get_random_range, get_random_string};
    use crate::server::config::{CliServer, ConfigServer};
    use crate::server::Server;
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
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
        env::remove_var("LISTEN_FDS");
        env::remove_var("LISTEN_PID");
        env::remove_var("RUROCO_LISTEN_ADDRESS");
        let socket = ConfigServer::default().create_server_udp_socket(None).unwrap();
        let result = socket.local_addr().unwrap();
        assert_eq!(format!("{result:?}"), "[::]:34020");
    }

    #[test]
    fn test_create_invalid_pid() {
        env::set_var("LISTEN_PID", "12345");
        env::set_var("LISTEN_FDS", "1");
        env::remove_var("RUROCO_LISTEN_ADDRESS");

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
        assert_eq!(
            result.err().unwrap().to_string(),
            "LISTEN_PID (12345) does not match current PID"
        );
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
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("TOML parse error") || msg.contains("Could not create ConfigServer from"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_create_from_invalid_toml_path() {
        let result = Server::create_from_path(&PathBuf::from("/tmp/path/does/not/exist"));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            r#"Could not read "/tmp/path/does/not/exist": No such file or directory (os error 2)"#
        );
    }

    #[test]
    fn test_create_from_path() {
        let server_port = get_random_range(1024, 65535).unwrap();
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
        let mut server = create_server().expect("could not create server");
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));

        assert_eq!(
            server.run_loop_iteration(success_data).unwrap_err().to_string(),
            format!("Invalid read count 0, expected {MSG_SIZE} from 127.0.0.1:8080")
        );
    }

    #[test]
    fn test_loop_iteration_decrypt_error() {
        let mut server = create_server().expect("could not create server");
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((MSG_SIZE, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));
        assert_eq!(
            server.run_loop_iteration(success_data).unwrap_err().to_string(),
            "Could not find key for id [0, 0, 0, 0, 0, 0, 0, 0]"
        );
    }

    #[test]
    fn test_loop_iteration_error() {
        let mut server = create_server().expect("could not create server");
        let error_data: io::Result<(usize, SocketAddr)> =
            Err(io::Error::other("An error occurred"));

        assert_eq!(
            server.run_loop_iteration(error_data).unwrap_err().to_string(),
            "Could not receive bytes from socket: An error occurred"
        );
    }

    fn create_server() -> anyhow::Result<Server> {
        let temp_dir = tempfile::tempdir()?;
        let test_folder_path = temp_dir.keep();

        let key_path = test_folder_path.join(gen_file_name(".key"));
        fs::write(&key_path, Generator::create()?.gen()?)?;

        Server::create(
            ConfigServer {
                config_dir: test_folder_path.clone(),
                ..Default::default()
            },
            Some(format!("127.0.0.1:{}", get_random_range(1024, 65535)?)),
        )
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = get_random_string(16).unwrap();
        format!("{rand_str}{suffix}")
    }
}
