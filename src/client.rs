//! This module is responsible for sending data to the server and for generating PEM files

use std::fmt::{Debug, Display};
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;

use openssl::pkey::Private;
use openssl::rsa::Rsa;
use openssl::version::version;

use std::net::ToSocketAddrs;

use crate::common::{info, PADDING_SIZE, RSA_PADDING};
use crate::data::ServerData;

/// Send data to the server to execute a predefined command
///
/// * `pem_path` - Path to the private PEM to encrypt the data with
/// * `address` - IP address and port to send the data to
/// * `command` - Which command the commander should execute
/// * `deadline` - After how many seconds from now the commander has to start executing the command
/// * `now` - current timestamp in ns
pub fn send(
    pem_path: PathBuf,
    address: String,
    command: String,
    deadline: u16,
    strict: bool,
    source_ip: Option<String>,
    now: u128,
) -> Result<(), String> {
    info(format!(
        "Connecting to udp://{address}, loading PEM from {pem_path:?}, using {} ...",
        version()
    ));

    let destination_ips = address
        .to_socket_addrs()
        .map_err(|err| format!("Could not resolve hostname for {address}: {err}"))?
        .filter(|a| a.is_ipv4())
        .collect::<Vec<SocketAddr>>();

    let destination_ip = match destination_ips.first() {
        Some(a) => a.ip().to_string(),
        None => return Err(format!("Could not find any IPv4 address for {address}")),
    };

    let rsa = get_rsa_private(&pem_path)?;
    let data_to_encrypt =
        get_data_to_encrypt(&command, &rsa, deadline, strict, source_ip, destination_ip, now)?;
    let encrypted_data = encrypt_data(&data_to_encrypt, &rsa)?;

    // create UDP socket and send the encrypted data to the specified address
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| socket_err(e, &address))?;
    socket.connect(&address).map_err(|e| socket_err(e, &address))?;
    socket.send(&encrypted_data).map_err(|e| socket_err(e, &address))?;

    info(format!("Sent command {command} to udp://{address}"));
    Ok(())
}

/// Generate a public and private PEM file with the provided key_size
///
/// * `private_path` - Path to the private PEM file which needs to be created
/// * `public_path` - Path to the public PEM file which needs to be created
/// * `key_size` - key size
pub fn gen(private_path: PathBuf, public_path: PathBuf, key_size: u32) -> Result<(), String> {
    validate_pem_path(&public_path)?;
    validate_pem_path(&private_path)?;

    info(format!("Generating new rsa key with {key_size} bits and saving it to {private_path:?} and {public_path:?}. This might take a while..."));
    let rsa = Rsa::generate(key_size)
        .map_err(|e| format!("Could not generate rsa for key size {key_size}: {e}"))?;

    let private_key_pem = get_pem_data(&rsa, "private")?;
    let public_key_pem = get_pem_data(&rsa, "public")?;

    write_pem_data(&private_path, private_key_pem, "private")?;
    write_pem_data(&public_path, public_key_pem, "public")?;

    Ok(())
}

fn pem_load_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not load {val:?}: {err}")
}

fn socket_err<I: Display, E: Debug>(err: I, val: E) -> String {
    format!("Could not connect/send data to {val:?}: {err}")
}

fn encrypt_data(data_to_encrypt: &Vec<u8>, rsa: &Rsa<Private>) -> Result<Vec<u8>, String> {
    let mut encrypted_data = vec![0; rsa.size() as usize];
    rsa.private_encrypt(data_to_encrypt, &mut encrypted_data, RSA_PADDING).map_err(|e| {
        format!("Could not encrypt ({} bytes) {data_to_encrypt:?}: {e}", data_to_encrypt.len())
    })?;
    Ok(encrypted_data)
}

fn get_rsa_private(pem_path: &PathBuf) -> Result<Rsa<Private>, String> {
    // encrypt data we want to send - load RSA private key from PEM file for that
    let pem_data = fs::read(pem_path).map_err(|e| pem_load_err(e, pem_path))?;
    Rsa::private_key_from_pem(&pem_data).map_err(|e| pem_load_err(e, pem_path))
}

fn get_data_to_encrypt(
    command: &str,
    rsa: &Rsa<Private>,
    deadline: u16,
    strict: bool,
    source_ip: Option<String>,
    destination_ip: String,
    now_ns: u128,
) -> Result<Vec<u8>, String> {
    let data_to_encrypt =
        ServerData::create(command, deadline, strict, source_ip, destination_ip, now_ns)
            .serialize()?;
    let data_to_encrypt_len = data_to_encrypt.len();
    let rsa_size = rsa.size() as usize;
    if data_to_encrypt_len + PADDING_SIZE > rsa_size {
        let max_size = rsa_size - PADDING_SIZE;
        return Err(format!(
            "Too much data, must be at most {max_size} bytes, but was {data_to_encrypt_len} bytes. \
            Reduce command name length or create a bigger RSA key size."
        ));
    }

    Ok(data_to_encrypt)
}

fn get_pem_data(rsa: &Rsa<Private>, name: &str) -> Result<Vec<u8>, String> {
    let data = match name {
        "public" => rsa.public_key_to_pem(),
        "private" => rsa.private_key_to_pem(),
        _ => return Err(format!("Invalid pem data name {name}")),
    };

    data.map_err(|e| format!("Could not create {name} key pem: {e}"))
}

fn write_pem_data(path: &PathBuf, data: Vec<u8>, name: &str) -> Result<(), String> {
    fs::write(path, data).map_err(|e| format!("Could not write {name} key to {path:?}: {e}"))?;
    Ok(())
}

fn validate_pem_path(path: &PathBuf) -> Result<(), String> {
    match path.to_str() {
        Some(s) if s.ends_with(".pem") && !path.exists() => Ok(()),
        Some(s) if path.exists() => Err(format!("Could not create PEM file: {s} already exists")),
        Some(s) => Err(format!("Could not read PEM file: {s} does not end with .pem")),
        None => Err(format!("Could not convert PEM path {path:?} to string")),
    }
}

#[cfg(test)]
mod tests {
    use crate::data::ServerData;

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ServerData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            5,
            false,
            Some(String::from("192.168.178.123")),
            1725821510 * 1_000_000_000,
        )
        .serialize()
        .unwrap();
        let server_data_str = String::from_utf8_lossy(&server_data).to_string();

        assert_eq!(server_data_str, "c=\"some_kind_of_long_but_not_really_that_long_command\"\nd=\"1725821515000000000\"\ns=0\ni=\"192.168.178.123\"");
        assert_eq!(
            ServerData::deserialize(&server_data).unwrap(),
            ServerData {
                c: String::from("some_kind_of_long_but_not_really_that_long_command"),
                d: 1725821515000000000,
                s: 0,
                i: Some(String::from("192.168.178.123")),
            }
        );
    }
}
