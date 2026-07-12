use crate::client::config::{get_conf_dir, SendCommand};
use crate::client::counter::Counter;
use crate::common::client_data::ClientData;
use crate::common::data_parser::DataParser;
use crate::common::logging::error;
use crate::common::protocol::PLAINTEXT_SIZE;
use crate::common::{info, now_nanos, resolve_path};
use anyhow::{bail, Context};
use openssl::version::version;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroizing;

#[derive(Debug)]
pub struct Sender {
    pub(super) cmd: SendCommand,
    pub(super) src_ip: Option<IpAddr>,
    pub(super) data_parser: DataParser,
    pub(super) counter: Counter,
}

impl Sender {
    pub fn create(mut cmd: SendCommand) -> anyhow::Result<Self> {
        cmd.address = Self::ensure_port(cmd.address, crate::common::DEFAULT_PORT)?;
        let src_ip = cmd
            .ip
            .clone()
            .map(|ip| ip.parse().with_context(|| format!("Invalid --ip value {ip:?}")))
            .transpose()?;
        let key: Zeroizing<String> = std::fs::read_to_string(&cmd.key_file)
            .with_context(|| format!("Could not read key file {:?}", cmd.key_file))?
            .into();
        let counter_path = Self::get_counter_path()?;
        info(format!("Loading counter from {counter_path:?} ..."));
        Ok(Self {
            data_parser: DataParser::create(key.trim())?,
            cmd,
            src_ip,
            counter: Counter::create_and_init(counter_path, now_nanos()?)?,
        })
    }

    /// Normalize the destination address: a complete socket address ("1.2.3.4:80", "[::1]:80")
    /// is kept as-is, a bare IP ("127.0.0.1", "::1") or bare hostname gets the default port
    /// appended, and "host:port" is kept for DNS resolution later. Everything else (non-numeric
    /// port, bracketed IPv6 without port) is rejected here instead of surfacing as a misleading
    /// resolution error.
    fn ensure_port(address: String, default_port: u16) -> anyhow::Result<String> {
        if address.parse::<SocketAddr>().is_ok() {
            return Ok(address);
        }

        if let Ok(ip) = address.parse::<IpAddr>() {
            return Ok(SocketAddr::new(ip, default_port).to_string());
        }

        match address.rsplit_once(':') {
            None if !address.is_empty() => Ok(format!("{address}:{default_port}")),
            Some((host, port)) if !host.is_empty() && !host.contains(':') => port
                .parse::<u16>()
                .map(|_| address.clone())
                .with_context(|| format!("Invalid port {port:?} in address {address:?}")),
            _ => bail!(
                "Invalid address {address:?}, expected \"host\", \"host:port\", \"ip\" or \"ip:port\""
            ),
        }
    }

    pub fn get_counter_path() -> anyhow::Result<PathBuf> {
        Ok(resolve_path(&get_conf_dir()?).join("counter"))
    }

    /// Send data to the server to execute a predefined command
    pub fn send(&mut self) -> anyhow::Result<()> {
        info(format!("Connecting to udp://{}, using {} ...", &self.cmd.address, version(),));
        let destination_addrs = self.get_destination_ips()?;
        info(format!("Found addresses {destination_addrs:?} for {}", &self.cmd.address));
        self.send_to_destinations(&destination_addrs)
    }

