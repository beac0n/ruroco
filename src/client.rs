//! This module is responsible for sending data to the server and for generating PEM files

use std::fmt::{Debug, Display};
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;

use openssl::pkey::Private;
use openssl::rsa::Rsa;
use openssl::version::version;

use crate::common::{hash_public_key, info, time_from_ntp, PADDING_SIZE, RSA_PADDING};
use crate::config_client::{CliClient, CommandsClient, SendCommand};
use crate::data::ClientData;
use std::net::ToSocketAddrs;

pub fn run_client(client: CliClient) -> Result<(), String> {
    match client.command {
        CommandsClient::Gen(gen_command) => {
            gen(&gen_command.private_pem_path, &gen_command.public_pem_path, gen_command.key_size)
        }
        CommandsClient::Send(send_command) => {
            let ntp = send_command.ntp.clone();
            send(send_command, time_from_ntp(&ntp)?)
        }
    }
}

/// Send data to the server to execute a predefined command
///
/// * `send_command` - data holding information how to send the command - see SendCommand
/// * `now` - current timestamp in ns
pub fn send(send_command: SendCommand, now: u128) -> Result<(), String> {
    let address = send_command.address;
    let pem_path = send_command.private_pem_path;
    let command = send_command.command;

    info(&format!(
        "Connecting to udp://{address}, loading PEM from {pem_path:?}, using {} ...",
        version()
    ));

    let destination_ips: Vec<SocketAddr> = address
        .to_socket_addrs()
        .map_err(|err| format!("Could not resolve hostname for {address}: {err}"))?
        .collect();

    let destination_ipv4s: Vec<&SocketAddr> =
        destination_ips.iter().filter(|a| a.is_ipv4()).collect();

    let destination_ipv6s: Vec<&SocketAddr> =
        destination_ips.iter().filter(|a| a.is_ipv6()).collect();

    let (destination_ip, bind_address) =
        match (destination_ipv4s.first(), destination_ipv6s.first()) {
            (_, Some(ipv6)) if send_command.ipv6 && !send_command.ipv4 => {
                (ipv6.ip().to_string(), "[::]:0")
            }
            (Some(ipv4), _) if !send_command.ipv6 && send_command.ipv4 => {
                (ipv4.ip().to_string(), "0.0.0.0:0")
            }
            (Some(ipv4), _) => (ipv4.ip().to_string(), "0.0.0.0:0"),
            (_, Some(ipv6)) => (ipv6.ip().to_string(), "[::]:0"),
            _ => return Err(format!("Could not find any IPv4 or IPv6 address for {address}")),
        };

    info(&format!("Found IPs {destination_ipv4s:?} and {destination_ipv6s:?} for {address}, connecting to {destination_ip}"));

    let rsa = get_rsa_private(&pem_path)?;
    let data_to_encrypt = get_data_to_encrypt(
        &command,
        &rsa,
        send_command.deadline,
        !send_command.permissive,
        send_command.ip,
        destination_ip,
        now,
    )?;
    let data_to_send = get_data_to_send(&data_to_encrypt, &rsa)?;

    // create UDP socket and send the encrypted data to the specified address
    let socket = UdpSocket::bind(bind_address).map_err(|e| socket_err(e, &address))?;
    socket.connect(&address).map_err(|e| socket_err(e, &address))?;
    socket.send(&data_to_send).map_err(|e| socket_err(e, &address))?;

    info(&format!("Sent command {command} from {bind_address} to udp://{address}"));
    Ok(())
}

fn get_data_to_send(data_to_encrypt: &Vec<u8>, rsa: &Rsa<Private>) -> Result<Vec<u8>, String> {
    let pem_pub_key = rsa
        .public_key_to_pem()
        .map_err(|e| format!("Could not create public pem from private key: {e}"))?;
    let mut data_to_send = hash_public_key(pem_pub_key)?;
    let encrypted_data = encrypt_data(data_to_encrypt, rsa)?;
    data_to_send.extend(encrypted_data);

    Ok(data_to_send)
}

/// Generate a public and private PEM file with the provided key_size
///
/// * `private_path` - Path to the private PEM file which needs to be created
/// * `public_path` - Path to the public PEM file which needs to be created
/// * `key_size` - key size
pub fn gen(private_path: &Path, public_path: &Path, key_size: u32) -> Result<(), String> {
    validate_pem_path(public_path)?;
    validate_pem_path(private_path)?;

    info(&format!("Generating new rsa key with {key_size} bits and saving it to {private_path:?} and {public_path:?}. This might take a while..."));
    let rsa = Rsa::generate(key_size)
        .map_err(|e| format!("Could not generate rsa for key size {key_size}: {e}"))?;

    let private_key_pem = get_pem_data(&rsa, "private")?;
    let public_key_pem = get_pem_data(&rsa, "public")?;

    write_pem_data(private_path, private_key_pem, "private")?;
    write_pem_data(public_path, public_key_pem, "public")?;

    info(&format!("Generated new rsa key with {key_size} bits and saved it to {private_path:?} and {public_path:?}"));

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

fn get_rsa_private(pem_path: &Path) -> Result<Rsa<Private>, String> {
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
        ClientData::create(command, deadline, strict, source_ip, destination_ip, now_ns)
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

fn write_pem_data(path: &Path, data: Vec<u8>, name: &str) -> Result<(), String> {
    match path.parent() {
        Some(p) => {
            fs::create_dir_all(p).map_err(|e| format!("Could not create directory ({e}) {p:?}"))?
        }
        None => Err(format!("Could not get parent directory of {path:?}"))?,
    }

    fs::write(path, data).map_err(|e| format!("Could not write {name} key to {path:?}: {e}"))?;
    Ok(())
}

fn validate_pem_path(path: &Path) -> Result<(), String> {
    match path.to_str() {
        Some(s) if s.ends_with(".pem") && !path.exists() => Ok(()),
        Some(s) if path.exists() => Err(format!("Could not create PEM file: {s} already exists")),
        Some(s) => Err(format!("Could not read PEM file: {s} does not end with .pem")),
        None => Err(format!("Could not convert PEM path {path:?} to string")),
    }
}

#[cfg(test)]
mod tests {
    use crate::config_client::CliClient;
    use crate::data::ClientData;
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    #[test]
    fn test_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            5,
            false,
            Some("192.168.178.123".to_string()),
            "192.168.178.124".to_string(),
            1725821510 * 1_000_000_000,
        )
        .serialize()
        .unwrap();
        let server_data_str = String::from_utf8_lossy(&server_data).to_string();

        assert_eq!(server_data_str, "c=\"some_kind_of_long_but_not_really_that_long_command\"\nd=\"1725821515000000000\"\ns=0\ni=\"192.168.178.123\"\nh=\"192.168.178.124\"");
        assert_eq!(
            ClientData::deserialize(&server_data).unwrap(),
            ClientData {
                c: "some_kind_of_long_but_not_really_that_long_command".to_string(),
                d: 1725821515000000000,
                s: 0,
                i: Some("192.168.178.123".to_string()),
                h: "192.168.178.124".to_string()
            }
        );
    }
}
