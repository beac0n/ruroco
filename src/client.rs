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

pub fn send(pem_path: PathBuf, address: String, command: String) -> Result<(), String> {
    info!("Connecting to udp://{address}, loading PEM from {pem_path:?}, using {} ...", version());

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

    info!("Sent command {command} to udp://{address}");
    Ok(())
}

pub fn gen(private_path: PathBuf, public_path: PathBuf, key_size: u32) -> Result<(), String> {
    validate_pem_path(&public_path)?;
    validate_pem_path(&private_path)?;

    info!("Generating new rsa key with {key_size} bits and saving it to {private_path:?} and {public_path:?}. This might take a while...");
    let rsa = Rsa::generate(key_size)
        .map_err(|e| format!("Could not generate rsa for key size {key_size}: {e}"))?;

    let private_key_pem =
        rsa.private_key_to_pem().map_err(|e| format!("Could not create private key pem: {e}"))?;

    let public_key_pem =
        rsa.public_key_to_pem().map_err(|e| format!("Could not create public key pem: {e}"))?;

    fs::write(&private_path, private_key_pem)
        .map_err(|e| format!("Could not write private key to {private_path:?}: {e}"))?;

    fs::write(&public_path, public_key_pem)
        .map_err(|e| format!("Could not write public key to {public_path:?}: {e}"))?;
    Ok(())
}

fn validate_pem_path(path: &PathBuf) -> Result<(), String> {
    match path.to_str() {
        Some(s) if s.ends_with(".pem") && !path.exists() => Ok(()),
        Some(s) if path.exists() => {
            Err(format!("Could not create PEM file: {s} already exists").into())
        }
        Some(s) => Err(format!("Could not read PEM file: {s} does not end with .pem").into()),
        None => Err(format!("Could not convert PEM path {path:?} to string").into()),
    }
}