    /// Attempts every destination in turn instead of stopping at the first failure - e.g. a
    /// hostname resolving to both an IPv4 and an IPv6 address should still reach the server over
    /// whichever address family actually works, rather than giving up on IPv6 just because IPv4
    /// failed first. Only fails if every destination did.
    fn send_to_destinations(&mut self, destination_addrs: &[SocketAddr]) -> anyhow::Result<()> {
        let mut failures = Vec::new();
        for (i, destination_addr) in destination_addrs.iter().enumerate() {
            if i > 0 && self.cmd.send_delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.cmd.send_delay_ms));
            }
            if let Err(e) = self.send_data(*destination_addr) {
                error(format!("Could not send to {destination_addr}: {e}"));
                failures.push(format!("{destination_addr}: {e}"));
            }
        }

        if !destination_addrs.is_empty() && failures.len() == destination_addrs.len() {
            bail!("Could not send to any destination: {}", failures.join("; "));
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
            self.src_ip,
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
    use std::net::SocketAddr;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const IP: &str = "192.168.178.123";

    fn set_test_conf_dir() -> TempDir {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        dir
    }

    /// Writes a freshly generated key to `<dir>/test.key` and returns its path, for tests that
    /// need a `key_file` pointing at a real, valid key.
    fn write_key_file(dir: &Path) -> PathBuf {
        let key = Generator::create().unwrap().gen().unwrap();
        let path = dir.join("test.key");
        std::fs::write(&path, key).unwrap();
        path
    }

    #[test_with::env(TEST_ONLINE)]
    #[test]
    fn test_get_2_destination_ips() {
        let conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses(&conf_dir, "google.com:80").len(), 2);
    }

    #[test_with::env(TEST_ONLINE)]
    #[test]
    fn test_get_ivp6_destination_ips() {
        let conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses(&conf_dir, "ipv6.google.com:80").len(), 1);
    }

    #[test_with::env(TEST_ONLINE)]
    #[test]
    fn test_get_ivp4_destination_ips() {
        let conf_dir = set_test_conf_dir();
        assert_eq!(get_ip_addresses(&conf_dir, "ipv4.google.com:80").len(), 1);
    }

    #[test]
    fn test_send_print_help() {
        let _conf_dir = set_test_conf_dir();
        let result = CliClient::try_parse_from(vec!["ruroco", "send", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_send_invalid_ip() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());

        let result = Sender::create(SendCommand {
            key_file,
            ip: Some("not-an-ip".to_string()),
            ..Default::default()
        });

        assert!(result.unwrap_err().to_string().contains("Invalid --ip value"));
    }

    #[test]
    fn test_send_invalid_key() {
        let conf_dir = set_test_conf_dir();
        let key_file = conf_dir.path().join("test.key");
        std::fs::write(&key_file, "DEADBEEF").unwrap();

        let result = Sender::create(SendCommand {
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        assert_eq!(result.unwrap_err().to_string(), "Key too short");
    }

    #[test]
    fn test_send_invalid_key_file_missing() {
        let conf_dir = set_test_conf_dir();

        let result = Sender::create(SendCommand {
            key_file: conf_dir.path().join("does_not_exist.key"),
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        assert!(result.unwrap_err().to_string().contains("Could not read key file"));
    }

    #[test]
    fn test_send_invalid_port_value() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());

        let result = Sender::create(SendCommand {
            address: "127.0.0.1:asd".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid port"), "unexpected error: {err}");
    }

    #[test]
    fn test_send_unknown_service() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let address = "999.999.999.999:9999".to_string();
        let mut sender = Sender::create(SendCommand {
            address: address.clone(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let mut sender = Sender::create(SendCommand {
            address: "[::ffff:127.0.0.1]:1234".to_string(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let result = send_test(&conf_dir, "127.0.0.1:1234");
        assert!(result.is_ok(), "send_ipv4 failed: {result:?}");
    }

    #[test]
    fn test_send_ipv6() {
        let conf_dir = set_test_conf_dir();
        let result = send_test(&conf_dir, "[::1]:1234");
        assert!(result.is_ok(), "send_ipv6 failed: {result:?}");
    }

    #[test]
    fn test_send_to_destinations_continues_after_one_fails() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let mut sender = Sender::create(SendCommand {
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        // 255.255.255.255 is the broadcast address; sending to it without SO_BROADCAST reliably
        // fails with "Permission denied" - a deterministic stand-in for "this destination is
        // unreachable" that doesn't depend on real network conditions.
        let unreachable: SocketAddr = "255.255.255.255:1234".parse().unwrap();
        let reachable: SocketAddr = "127.0.0.1:1234".parse().unwrap();

        let result = sender.send_to_destinations(&[unreachable, reachable]);

        assert!(result.is_ok(), "one working destination must be enough: {result:?}");
    }

    #[test]
    fn test_send_to_destinations_fails_when_all_destinations_fail() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let mut sender = Sender::create(SendCommand {
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();

        let unreachable: SocketAddr = "255.255.255.255:1234".parse().unwrap();

        let result = sender.send_to_destinations(&[unreachable, unreachable]);

        assert!(result.unwrap_err().to_string().contains("Could not send to any destination"));
    }

    fn send_test(conf_dir: &TempDir, address: &str) -> anyhow::Result<()> {
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: address.to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        });
        sender?.send()
    }

    fn get_ip_addresses(conf_dir: &TempDir, host: &str) -> Vec<SocketAddr> {
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: host.to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        });

        sender.unwrap().get_destination_ips().unwrap()
    }

    #[test]
    fn test_get_destination_ips_ipv4_only_flag() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "ipv6.google.com:80".to_string(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "ipv4.google.com:80".to_string(),
            key_file,
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
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "::1".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sender.cmd.address, "[::1]:80");
    }

    #[test]
    fn test_ensure_port_bracketed_ipv6_without_port_is_rejected() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let result = Sender::create(SendCommand {
            address: "[::1]".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        });
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid address"), "unexpected error: {err}");
    }

    #[test]
    fn test_ensure_port_hostname_without_port() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "schempp.dev".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sender.cmd.address, "schempp.dev:80");
    }

    #[test]
    fn test_ensure_port_ipv4_without_port() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let sender = Sender::create(SendCommand {
            address: "127.0.0.1".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sender.cmd.address, "127.0.0.1:80");
    }

    #[test_with::env(TEST_ONLINE)]
    #[test]
    fn test_send_delay_applied_for_second_ip() {
        let conf_dir = set_test_conf_dir();
        let key_file = write_key_file(conf_dir.path());
        let mut sender = Sender::create(SendCommand {
            address: "google.com:80".to_string(),
            key_file,
            ip: Some(IP.to_string()),
            send_delay_ms: 1,
            ..Default::default()
        })
        .unwrap();
        // google.com resolves to both IPv4 and IPv6, so the delay path is hit for the 2nd IP
        let _ = sender.send();
    }
}
