use crate::client::config::{get_conf_dir, SendCommand};
use crate::client::counter::Counter;
use crate::common::client_data::ClientData;
use crate::common::data_parser::DataParser;
use crate::common::protocol::PLAINTEXT_SIZE;
use crate::common::{info, now_nanos, resolve_path};
use openssl::version::version;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct Sender {
    pub(super) cmd: SendCommand,
    pub(super) data_parser: DataParser,
    pub(super) counter: Counter,
}

impl Sender {
    pub fn create(mut cmd: SendCommand) -> anyhow::Result<Self> {
        cmd.address = Self::ensure_port(cmd.address, crate::common::DEFAULT_PORT);
        let counter_path = Self::get_counter_path()?;
        info(format!("Loading counter from {counter_path:?} ..."));
        Ok(Self {
            data_parser: DataParser::create(&cmd.key)?,
            cmd,
            counter: Counter::create_and_init(counter_path, now_nanos()?)?,
        })
    }

    fn ensure_port(address: String, default_port: u16) -> String {
        if address.starts_with('[') {
            // IPv6 literal: [::1] or [::1]:port
            if address.contains("]:") {
                address
            } else {
                format!("{address}:{default_port}")
            }
        } else if address.contains(':') {
            // IPv4 with port (1.2.3.4:port) or bare IPv6 — keep as-is
            address
        } else {
            // hostname or IPv4 without port
            format!("{address}:{default_port}")
        }
    }

    pub fn get_counter_path() -> anyhow::Result<PathBuf> {
        Ok(resolve_path(&get_conf_dir()?).join("counter"))
    }

