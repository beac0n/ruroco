use crate::client::config::{get_conf_dir, SendCommand};
use crate::client::counter::Counter;
use crate::common::client_data::ClientData;
use crate::common::crypto_handler::PLAINTEXT_SIZE;
use crate::common::data_parser::DataParser;
use crate::common::{info, resolve_path};
use openssl::version::version;
use std::fmt::{Debug, Display};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
pub struct Sender {
    cmd: SendCommand,
    data_parser: DataParser,
    counter: u128,
}

impl Sender {
    /// Create a new Sender instance
    ///
    /// * `send_command` - data holding information how to send the command - see SendCommand
    pub fn create(cmd: SendCommand) -> Result<Self, String> {
        let counter_path = Self::get_counter_path()?;
        info(&format!("Loading counter from {counter_path:?} ..."));
        let initial_counter = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Could not get duration since {:?}: {e}", SystemTime::UNIX_EPOCH))?
            .as_nanos();
        let mut counter = Counter::create(counter_path, initial_counter)?;
        counter.inc()?;
        Ok(Self {
            data_parser: DataParser::create(&cmd.key)?,
            cmd,
            counter: counter.count(),
        })
    }

    pub fn get_counter_path() -> Result<PathBuf, String> {
        Ok(resolve_path(&get_conf_dir()?).join("counter"))
    }

    /// Send data to the server to execute a predefined command
    pub fn send(&self) -> Result<(), String> {
        info(&format!("Connecting to udp://{}, using {} ...", &self.cmd.address, version(),));
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
        Ok(match (destination_ipv4s.first(), destination_ipv6s.first()) {
            // ipv4 or ipv6 where not defined or where both defined
            (Some(ipv4), Some(ipv6)) if use_ip_undef => vec![ipv4.ip(), ipv6.ip()],
            (Some(ipv4), None) if use_ip_undef => vec![ipv4.ip()],
            (None, Some(ipv6)) if use_ip_undef => vec![ipv6.ip()],
            // ipv4 xor ipv6 where defined
            (_, Some(ipv6)) if self.cmd.ipv6 => vec![ipv6.ip()],
            (Some(ipv4), _) if self.cmd.ipv4 => vec![ipv4.ip()],
            (_, None) if self.cmd.ipv6 => {
                return Err(format!("Could not find any IPv6 address for {address}"))
            }
            (None, _) if self.cmd.ipv4 => {
                return Err(format!("Could not find any IPv4 address for {address}"))
            }
            // could not find any address
            _ => return Err(format!("Could not find any IPv4 or IPv6 address for {address}")),
        })
    }

    fn send_data(&self, ip: IpAddr) -> Result<(), String> {
        let bind_address = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };

        info(&format!("Connecting to {ip}..."));
        let data_to_encrypt = self.get_data_to_encrypt(ip)?;
        let data_to_send = self.data_parser.encode(&data_to_encrypt)?;

        // create UDP socket and send the encrypted data to the specified address
        let address = &self.cmd.address;
        let socket = UdpSocket::bind(bind_address).map_err(|e| Self::socket_err(e, address))?;
        socket.connect(address).map_err(|e| Self::socket_err(e, address))?;
        socket.send(&data_to_send).map_err(|e| Self::socket_err(e, address))?;

        info(&format!("Sent command {} from {bind_address} to udp://{address}", &self.cmd.command));
        Ok(())
    }

    fn socket_err<I: Display, E: Debug>(err: I, val: E) -> String {
        format!("Could not connect/send data to {val:?}: {err}")
    }

    fn get_data_to_encrypt(&self, destination_ip: IpAddr) -> Result<[u8; PLAINTEXT_SIZE], String> {
        ClientData::create(
            &self.cmd.command,
            !self.cmd.permissive,
            self.cmd.ip.clone().and_then(|d| d.parse().ok()),
            destination_ip,
            self.counter,
        )?
        .serialize()
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    use crate::client::config::{CliClient, SendCommand};
    use crate::client::gen::Generator;
    use crate::client::send::Sender;
    use crate::common::get_random_string;
    use std::fs;
    use std::fs::File;
    use std::net::IpAddr;

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
    fn test_send_invalid_key() {
        let key_file_name = gen_file_name(".key");
        File::create(&key_file_name).unwrap();

        let result = Sender::create(SendCommand {
            key: "DEADBEEF".to_string(),
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        let _ = fs::remove_file(&key_file_name);

        assert_eq!(result.unwrap_err(), "Key too short");
    }

    #[test]
    fn test_send_invalid_port_value() {
        let key = Generator::create().unwrap().gen().unwrap();
        let address = "127.0.0.1:asd".to_string();
        let sender = Sender::create(SendCommand {
            address: address.clone(),
            key,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}: invalid port value")
        );
    }

    #[test]
    fn test_send_unknown_service() {
        let address = "999.999.999.999:9999".to_string();
        let sender = Sender::create(SendCommand {
            address: address.clone(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();
        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "Could not resolve hostname for {address}: \
                failed to lookup address information: Name or service not known"
            )
        );
    }

    #[test]
    fn test_send_huge_command() {
        let sender = Sender::create(SendCommand {
            address: "[::ffff:127.0.0.1]:1234".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            command: "#".repeat(6000),
            ip: Some("::ffff:192.168.178.123".to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();
        assert!(result.is_ok(),);
    }

    #[test]
    fn test_send_ipv4() {
        assert!(send_test("127.0.0.1:1234").is_ok());
    }

    #[test]
    fn test_send_ipv6() {
        assert!(send_test("[::1]:1234").is_ok());
    }

    fn send_test(address: &str) -> Result<(), String> {
        let sender = Sender::create(SendCommand {
            address: address.to_string(),
            key: Generator::create()?.gen()?,
            ip: Some(IP.to_string()),
            ..Default::default()
        });
        sender?.send()
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = get_random_string(16).unwrap();
        format!("{rand_str}{suffix}")
    }

    fn get_ip_addresses(host: &str) -> Vec<IpAddr> {
        let sender = Sender::create(SendCommand {
            address: host.to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        let ip_addrs = sender.unwrap().get_destination_ips().unwrap();
        dbg!(&ip_addrs);
        ip_addrs
    }
}
