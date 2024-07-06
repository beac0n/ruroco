use std::fmt::{Debug, Display};
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;

use log::info;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use openssl::version::version;

use crate::common::{PADDING_SIZE, RSA_PADDING, time};

fn pem_load_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not load {val:?}: {err}")
}

fn socket_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not connect/send data to {val:?}: {err}")
}

pub fn send(
    pem_path: PathBuf,
    address: String,
    command: String,
    deadline: u16,
) -> Result<(), String> {
    info!("Connecting to udp://{address}, loading PEM from {pem_path:?}, using {} ...", version());

    let rsa = get_rsa_private(&pem_path)?;
    let data_to_encrypt = get_data_to_encrypt(&command, &rsa, deadline)?;
    let encrypted_data = encrypt_data(&data_to_encrypt, &rsa)?;

    // create UDP socket and send the encrypted data to the specified address
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| socket_err(e, &address))?;
    socket.connect(&address).map_err(|e| socket_err(e, &address))?;
    socket.send(&encrypted_data).map_err(|e| socket_err(e, &address))?;

    info!("Sent command {command} to udp://{address}");
    Ok(())
}

fn encrypt_data(data_to_encrypt: &Vec<u8>, rsa: &Rsa<Private>) -> Result<Vec<u8>, String> {
    let mut encrypted_data = vec![0; rsa.size() as usize];
    rsa.private_encrypt(&data_to_encrypt, &mut encrypted_data, RSA_PADDING).map_err(|e| {
        format!("Could not encrypt ({} bytes) {data_to_encrypt:?}: {e}", data_to_encrypt.len())
    })?;
    Ok(encrypted_data)
}

fn get_rsa_private(pem_path: &PathBuf) -> Result<Rsa<Private>, String> {
    // encrypt data we want to send - load RSA private key from PEM file for that
    let pem_data = fs::read(&pem_path).map_err(|e| pem_load_err(e, &pem_path))?;
    Ok(Rsa::private_key_from_pem(&pem_data).map_err(|e| pem_load_err(e, &pem_path))?)
}

fn get_data_to_encrypt(
    command: &str,
    rsa: &Rsa<Private>,
    deadline: u16,
) -> Result<Vec<u8>, String> {
    // collect data to encrypt: now-timestamp + command + random data -> all as bytes
    let mut data_to_encrypt = Vec::new();

    let timestamp = time()? + (u128::from(deadline) * 1_000_000_000);
    let timestamp_bytes = timestamp.to_le_bytes().to_vec();
    let timestamp_len = timestamp_bytes.len();
    data_to_encrypt.extend(timestamp_bytes);

    data_to_encrypt.extend(command.as_bytes().to_vec());

    let rsa_size = rsa.size() as usize;
    if data_to_encrypt.len() + PADDING_SIZE > rsa_size {
        let max_size = rsa_size - PADDING_SIZE - timestamp_len;
        return Err(format!("Command too long, must be at most {max_size} bytes").into());
    }

    Ok(data_to_encrypt)
}

pub fn gen(private_path: PathBuf, public_path: PathBuf, key_size: u32) -> Result<(), String> {
    validate_pem_path(&public_path)?;
    validate_pem_path(&private_path)?;

    info!("Generating new rsa key with {key_size} bits and saving it to {private_path:?} and {public_path:?}. This might take a while...");
    let rsa = Rsa::generate(key_size)
        .map_err(|e| format!("Could not generate rsa for key size {key_size}: {e}"))?;

    let private_key_pem = get_pem_data(&rsa, "private")?;
    let public_key_pem = get_pem_data(&rsa, "public")?;

    write_pem_data(&private_path, private_key_pem, "private")?;
    write_pem_data(&public_path, public_key_pem, "public")?;

    Ok(())
}

fn get_pem_data(rsa: &Rsa<Private>, name: &str) -> Result<Vec<u8>, String> {
    let data = match name {
        "public" => rsa.public_key_to_pem(),
        "private" => rsa.private_key_to_pem(),
        _ => return Err(format!("Invalid pem data name {name}").into()),
    };

    Ok(data.map_err(|e| format!("Could not create {name} key pem: {e}"))?)
}

fn write_pem_data(path: &PathBuf, data: Vec<u8>, name: &str) -> Result<(), String> {
    fs::write(&path, data).map_err(|e| format!("Could not write {name} key to {path:?}: {e}"))?;
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
