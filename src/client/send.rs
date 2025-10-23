use crate::common::crypto_handler::CryptoHandler;
use crate::common::data::ClientData;
use crate::common::info;
use crate::config::config_client::SendCommand;
use openssl::version::version;
use std::fmt::{Debug, Display};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};

#[derive(Debug)]
pub struct Sender {
    cmd: SendCommand,
    now: u128,
    crypto_handler: CryptoHandler,
}

impl Sender {
    /// Create a new Sender instance
    ///
    /// * `send_command` - data holding information how to send the command - see SendCommand
    /// * `now` - current timestamp in ns
    pub fn create(cmd: SendCommand, now: u128) -> Result<Self, String> {
        Ok(Self {
            crypto_handler: CryptoHandler::from_key_path(&cmd.key)?,
            cmd,
            now,
        })
    }

    /// Send data to the server to execute a predefined command
    pub fn send(&self) -> Result<(), String> {
        info(&format!(
            "Connecting to udp://{}, loading key from {:?}, using {} ...",
            &self.cmd.address,
            &self.cmd.key,
            version(),
        ));

        let destination_ips_validated = self.get_destination_ips()?;
        info(&format!("Found IPs {destination_ips_validated:?} for {}", &self.cmd.address));
        match destination_ips_validated.as_slice() {
            [destination_ip] => self.send_data(*destination_ip)?,
            [_, ipv6_destination_ip] => self.send_data(*ipv6_destination_ip)?,
            _ => return Err(format!("Found too many IPs: {destination_ips_validated:?}")),
        }

        Ok(())
    }

    fn get_destination_ips(&self) -> Result<Vec<IpAddr>, String> {
        let address = &self.cmd.address;

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

        let use_ip_undef = self.cmd.ipv4 == self.cmd.ipv6;

        let cnfa = "Could not find any";
        let afa = format!("address for {address}");

        Ok(match (destination_ipv4s.first(), destination_ipv6s.first()) {
            // ipv4 or ipv6 where not defined or where both defined
            (Some(ipv4), Some(ipv6)) if use_ip_undef => vec![ipv4.ip(), ipv6.ip()],
            (Some(ipv4), None) if use_ip_undef => vec![ipv4.ip()],
            (None, Some(ipv6)) if use_ip_undef => vec![ipv6.ip()],
            // ipv4 xor ipv6 where defined
            (_, Some(ipv6)) if self.cmd.ipv6 => vec![ipv6.ip()],
            (Some(ipv4), _) if self.cmd.ipv4 => vec![ipv4.ip()],
            (_, None) if self.cmd.ipv6 => return Err(format!("{cnfa} IPv6 {afa}")),
            (None, _) if self.cmd.ipv4 => return Err(format!("{cnfa} IPv4 {afa}")),
            // could not find any address
            _ => return Err(format!("{cnfa} IPv4 or IPv6 {afa}")),
        })
    }

    fn send_data(&self, ip: IpAddr) -> Result<(), String> {
        let ip_str = ip.to_string();
        let bind_address = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };

        info(&format!("Connecting to {ip_str}..."));
        let data_to_encrypt = self.get_data_to_encrypt(ip_str)?;
        let data_to_send = self.get_data_to_send(&data_to_encrypt)?;

        // create UDP socket and send the encrypted data to the specified address
        let address = &self.cmd.address;
        let socket = UdpSocket::bind(bind_address).map_err(|e| Self::socket_err(e, address))?;
        socket.connect(address).map_err(|e| Self::socket_err(e, address))?;
        socket.send(&data_to_send).map_err(|e| Self::socket_err(e, address))?;

