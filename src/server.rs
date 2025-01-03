use crate::blocklist::Blocklist;
use crate::common::{error, info, time_from_ntp, RSA_PADDING, SHA256_DIGEST_LENGTH};
use crate::config_server::{CliServer, ConfigServer};
use crate::data::{ClientData, CommanderData};
use openssl::pkey::Public;
use openssl::rsa::Rsa;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Server {
    config: ConfigServer,
    rsa: HashMap<Vec<u8>, Rsa<Public>>,
    rsa_size: usize,
    socket: UdpSocket,
    client_recv_data: Vec<u8>,
    decrypted_data: Vec<u8>,
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

        let (rsa_size, rsa) = config.create_rsa()?;

        Ok(Server {
            rsa,
            rsa_size,
            socket: config.create_server_udp_socket(address)?,
            decrypted_data: vec![0; rsa_size],
            client_recv_data: vec![0; Self::get_client_recv_data_size(rsa_size)],
            socket_path: config.get_commander_unix_socket_path(),
            blocklist: config.create_blocklist(),
            config,
        })
    }

    fn get_client_recv_data_size(rsa_size: usize) -> usize {
        rsa_size + SHA256_DIGEST_LENGTH
    }

    pub fn run(&mut self) -> Result<(), String> {
        info(&format!("Running server on {:?}", self.socket));
        loop {
            let data = self.receive();
            self.run_loop_iteration(data);
        }
    }

    fn run_loop_iteration(&mut self, data: std::io::Result<(usize, SocketAddr)>) -> Option<String> {
        let data_size = Self::get_client_recv_data_size(self.rsa_size);
        let error_msg = match data {
            Ok((count, src)) if count != data_size => {
                Some(format!("Invalid read count {count}, expected {} from {src}", data_size))
            }
            Ok((count, src)) => {
                info(&format!("Successfully received {count} bytes from {src}"));
                match self.decrypt() {
                    Ok(count) => {
                        self.validate(count, src.ip());
                        None
                    }
                    Err(e) => Some(e),
                }
            }
            Err(e) => Some(format!("Could not recv_from socket from {:?}: {e}", self.socket)),
        };

        self.client_recv_data = vec![0; data_size];
        self.decrypted_data = vec![0; self.rsa_size];

        match error_msg {
            Some(s) => {
                error(&s);
                Some(s)
            }
            None => None,
        }
    }

    fn receive(&mut self) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(&mut self.client_recv_data)
    }

    fn decrypt(&mut self) -> Result<usize, String> {
        let hash_bytes = &self.client_recv_data[..SHA256_DIGEST_LENGTH];
        let encrypted_data = &self.client_recv_data[SHA256_DIGEST_LENGTH..];
        match self
            .rsa
            .get(hash_bytes)
            .map(|rsa| rsa.public_decrypt(encrypted_data, &mut self.decrypted_data, RSA_PADDING))
        {
            Some(r) => r.map_err(|e| format!("Could not decrypt {:X?}: {e}", encrypted_data)),
            None => Err(format!("Could not find public pem for hash {hash_bytes:X?}")),
        }
    }

    fn validate(&mut self, count: usize, src_ip_addr: IpAddr) {
        let src_ip = match src_ip_addr.to_string() {
            // see https://datatracker.ietf.org/doc/html/rfc5156#section-2.2
            ip if ip.starts_with("::ffff:") => ip.replacen("::ffff:", "", 1),
            ip => ip,
        };

        self.decrypted_data.truncate(count);
        match self.decode() {
            Ok((now_ns, client_data)) if now_ns > client_data.deadline() => {
                let deadline = client_data.deadline();
                error(&format!("Invalid deadline - now {now_ns} is after {deadline}"))
            }
            Ok((_, client_data)) if !self.config.ips.contains(&client_data.destination_ip()) => {
                let destination_ip = client_data.destination_ip();
                let ips = &self.config.ips;
                error(&format!("Invalid host IP - expected {ips:?} to contain {destination_ip}"))
            }
            Ok((_, client_data)) if self.blocklist.is_blocked(client_data.deadline()) => {
                error(&format!("Invalid deadline - {} is on blocklist", client_data.deadline()))
            }
            Ok((_, client_data)) if client_data.validate_source_ip(&src_ip) => {
                let client_src_ip_str = client_data.source_ip().unwrap();
                error(&format!("Invalid source IP - expected {client_src_ip_str}, actual {src_ip}"))
            }
            Ok((now_ns, client_data)) => {
                let command_name = client_data.c.to_string();
                let ip = client_data.source_ip().unwrap_or(src_ip);
                info(&format!("Valid data - trying {command_name} with {ip}"));

                self.send_command(CommanderData { command_name, ip });
                self.update_block_list(now_ns, client_data.deadline());
            }
            Err(e) => error(&format!("Could not decode data: {e}")),
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

    fn decode(&self) -> Result<(u128, ClientData), String> {
        match ClientData::deserialize(&self.decrypted_data) {
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
    use crate::client::gen;
    use crate::config_server::ConfigServer;
    use crate::server::Server;
    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::{env, fs, io};

    impl PartialEq for Server {
        fn eq(&self, other: &Self) -> bool {
            self.client_recv_data == other.client_recv_data
                && self.decrypted_data == other.decrypted_data
                && self.socket_path == other.socket_path
                && self.blocklist == other.blocklist
        }
    }

    #[test]
    fn test_create_from_path() {
        let server_port = rand::thread_rng().gen_range(1024..65535);
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
            "Invalid read count 0, expected 160 from 127.0.0.1:8080".to_string()
        );
    }

    #[test]
    fn test_loop_iteration_decrypt_error() {
        let mut server = create_server();
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((160, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));
        assert_eq!(server.run_loop_iteration(success_data).unwrap(), "Could not find public pem for hash [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]");
    }

    #[test]
    fn test_loop_iteration_error() {
        let mut server = create_server();
        let error_data: io::Result<(usize, SocketAddr)> =
            Err(io::Error::new(io::ErrorKind::Other, "An error occurred"));

        assert!(server
            .run_loop_iteration(error_data)
            .unwrap()
            .starts_with("Could not recv_from socket from UdpSocket { addr: 127.0.0.1:"));
    }

    fn create_server() -> Server {
        let test_folder_path = PathBuf::from("/dev/shm").join(gen_file_name(""));
        let private_pem_dir = test_folder_path.join("private");

        let _ = fs::create_dir_all(&test_folder_path);
        let _ = fs::create_dir_all(&private_pem_dir);

        gen(
            &private_pem_dir.join(gen_file_name(".pem")),
            &test_folder_path.join(gen_file_name(".pem")),
            1024,
        )
        .expect("could not generate key");

        Server::create(
            ConfigServer {
                config_dir: test_folder_path.clone(),
                ..Default::default()
            },
            Some(format!("127.0.0.1:{}", rand::thread_rng().gen_range(1024..65535))),
        )
        .expect("could not create server")
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
