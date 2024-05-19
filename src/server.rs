use std::{fs, str};
use std::error::Error;
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use log::{error, info};
use openssl::error::ErrorStack;
use openssl::pkey::Public;
use openssl::rsa::{Padding, Rsa};

use crate::common::{SOCKET_FILE_PATH, time};

pub struct Server {
    rsa: Rsa<Public>,
    socket: UdpSocket,
    address: String,
    max_delay: u128,
    encrypted_data: [u8; 1024],
    decrypted_data: Vec<u8>,
}

struct DecodedData {
    timestamp_ns: u128,
    now_ns: u128,
    command_name: String,
}

impl Server {
    pub fn create(pem_path: PathBuf, address: String, max_delay: u128) -> Server {
        info!("Creating server, loading public PEM from {pem_path:?} ...");

        let pem_data = fs::read(pem_path).unwrap();
        let rsa = Rsa::public_key_from_pem(&pem_data).unwrap();
        let socket = UdpSocket::bind(&address).unwrap();
        let decrypted_data = vec![0; rsa.size() as usize];

        Server {
            rsa,
            address,
            max_delay,
            socket,
            decrypted_data,
            encrypted_data: [0; 1024],
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Running server on udp://{}", self.address);
        let expected_count = self.encrypted_data.len();
        loop {
            match self.receive() {
                Ok((count, src)) if count != expected_count => {
                    error!("Invalid read count {count}, expected {expected_count} from {src}")
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
        }
    }

    fn receive(&mut self) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(&mut self.encrypted_data)
    }

    fn decrypt(&mut self) -> Result<usize, ErrorStack> {
        self.rsa
            .public_decrypt(&self.encrypted_data, &mut self.decrypted_data, Padding::PKCS1)
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
            Err(e) => error!("Could not send data to commander: {e}"),
        }
    }

    fn write_to_socket(&self, command_name: &str) -> Result<(), Box<dyn Error>> {
        let mut stream = UnixStream::connect(SOCKET_FILE_PATH)?;
        stream.write_all(command_name.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    fn decode(&self) -> Result<DecodedData, Box<dyn Error>> {
        let mut buffer = [0u8; 16];
        buffer.copy_from_slice(&self.decrypted_data[..16]);

        Ok(DecodedData {
            timestamp_ns: u128::from_le_bytes(buffer),
            now_ns: time()?,
            command_name: str::from_utf8(&self.decrypted_data[16..])?.to_string(),
        })
    }
}