        info(&format!("Sent command {} from {bind_address} to udp://{address}", &self.cmd.command));
        Ok(())
    }

    fn get_data_to_send(&self, data_to_encrypt: &[u8]) -> Result<Vec<u8>, String> {
        let (iv, cipher, tag) = self.crypto_handler.encrypt(data_to_encrypt)?;
        let data_to_send_len = iv.len() + cipher.len() + tag.len();
        let max_size = 160;
        if data_to_send_len > max_size {
            return Err(format!(
                "Too much data, must be at most {max_size} bytes, \
                but was {data_to_send_len} bytes. Reduce command name length."
            ));
        }

        Ok([iv, cipher, tag].concat())
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

        Ok(data_to_encrypt)
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};

    use crate::client::gen::Generator;
    use crate::client::send::Sender;
    use crate::common::time;
    use crate::config::config_client::{CliClient, SendCommand};
    use std::fs;
    use std::fs::File;
    use std::net::IpAddr;
    use std::path::PathBuf;

    const IP: &str = "192.168.178.123";

    #[test]
    fn test_get_2_destination_ips() {
        assert_eq!(get_ip_addresses("google.com:80").len(), 2);
    }

    #[test]
    fn test_get_ivp6_destination_ips() {
        assert_eq!(get_ip_addresses("ipv6.google.com:80").len(), 1);
    }

    #[test]
    fn test_get_ivp4_destination_ips() {
        assert_eq!(get_ip_addresses("ipv4.google.com:80").len(), 1);
    }

    #[test]
    fn test_send_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "send", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_send_no_such_file() {
        let key_file_name = gen_file_name(".key");

        let result = Sender::create(
            SendCommand {
                key: PathBuf::from(&key_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        assert_eq!(
            result.unwrap_err(),
            format!("Could not read key file: No such file or directory (os error 2)")
        );
    }

    #[test]
    fn test_send_invalid_key() {
        let key_file_name = gen_file_name(".key");
        File::create(&key_file_name).unwrap();

        let result = Sender::create(
            SendCommand {
                key: PathBuf::from(&key_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&key_file_name);

        assert_eq!(result.unwrap_err(), "Key length must be 32");
    }

    #[test]
    fn test_send_invalid_port_value() {
        let key_file = gen_file_name(".key");

        let key_path = PathBuf::from(&key_file);
        Generator::create(&key_path).unwrap().gen().unwrap();

        let address = "127.0.0.1:asd".to_string();

        let sender = Sender::create(
            SendCommand {
                address: address.clone(),
                key: key_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

        let _ = fs::remove_file(&key_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}: invalid port value")
        );
    }

    #[test]
    fn test_send_unknown_service() {
        let key_file = gen_file_name(".key");

        let key_path = PathBuf::from(&key_file);
        Generator::create(&key_path).unwrap().gen().unwrap();

        let address = "999.999.999.999:9999".to_string();

        let sender = Sender::create(
            SendCommand {
                address: address.clone(),
                key: key_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

        let _ = fs::remove_file(&key_file);

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
        let key_file = gen_file_name(".key");
        let key_path = PathBuf::from(&key_file);

        Generator::create(&key_path).unwrap().gen().unwrap();

        let sender = Sender::create(
            SendCommand {
                key: key_path,
                command: "#".repeat(66),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        )
        .unwrap();

        let result = sender.send();

        let _ = fs::remove_file(&key_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            "Too much data, must be at most 160 bytes, but was 176 bytes. \
                Reduce command name length."
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
        let key_file = gen_file_name(".key");

        let key_path = PathBuf::from(&key_file);
        Generator::create(&key_path)?.gen()?;

        let sender = Sender::create(
            SendCommand {
                address: address.to_string(),
                key: key_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time()?,
        );

        let result = sender?.send();

        let _ = fs::remove_file(&key_file);

        result
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }

    fn get_ip_addresses(host: &str) -> Vec<IpAddr> {
        let key_file = gen_file_name(".key");
        let key_path = PathBuf::from(&key_file);
        Generator::create(&key_path).unwrap().gen().unwrap();

        let sender = Sender::create(
            SendCommand {
                address: host.to_string(),
                key: key_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let ip_addrs = sender.unwrap().get_destination_ips().unwrap();
        dbg!(&ip_addrs);
        ip_addrs
    }
}
