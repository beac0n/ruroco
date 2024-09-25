use openssl::error::ErrorStack;
use openssl::pkey::Public;
use openssl::rsa::Rsa;
use openssl::version::version;
use std::fs::ReadDir;
use std::io::Write;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::{env, fs};

use crate::blocklist::Blocklist;
use crate::common::{error, get_socket_path, info, resolve_path, time, RSA_PADDING};
use crate::config_server::ConfigServer;
use crate::data::{CommanderData, ServerData};

#[derive(Debug)]
pub struct Server {
    rsa: Rsa<Public>,
    socket: UdpSocket,
    address: String,
    ip: String,
    encrypted_data: Vec<u8>,
    decrypted_data: Vec<u8>,
    socket_path: PathBuf,
    blocklist: Blocklist,
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        let self_address_split = self.address.split(':').next().unwrap_or("");
        let other_address_split = other.address.split(':').next().unwrap_or("");
        self_address_split == other_address_split
            && self.encrypted_data == other.encrypted_data
            && self.decrypted_data == other.decrypted_data
            && self.socket_path == other.socket_path
            && self.blocklist == other.blocklist
    }
}

impl Server {
    pub fn create_from_path(path: PathBuf) -> Result<Server, String> {
        match fs::read_to_string(&path) {
            Ok(config) => Server::create(ConfigServer::deserialize(&config)?),
            Err(e) => Err(format!("Could not read {path:?}: {e}")),
        }
    }

    pub fn create(config: ConfigServer) -> Result<Server, String> {
        let address = config.address;
        let config_dir = resolve_path(&config.config_dir);

        let ip_addr = config.ip.parse::<IpAddr>().map_err(|e| {
            format!("Could not parse configured host IP address {}: {e}", config.ip)
        })?;

        if !ip_addr.is_ipv4() {
            return Err(format!(
                "Only IPv4 Addresses are currently supported, got IP config {}",
                ip_addr
            ));
        }

        let pem_path = Self::get_pem_path(&config_dir)?;
        info(format!(
            "Creating server, loading public PEM from {pem_path:?}, using {} ...",
            version()
        ));

        let pem_data =
            fs::read(&pem_path).map_err(|e| format!("Could not read {pem_path:?}: {e}"))?;
        let rsa = Rsa::public_key_from_pem(&pem_data)
            .map_err(|e| format!("Could not load public key from {pem_path:?}: {e}"))?;

        let socket = Self::create_udp_socket(&address)?;

        let rsa_size = rsa.size() as usize;
        let decrypted_data = vec![0; rsa_size];
        let encrypted_data = vec![0; rsa_size];

        Ok(Server {
            rsa,
            address,
            ip: ip_addr.to_string(),
            socket,
            decrypted_data,
            encrypted_data,
            socket_path: get_socket_path(&config_dir),
            blocklist: Blocklist::create(&config_dir),
        })
    }

