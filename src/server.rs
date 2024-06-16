use std::{env, fs, str};
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use log::{error, info};
use openssl::error::ErrorStack;
use openssl::pkey::Public;
use openssl::rsa::Rsa;
use openssl::version::version;

use crate::common::{RSA_PADDING, time};

pub struct Server {
    rsa: Rsa<Public>,
    socket: UdpSocket,
    address: String,
    max_delay: u128,
    encrypted_data: Vec<u8>,
    decrypted_data: Vec<u8>,
    socket_path: PathBuf,
}

struct DecodedData {
    timestamp_ns: u128,
    now_ns: u128,
    command_name: String,
}

impl Server {
    pub fn create(
        pem_path: PathBuf,
        address: String,
        max_delay_sec: u16,
        socket_file_path: PathBuf,
    ) -> Result<Server, String> {
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
            max_delay: u128::from(max_delay_sec) * 1_000_000_000,
            socket,
            decrypted_data,
            encrypted_data,
            socket_path: socket_file_path,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!("Running server on udp://{}", self.address);
        let rsa_size = self.rsa.size() as usize;
        loop {
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
            Ok(data) if data.timestamp_ns > data.now_ns => {
                error!("Invalid content {} is newer than now {}", data.timestamp_ns, data.now_ns)
            }
            Ok(data) if data.timestamp_ns < data.now_ns - self.max_delay => {
                error!(
                    "Invalid content {} is older than now {} - {} = {}",
                    data.timestamp_ns,
                    data.now_ns,
                    self.max_delay,
                    data.now_ns - self.max_delay
                )
            }
            Ok(data) => {
                info!("Successfully validated data - {} is not too old/new", data.timestamp_ns);
                self.send_command(&data.command_name)
            }
            Err(e) => error!("Could not decode data: {e}"),
        };
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
            timestamp_ns: u128::from_le_bytes(buffer),
            now_ns: time()?,
            command_name: String::from_utf8_lossy(&self.decrypted_data[16..]).to_string(),
        })
    }
}
