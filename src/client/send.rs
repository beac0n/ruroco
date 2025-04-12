use crate::common::data::ClientData;
use crate::common::{hash_public_key, info, PADDING_SIZE, RSA_PADDING};
use crate::config::config_client::SendCommand;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use openssl::version::version;
use std::fmt::{Debug, Display};
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};

    use crate::client::gen::gen;
    use crate::client::send::send;
    use crate::common::time;
    use crate::config::config_client::{CliClient, SendCommand};
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;

    const IP: &str = "192.168.178.123";

    #[test]
    fn test_send_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "send", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_send_no_such_file() {
        let pem_file_name = gen_file_name(".pem");

        let result = send(
            SendCommand {
                private_pem_path: PathBuf::from(&pem_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not load \"{pem_file_name}\": No such file or directory (os error 2)")
        );
    }

    #[test]
    fn test_send_invalid_pem() {
        let pem_file_name = gen_file_name(".pem");
        File::create(&pem_file_name).unwrap();

        let result = send(
            SendCommand {
                private_pem_path: PathBuf::from(&pem_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&pem_file_name);

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No supported data to decode. Input type: PEM"));
    }

    #[test]
    fn test_send_invalid_port_value() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let address = "127.0.0.1:asd".to_string();
        let result = send(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}: invalid port value")
        );
    }

    #[test]
    fn test_send_unknown_service() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let address = "999.999.999.999:9999".to_string();
        let result = send(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "Could not resolve hostname for {address}: \
                failed to lookup address information: Name or service not known"
            )
        );
    }

    #[test]
    fn test_send_command_too_long() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let result = send(
            SendCommand {
                private_pem_path,
                command: "#".repeat(66),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            "Too much data, must be at most 117 bytes, but was 132 bytes. \
                Reduce command name length or create a bigger RSA key size."
                .to_string()
        );
    }

    #[test]
    fn test_send_ipv4() {
        assert!(send_test("127.0.0.1:1234").is_ok());
    }

    #[test]
    fn test_send_ipv6() {
        assert!(send_test("::1:1234").is_ok());
    }

    fn send_test(address: &str) -> Result<(), String> {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024)?;

        let result = send(
            SendCommand {
                address: address.to_string(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time()?,
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        result
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