    /// Send data to the server to execute a predefined command
    pub fn send(&mut self) -> anyhow::Result<()> {
        info(format!("Connecting to udp://{}, using {} ...", &self.cmd.address, version(),));
        let destination_ips_validated = self.get_destination_ips()?;
        info(format!("Found IPs {destination_ips_validated:?} for {}", &self.cmd.address));
        for (i, destination_ip) in destination_ips_validated.iter().enumerate() {
            if i > 0 && self.cmd.send_delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.cmd.send_delay_ms));
            }
            self.send_data(*destination_ip)?;
        }

        Ok(())
    }

    pub(super) fn get_data_to_encrypt(
        &self,
        destination_ip: IpAddr,
    ) -> anyhow::Result<[u8; PLAINTEXT_SIZE]> {
        ClientData::create(
            &self.cmd.command,
            !self.cmd.permissive,
            self.cmd.ip.clone().and_then(|d| d.parse().ok()),
            destination_ip,
            self.counter.count(),
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
    use std::fs;
    use std::fs::File;
    use std::net::IpAddr;
    use tempfile::TempDir;

    const IP: &str = "192.168.178.123";

    fn set_test_conf_dir() -> TempDir {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        dir
    }

    #[test]
    fn test_get_2_destination_ips() {
        let _conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses("google.com:80").len(), 2);
    }

    #[test]
    fn test_get_ivp6_destination_ips() {
        let _conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses("ipv6.google.com:80").len(), 1);
    }

    #[test]
    fn test_get_ivp4_destination_ips() {
        let _conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses("ipv4.google.com:80").len(), 1);
    }

    #[test]
    fn test_send_print_help() {
        let _conf_dir = set_test_conf_dir();
        let result = CliClient::try_parse_from(vec!["ruroco", "send", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_send_invalid_key() {
        let _conf_dir = set_test_conf_dir();
        let key_file_name = "test.key";
        File::create(key_file_name).unwrap();

        let result = Sender::create(SendCommand {
            key: "DEADBEEF".to_string(),
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        let _ = fs::remove_file(key_file_name);

        assert_eq!(result.unwrap_err().to_string(), "Key too short");
    }

    #[test]
    fn test_send_invalid_port_value() {
        let _conf_dir = set_test_conf_dir();
        let key = Generator::create().unwrap().gen().unwrap();
        let address = "127.0.0.1:asd".to_string();
        let mut sender = Sender::create(SendCommand {
            address: address.clone(),
            key,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}")
        );
    }

    #[test]
    fn test_send_unknown_service() {
        let _conf_dir = set_test_conf_dir();
        let address = "999.999.999.999:9999".to_string();
        let mut sender = Sender::create(SendCommand {
            address: address.clone(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}")
        );
    }

    #[test]
    fn test_send_huge_command() {
        let _conf_dir = set_test_conf_dir();
        let mut sender = Sender::create(SendCommand {
            address: "[::ffff:127.0.0.1]:1234".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            command: "#".repeat(6000),
            ip: Some("::ffff:192.168.178.123".to_string()),
            ..Default::default()
        })
        .unwrap();

        let result = sender.send();
        assert!(result.is_ok(), "send_huge_command failed: {result:?}");
    }

    #[test]
    fn test_send_ipv4() {
        let _conf_dir = set_test_conf_dir();
        let result = send_test("127.0.0.1:1234");
        assert!(result.is_ok(), "send_ipv4 failed: {result:?}");
    }

    #[test]
    fn test_send_ipv6() {
        let _conf_dir = set_test_conf_dir();
        let result = send_test("[::1]:1234");
        assert!(result.is_ok(), "send_ipv6 failed: {result:?}");
    }

    fn send_test(address: &str) -> anyhow::Result<()> {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: address.to_string(),
            key: Generator::create()?.gen()?,
            ip: Some(IP.to_string()),
            ..Default::default()
        });
        sender?.send()
    }

    fn get_ip_addresses(host: &str) -> Vec<IpAddr> {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: host.to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        sender.unwrap().get_destination_ips().unwrap()
    }

    #[test]
    fn test_get_destination_ips_ipv4_only_flag() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ipv4: true,
            ipv6: false,
            ..Default::default()
        })
        .unwrap();

        let ips = sender.get_destination_ips().unwrap();
        assert!(ips.iter().all(|ip| ip.is_ipv4()));
    }

    #[test]
    fn test_get_destination_ips_ipv6_only_flag() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ipv4: false,
            ipv6: true,
            ..Default::default()
        })
        .unwrap();

        let ips = sender.get_destination_ips().unwrap();
        assert!(ips.iter().all(|ip| ip.is_ipv6()));
    }

    #[test]
    fn test_get_destination_ips_ipv4_flag_no_ipv4_available() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "ipv6.google.com:80".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ipv4: true,
            ipv6: false,
            ..Default::default()
        })
        .unwrap();

        let result = sender.get_destination_ips();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find any IPv4"));
    }

    #[test]
    fn test_get_destination_ips_ipv6_flag_no_ipv6_available() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "ipv4.google.com:80".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ipv4: false,
            ipv6: true,
            ..Default::default()
        })
        .unwrap();

        let result = sender.get_destination_ips();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find any IPv6"));
    }

    #[test]
    fn test_get_counter_path() {
        let _conf_dir = set_test_conf_dir();
        let path = Sender::get_counter_path().unwrap();
        assert!(path.to_str().unwrap().ends_with("counter"));
    }

    #[test]
    fn test_ensure_port_ipv6_without_port() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "[::1]".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sender.cmd.address, "[::1]:80");
    }

    #[test]
    fn test_ensure_port_ipv4_without_port() {
        let _conf_dir = set_test_conf_dir();
        let sender = Sender::create(SendCommand {
            address: "127.0.0.1".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sender.cmd.address, "127.0.0.1:80");
    }

    #[test]
    fn test_send_delay_applied_for_second_ip() {
        let _conf_dir = set_test_conf_dir();
        let mut sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key: Generator::create().unwrap().gen().unwrap(),
            ip: Some(IP.to_string()),
            send_delay_ms: 1,
            ..Default::default()
        })
        .unwrap();
        // google.com resolves to both IPv4 and IPv6, so the delay path is hit for the 2nd IP
        let _ = sender.send();
    }
}
