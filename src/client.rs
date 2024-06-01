use std::error::Error;
use std::fmt::{Debug, Display};
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;

use log::info;
use openssl::rsa::Rsa;
use openssl::version::version;

use crate::common::{RSA_PADDING, time};

fn pem_load_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not load {val:?}: {err}")
}

fn socket_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not connect/send data to {val:?}: {err}")
}

pub fn send(pem_path: PathBuf, address: String, command: String) -> Result<(), Box<dyn Error>> {
    info!("Running client, connecting to udp://{address}, loading PEM from {pem_path:?}, using {} ...", version());

    // collect data to encrypt: now-timestamp + command -> all as bytes
    let now = time()?;
    let mut data_to_encrypt = now.to_le_bytes().to_vec();
    data_to_encrypt.extend(command.as_bytes().to_vec());
    // TODO: extend data_to_encrypt - add delimiter __ruroco__

    // encrypt data we want to send - load RSA private key from PEM file for that
    let pem_data = fs::read(&pem_path).map_err(|e| pem_load_err(e, &pem_path))?;
    let rsa = Rsa::private_key_from_pem(&pem_data).map_err(|e| pem_load_err(e, &pem_path))?;
    let mut encrypted_data = vec![0; rsa.size() as usize];
    rsa.private_encrypt(&data_to_encrypt, &mut encrypted_data, RSA_PADDING)
        .map_err(|e| format!("Could not encrypt {data_to_encrypt:?}: {e}"))?;

    // create UDP socket and send the encrypted data to the specified address
    let socket = UdpSocket::bind("127.0.0.1:0").map_err(|e| socket_err(e, &address))?;
    socket.connect(&address).map_err(|e| socket_err(e, &address))?;
    socket.send(&encrypted_data).map_err(|e| socket_err(e, &address))?;

    info!("Sent command {command} and timestamp {now} to udp://{address}");
    Ok(())
}

pub fn gen(private: PathBuf, public: PathBuf, key_size: u32) -> Result<(), Box<dyn Error>> {
    let public_string = public.to_str().expect("Could not convert provided public PEM path");
    let private_string = private.to_str().expect("Could not convert provided private PEM path");

    if !private_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate private PEM file: {private_string} does not end with .pem"
        )
        .into());
    }

    if !public_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate public PEM file: {public_string} does not end with .pem"
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
    fs::write(private, rsa.private_key_to_pem()?)?;
    fs::write(public, rsa.public_key_to_pem()?)?;
    Ok(())
}
