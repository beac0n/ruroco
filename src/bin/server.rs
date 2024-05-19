use std::{fs, str};
use std::error::Error;
use std::io::prelude::*;
use std::net::UdpSocket;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::Parser;
use log::{error, info};
use openssl::pkey::Public;
use openssl::rsa::{Padding, Rsa};

use ruroco::lib::{get_path, init_logger, SOCKET_FILE_PATH, time};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
    address: String,
    #[arg(short, long, default_value = get_path("ruroco_public.pem").into_os_string())]
    pem_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let address = &args.address;

    info!("Starting server on udp://{address}, loading PEM from {} ...", args.pem_path.display());
    let pem_data = fs::read(args.pem_path)?;
    let rsa: Rsa<Public> = Rsa::public_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind(address)?;

    loop {
        iteration(&rsa, address, &socket);
    }
}

fn iteration(rsa: &Rsa<Public>, address: &str, socket: &UdpSocket) {
    let expected_read_count = 1024;
    // make sure encrypted_data size == expected_read_count
    let mut encrypted_data = [0; 1024];
    return match socket.recv_from(&mut encrypted_data) {
        Ok((read_count, src)) if read_count < expected_read_count => {
            error!("Invalid read count {read_count}, expected {expected_read_count} - from {src}")
        }
        Ok(_) => validate_encrypted(rsa, &encrypted_data),
        Err(e) => error!("Could not recv_from socket from udp://{address}: {e}"),
    };
}

fn validate_encrypted(rsa: &Rsa<Public>, encrypted_data: &[u8; 1024]) {
    let mut decrypted_data = vec![0; rsa.size() as usize];
    return match rsa.public_decrypt(encrypted_data, &mut decrypted_data, Padding::PKCS1) {
        Ok(count) => validate_decrypted(&mut decrypted_data, count),
        Err(e) => error!("Could not public_decrypt {encrypted_data:X?}: {e}"),
    };
}

fn validate_decrypted(decrypted_data: &mut Vec<u8>, count: usize) {
    decrypted_data.truncate(count);
    let timestamp = vec_u8_to_u64(&decrypted_data);
    return match time() {
        Ok(now) if timestamp > now => error!("Invalid content {timestamp} is newer than now {now}"),
        Ok(now) if timestamp < now - 5 => {
            error!("Invalid content {timestamp} is older than now {now} - 5 = {}", now - 5)
        }
        Ok(_) => {
            info!("Successfully validated data - {timestamp} is not too old/new");
            match write_to_socket() {
                Ok(_) => {
                    info!("Successfully sent data to commander");
                }
                Err(e) => {
                    error!("Could not send data to commander: {e}")
                }
            }
        }
        Err(e) => error!("Could not get current time: {e}"),
    };
}

fn write_to_socket() -> Result<(), Box<dyn Error>> {
    let mut stream = UnixStream::connect(SOCKET_FILE_PATH)?;
    let command_name = "default"; // TODO: allow for different command names -> has to come from client
    stream.write_all(command_name.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn vec_u8_to_u64(data: &Vec<u8>) -> u64 {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&data);
    u64::from_le_bytes(buffer)
}
