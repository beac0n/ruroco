use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::{env, fs, str};

use log::{error, info};
use openssl::error::ErrorStack;
use openssl::pkey::Public;
use openssl::rsa::Rsa;
use openssl::version::version;

use crate::blocklist::Blocklist;
use crate::common::{get_socket_path, time, RSA_PADDING};

pub struct Server {
    rsa: Rsa<Public>,
    socket: UdpSocket,
    address: String,
    encrypted_data: Vec<u8>,
    decrypted_data: Vec<u8>,
    socket_path: PathBuf,
    blocklist: Blocklist,
}

struct DecodedData {
    deadline_ns: u128,
    now_ns: u128,
    command_name: String,
}

impl Server {
    pub fn create(config_dir: PathBuf, address: String) -> Result<Server, String> {
        let pem_path = Self::get_pem_path(&config_dir)?;
        info!("Creating server, loading public PEM from {pem_path:?}, using {} ...", version());

        let pem_data =
            fs::read(&pem_path).map_err(|e| format!("Could not read {pem_path:?}: {e}"))?;
        let rsa = Rsa::public_key_from_pem(&pem_data)
            .map_err(|e| format!("Could not load public key from {pem_path:?}: {e}"))?;

        let pid = std::process::id().to_string();
        let socket = match env::var("LISTEN_PID") {
            Ok(listen_pid) if listen_pid == pid => {
                info!("env var LISTEN_PID was set to our PID, creating socket from raw fd ...");
                let fd: RawFd = 3;
                let socket = unsafe { UdpSocket::from_raw_fd(fd) };
                socket
            }
            Ok(_) => {
                info!("env var LISTEN_PID was set, but not to our PID, binding to {address}");
                UdpSocket::bind(&address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))?
            }
            Err(_) => {
                info!("env var LISTEN_PID was not set, binding to {address}");
                UdpSocket::bind(&address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))?
            }
        };

        let rsa_size = rsa.size() as usize;
        let decrypted_data = vec![0; rsa_size];
        let encrypted_data = vec![0; rsa_size];

        Ok(Server {
            rsa,
            address,
            socket,
            decrypted_data,
            encrypted_data,
            socket_path: get_socket_path(&config_dir),
            blocklist: Blocklist::create(&config_dir),
        })
    }

    fn get_pem_path(config_dir: &PathBuf) -> Result<PathBuf, String> {
        let pem_files = Self::get_pem_files(&config_dir);

        return match pem_files.len() {
            0 => Err(format!("Could not find any .pem files in {config_dir:?}").into()),
            1 => Ok(pem_files.first().unwrap().clone()),
            other => Err(format!(
                "Only one public PEM is supported at this point in time, found {other}"
            )
            .into()),
        };
    }

    fn get_pem_files(config_dir: &PathBuf) -> Vec<PathBuf> {
        let mut pem_paths = vec![];
        match fs::read_dir(config_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            match path.extension() {
                                Some(extension) if path.is_file() && extension == "pem" => {
                                    pem_paths.push(path)
                                }
                                _ => {}
                            }
                        }
                        Err(e) => error!("Error reading entry: {e}"),
                    }
                }
            }
            Err(e) => error!("Error reading directory: {e}"),
        }
        pem_paths
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!("Running server on udp://{}", self.address);
        let rsa_size = self.rsa.size() as usize;
        loop {
            // TODO: How to deal with DoS attacks?
            match self.receive() {
                Ok((count, src)) if count != rsa_size => {
                    error!("Invalid read count {count}, expected {rsa_size} from {src}")
                }
                Ok((count, src)) => {
                    info!("Successfully received {count} bytes from {src}");
                    match self.decrypt() {
                        Ok(count) => self.validate(count),
                        Err(e) => error!("Could not decrypt {:X?}: {e}", self.encrypted_data),
                    }
                }
                Err(e) => error!("Could not recv_from socket from udp://{}: {e}", self.address),
            }

            self.encrypted_data = vec![0; rsa_size];
            self.decrypted_data = vec![0; rsa_size];
        }
    }

    fn receive(&mut self) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(&mut self.encrypted_data)
    }

    fn decrypt(&mut self) -> Result<usize, ErrorStack> {
        self.rsa.public_decrypt(&self.encrypted_data, &mut self.decrypted_data, RSA_PADDING)
    }

    fn validate(&mut self, count: usize) {
        self.decrypted_data.truncate(count);
        return match self.decode() {
            Ok(data) if data.now_ns > data.deadline_ns => {
                error!("Invalid data - now {} is after deadline {}", data.now_ns, data.deadline_ns)
            }
            Ok(data) if self.blocklist.is_blocked(data.deadline_ns) => {
                error!("Invalid data - deadline {} is on blocklist", data.deadline_ns)
            }
            Ok(data) => {
                info!(
                    "Successfully validated data - now {} is before deadline {}",
                    data.now_ns, data.deadline_ns
                );
                self.send_command(&data.command_name);
                self.update_block_list(&data);
            }
            Err(e) => error!("Could not decode data: {e}"),
        };
    }

    fn update_block_list(&mut self, data: &DecodedData) {
        self.blocklist.clean(data.now_ns);
        self.blocklist.add(data.deadline_ns);
        self.blocklist.save();
    }

    fn send_command(&self, command_name: &str) {
        match self.write_to_socket(command_name) {
            Ok(_) => info!("Successfully sent data to commander"),
            Err(e) => {
                error!("Could not send data to commander via socket {:?}: {e}", &self.socket_path)
            }
        }
    }

    fn write_to_socket(&self, command_name: &str) -> Result<(), String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| format!("Could not connect to socket {:?}: {e}", self.socket_path))?;
        stream.write_all(command_name.as_bytes()).map_err(|e| {
            format!("Could not write {command_name} to socket {:?}: {e}", self.socket_path)
        })?;
        stream
            .flush()
            .map_err(|e| format!("Could not flush stream for {:?}: {e}", self.socket_path))?;
        Ok(())
    }

    fn decode(&self) -> Result<DecodedData, String> {
        let mut buffer = [0u8; 16];
        buffer.copy_from_slice(&self.decrypted_data[..16]);

        Ok(DecodedData {
            deadline_ns: u128::from_le_bytes(buffer),
            now_ns: time()?,
            command_name: String::from_utf8_lossy(&self.decrypted_data[16..]).to_string(),
        })
    }
}
