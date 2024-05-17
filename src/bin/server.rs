use std::error::Error;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::{fs, str};

use clap::Parser;
use log::{error, info};
use openssl::pkey::Public;
use openssl::rsa::{Padding, Rsa};

use ruroco::lib;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
    address: String,
    #[arg(short, long, default_value = lib::get_default_pem_public().into_os_string())]
    pem_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    lib::init_logger();
    let args = Cli::parse();

    info!("Starting server on udp://{address}, loading PEM from {} ...", args.pem_path.display());
    let pem_data = fs::read(args.pem_path)?;
    let rsa: Rsa<Public> = Rsa::public_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind(&args.address)?;

    loop {
        iteration(&rsa, &args.address, &socket);
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
        Err(_) => error!("Could not recv_from socket from udp://{address}"),
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
    return match lib::time() {
        Ok(now) if timestamp > now => error!("Invalid content {timestamp} is newer than now {now}"),
        Ok(now) if timestamp < now - 5 => {
            error!("Invalid content {timestamp} is older than now {now} - 5 = {}", now - 5)
        }
        Ok(_) => {
            // TODO: execute command executor
            info!("Successfully validated data - {timestamp} is not too old/new")
        }
        Err(e) => error!("Could not get current time: {e}"),
    };
}

fn vec_u8_to_u64(data: &Vec<u8>) -> u64 {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&data);
    u64::from_le_bytes(buffer)
}
