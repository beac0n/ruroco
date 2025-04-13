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

#[derive(Debug)]
pub struct Sender {
    cmd: SendCommand,
    now: u128,
    rsa: Rsa<Private>,
    rsa_size: usize,
}

impl Sender {
    pub fn create(cmd: SendCommand, now: u128) -> Result<Self, String> {
        let rsa = Self::get_rsa_private(&cmd.private_pem_path)?;
        let rsa_size = rsa.size() as usize;
        Ok(Self {
            cmd,
            now,
            rsa,
            rsa_size,
        })
    }

    /// Send data to the server to execute a predefined command
    ///
    /// * `send_command` - data holding information how to send the command - see SendCommand
    /// * `now` - current timestamp in ns
    pub fn send(&self) -> Result<(), String> {
        info(&format!(
            "Connecting to udp://{}, loading PEM from {:?}, using {} ...",
            self.cmd.address,
            self.cmd.private_pem_path,
            version()
        ));

        let destination_ips: Vec<SocketAddr> = self
            .cmd
            .address
            .to_socket_addrs()
            .map_err(|err| format!("Could not resolve hostname for {}: {err}", self.cmd.address))?
            .collect();

        let destination_ipv4s: Vec<&SocketAddr> =
            destination_ips.iter().filter(|a| a.is_ipv4()).collect();

        let destination_ipv6s: Vec<&SocketAddr> =
            destination_ips.iter().filter(|a| a.is_ipv6()).collect();

        let use_ipv6 = self.cmd.ipv6;
        let use_ipv4 = self.cmd.ipv4;
        let command = &self.cmd.command;
        let address = &self.cmd.address;

        let cnfa = "Could not find any";

        let dest_and_bind_addresses: Vec<(String, &str)> =
            match (destination_ipv4s.first(), destination_ipv6s.first()) {
                // ipv4 xor ipv6 where defined
                (_, Some(ipv6)) if use_ipv6 && !use_ipv4 => vec![(ipv6.ip().to_string(), "[::]:0")],
                (Some(ipv4), _) if !use_ipv6 && use_ipv4 => {
                    vec![(ipv4.ip().to_string(), "0.0.0.0:0")]
                }
                (_, None) if use_ipv6 && !use_ipv4 => {
                    return Err(format!("{cnfa} IPv6 for {}", self.cmd.address))
                }
                (None, _) if !use_ipv6 && use_ipv4 => {
                    return Err(format!("{cnfa} IPv4 for {}", self.cmd.address))
                }
                // ipv4 or ipv6 where not defined or where both defined => ignore
                (Some(ipv4), Some(ipv6)) => vec![
                    (ipv4.ip().to_string(), "0.0.0.0:0"),
                    (ipv6.ip().to_string(), "[::]:0"),
                ],
                (Some(ipv4), None) => vec![(ipv4.ip().to_string(), "0.0.0.0:0")],
                (None, Some(ipv6)) => vec![(ipv6.ip().to_string(), "[::]:0")],
                _ => return Err(format!("{cnfa} IPv4 or IPv6 address for {address}")),
            };

        for (destination_ip, bind_address) in dest_and_bind_addresses {
            info(&format!("Found IPs {destination_ipv4s:?} and {destination_ipv6s:?} for {address}, connecting to {destination_ip}"));
            self.send_data(destination_ip, bind_address)?;
            info(&format!("Sent command {command} from {bind_address} to udp://{address}"));
        }

        Ok(())
    }

    fn send_data(&self, destination_ip: String, bind_address: &str) -> Result<(), String> {
        let data_to_encrypt = self.get_data_to_encrypt(destination_ip)?;
        let data_to_send = self.get_data_to_send(&data_to_encrypt)?;

        // create UDP socket and send the encrypted data to the specified address
        let address = &self.cmd.address;
        let socket = UdpSocket::bind(bind_address).map_err(|e| Self::socket_err(e, address))?;
        socket.connect(address).map_err(|e| Self::socket_err(e, address))?;
        socket.send(&data_to_send).map_err(|e| Self::socket_err(e, address))?;
        Ok(())
    }

    fn get_rsa_private(pem_path: &Path) -> Result<Rsa<Private>, String> {
        // encrypt data we want to send - load RSA private key from PEM file for that
        let pem_data = fs::read(pem_path).map_err(|e| Self::pem_load_err(e, pem_path))?;
        Rsa::private_key_from_pem(&pem_data).map_err(|e| Self::pem_load_err(e, pem_path))
    }

    fn get_data_to_send(&self, data_to_encrypt: &Vec<u8>) -> Result<Vec<u8>, String> {
        let pem_pub_key = (&self.rsa)
            .public_key_to_pem()
            .map_err(|e| format!("Could not create public pem from private key: {e}"))?;
        let mut data_to_send = hash_public_key(pem_pub_key)?;
        let encrypted_data = self.encrypt_data(data_to_encrypt)?;
        data_to_send.extend(encrypted_data);

        Ok(data_to_send)
    }

    fn encrypt_data(&self, data_to_encrypt: &Vec<u8>) -> Result<Vec<u8>, String> {
        let mut encrypted_data = vec![0; self.rsa_size];
        (&self.rsa).private_encrypt(data_to_encrypt, &mut encrypted_data, RSA_PADDING).map_err(
            |e| {
                format!(
                    "Could not encrypt ({} bytes) {data_to_encrypt:?}: {e}",
                    data_to_encrypt.len()
                )
            },
        )?;
        Ok(encrypted_data)
    }

    fn pem_load_err<I: Display, E: Debug>(err: I, val: E) -> String {
        format!("Could not load {val:?}: {err}")
    }

    fn socket_err<I: Display, E: Debug>(err: I, val: E) -> String {
        format!("Could not connect/send data to {val:?}: {err}")
    }

    fn get_data_to_encrypt(&self, destination_ip: String) -> Result<Vec<u8>, String> {
        let data_to_encrypt = ClientData::create(
            &self.cmd.command,
            self.cmd.deadline,
            !self.cmd.permissive,
            self.cmd.ip.clone(),
            destination_ip,
            self.now,
        )
        .serialize()?;
        let data_to_encrypt_len = data_to_encrypt.len();
        if data_to_encrypt_len + PADDING_SIZE > self.rsa_size {
            let max_size = self.rsa_size - PADDING_SIZE;
            return Err(format!(
            "Too much data, must be at most {max_size} bytes, but was {data_to_encrypt_len} bytes. \
            Reduce command name length or create a bigger RSA key size."
        ));
        }

        Ok(data_to_encrypt)
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};

    use crate::client::gen::gen;
    use crate::client::send::Sender;
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

        let result = Sender::create(
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

        let result = Sender::create(
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

        let sender = Sender::create(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

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

        let sender = Sender::create(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

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

        let sender = Sender::create(
            SendCommand {
                private_pem_path,
                command: "#".repeat(66),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

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

        let sender = Sender::create(
            SendCommand {
                address: address.to_string(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time()?,
        )
        .unwrap();

        let result = sender.send();

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        result
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