    fn create_udp_socket(address: &str) -> Result<UdpSocket, String> {
        let pid = std::process::id().to_string();
        match env::var("LISTEN_PID") {
            Ok(listen_pid) if listen_pid == pid => {
                info(String::from(
                    "env var LISTEN_PID was set to our PID, creating socket from raw fd ...",
                ));
                let fd: RawFd = 3;
                Ok(unsafe { UdpSocket::from_raw_fd(fd) })
            }
            Ok(_) => {
                info(format!(
                    "env var LISTEN_PID was set, but not to our PID, binding to {address}"
                ));
                UdpSocket::bind(address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
            Err(_) => {
                info(format!("env var LISTEN_PID was not set, binding to {address}"));
                UdpSocket::bind(address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
        }
    }

    fn get_pem_path(config_dir: &PathBuf) -> Result<PathBuf, String> {
        let pem_files = Self::get_pem_files(config_dir);

        match pem_files.len() {
            0 => Err(format!("Could not find any .pem files in {config_dir:?}")),
            1 => Ok(pem_files.first().unwrap().clone()),
            other => Err(format!("Only one public PEM is supported, found {other}")),
        }
    }

    fn get_pem_files(config_dir: &PathBuf) -> Vec<PathBuf> {
        let entries: ReadDir = match fs::read_dir(config_dir) {
            Ok(entries) => entries,
            Err(e) => {
                error(format!("Error reading directory: {e}"));
                return vec![];
            }
        };

        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().is_some() && path.extension().unwrap() == "pem"
            })
            .collect()
    }

    pub fn run(&mut self) -> Result<(), String> {
        info(format!("Running server on udp://{}", self.address));
        loop {
            let data = self.receive();
            self.run_loop_iteration(data);
        }
    }

    fn run_loop_iteration(&mut self, data: std::io::Result<(usize, SocketAddr)>) -> Option<String> {
        let rsa_size = self.rsa.size() as usize;
        let error_msg = match data {
            Ok((count, src)) if count != rsa_size => {
                Some(format!("Invalid read count {count}, expected {rsa_size} from {src}"))
            }
            Ok((count, src)) => {
                info(format!("Successfully received {count} bytes from {src}"));
                match self.decrypt() {
                    Ok(count) => {
                        self.validate(count, src.ip().to_string());
                        None
                    }
                    Err(e) => Some(format!("Could not decrypt {:X?}: {e}", self.encrypted_data)),
                }
            }
            Err(e) => Some(format!("Could not recv_from socket from udp://{}: {e}", self.address)),
        };

        self.encrypted_data = vec![0; rsa_size];
        self.decrypted_data = vec![0; rsa_size];

        match error_msg {
            Some(s) => {
                error(s.clone());
                Some(s)
            }
            None => None,
        }
    }

    fn receive(&mut self) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(&mut self.encrypted_data)
    }

    fn decrypt(&mut self) -> Result<usize, ErrorStack> {
        self.rsa.public_decrypt(&self.encrypted_data, &mut self.decrypted_data, RSA_PADDING)
    }

    fn validate(&mut self, count: usize, ip_src: String) {
        self.decrypted_data.truncate(count);
        match self.decode() {
            Ok((now_ns, data)) if now_ns > data.deadline() => {
                error(format!("Invalid deadline - now {now_ns} is after {}", data.deadline()))
            }
            Ok((_, data)) if self.ip != data.destination_ip() => error(format!(
                "Invalid host IP - expected {}, actual {}",
                self.ip,
                data.destination_ip()
            )),
            Ok((_, data)) if self.blocklist.is_blocked(data.deadline()) => {
                error(format!("Invalid deadline - {} is on blocklist", data.deadline()))
            }
            Ok((_, data))
                if data.is_strict()
                    && data.source_ip().is_some_and(|ip_sent| ip_sent != ip_src) =>
            {
                error(format!(
                    "Invalid source IP - expected {:?}, actual {ip_src}",
                    data.source_ip()
                ))
            }
            Ok((now_ns, data)) => {
                let command_name = String::from(&data.c);
                let ip = data.source_ip().unwrap_or(ip_src);
                info(format!("Valid data - trying {command_name} with {ip}"));

                self.send_command(CommanderData { command_name, ip });
                self.update_block_list(now_ns, data.deadline());
            }
            Err(e) => error(format!("Could not decode data: {e}")),
        }
    }

    fn update_block_list(&mut self, now_ns: u128, deadline_ns: u128) {
        self.blocklist.clean(now_ns);
        self.blocklist.add(deadline_ns);
        self.blocklist.save();
    }

    fn send_command(&self, data: CommanderData) {
        match self.write_to_socket(data) {
            Ok(_) => info(String::from("Successfully sent data to commander")),
            Err(e) => error(format!(
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

    fn decode(&self) -> Result<(u128, ServerData), String> {
        match ServerData::deserialize(&self.decrypted_data) {
            Ok(data) => Ok((time()?, data)),
            Err(e) => Err(e),
        }
    }
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
    use std::{fs, io};

    #[test]
    fn test_loop_iteration_invalid_read_count() {
        let mut server = create_server();
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));

        assert_eq!(
            server.run_loop_iteration(success_data).unwrap(),
            String::from("Invalid read count 0, expected 128 from 127.0.0.1:8080")
        );
    }

    #[test]
    fn test_loop_iteration_decrypt_error() {
        let mut server = create_server();
        let success_data: io::Result<(usize, SocketAddr)> =
            Ok((128, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));
        assert!(server.run_loop_iteration(success_data).unwrap().starts_with("Could not decrypt "));
    }

    #[test]
    fn test_loop_iteration_error() {
        let mut server = create_server();
        let error_data: io::Result<(usize, SocketAddr)> =
            Err(io::Error::new(io::ErrorKind::Other, "An error occurred"));

        assert!(server
            .run_loop_iteration(error_data)
            .unwrap()
            .starts_with("Could not recv_from socket from udp://127.0.0.1:"));
    }

    fn create_server() -> Server {
        let test_folder_path = PathBuf::from("/dev/shm").join(gen_file_name(""));
        let private_pem_dir = test_folder_path.join("private");

        let _ = fs::create_dir_all(&test_folder_path);
        let _ = fs::create_dir_all(&private_pem_dir);

        gen(
            private_pem_dir.join(gen_file_name(".pem")),
            test_folder_path.join(gen_file_name(".pem")),
            1024,
        )
        .expect("could not generate key");

        Server::create(ConfigServer {
            address: format!("127.0.0.1:{}", rand::thread_rng().gen_range(1024..65535)),
            config_dir: test_folder_path.clone(),
            ..Default::default()
        })
        .expect("could not create server")
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
