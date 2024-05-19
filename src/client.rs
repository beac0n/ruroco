use std::error::Error;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use log::info;
use openssl::rsa::{Padding, Rsa};
use crate::common::time;

pub fn send(pem_path: PathBuf, address: String, command: String) -> Result<(), Box<dyn Error>> {
    info!("Running client, connecting to udp://{address}, loading PEM from {pem_path:?} ...");
    let pem_data = fs::read(pem_path)?;
    let rsa = Rsa::private_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    let now = time()?;
    let mut now_bytes_and_command_bytes = now.to_le_bytes().to_vec();
    now_bytes_and_command_bytes.extend(command.as_bytes().to_vec());

    let mut encrypted_data = vec![0; rsa.size() as usize];
    rsa.private_encrypt(&now_bytes_and_command_bytes, &mut encrypted_data, Padding::PKCS1)?;
    socket.connect(&address)?;
    socket.send(&encrypted_data)?;
    info!("Sent command {command} and timestamp {now} to udp://{address}");
    Ok(())
}

pub fn gen(private: PathBuf, public: PathBuf, key_size: u32) -> Result<(), Box<dyn Error>> {
    let public_string = public.to_string_lossy().into_owned();
    let private_string = private.to_string_lossy().into_owned();

    if !private_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate private PEM file: {private_string} does not end with .pem"
        )
        .into());
    }

    if !public_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate private PEM file: {public_string} does not end with .pem"
        )
        .into());
    }

    if private.exists() {
        return Err(format!(
            "Could not generate private PEM file: {private_string} already exists"
        )
        .into());
    }

    if public.exists() {
        return Err(
            format!("Could not generate public PEM file: {public_string} already exists").into()
        );
    }

    info!("Generating new rsa key with {key_size} bits and saving it to {private:?} and {public:?}. This might take a while...");
    let rsa = Rsa::generate(key_size)?;
    fs::write(&private, rsa.private_key_to_pem()?)?;
    fs::write(&public, rsa.public_key_to_pem()?)?;
    Ok(())
}