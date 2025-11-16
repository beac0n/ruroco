//! This module contains all data structs that are needed for the server side services to work.
//! The data that these structs represent are used for invoking the server binaries with CLI
//! (default) arguments or are used to deserialize configuration files

use crate::common::{get_commander_unix_socket_path, info, resolve_path, NTP_SYSTEM};
use crate::server::blocklist::Blocklist;
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::net::{IpAddr, UdpSocket};

use crate::common::crypto_handler::CryptoHandler;
use openssl::version::version;
use std::os::fd::{FromRawFd, RawFd};
use std::path::PathBuf;
use std::{env, fs};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliServer {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub config: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    pub commands: HashMap<String, String>,
    pub ips: Vec<String>,
    #[serde(default = "default_ntp")]
    pub ntp: String,
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
}

impl ConfigServer {
    pub fn deserialize(data: &str) -> Result<ConfigServer, String> {
        toml::from_str::<ConfigServer>(data)
            .map_err(|e| format!("Could not create ConfigServer from {data}: {e}"))
    }

    pub fn validate_ips(&self) -> Result<(), String> {
        for ip in self.ips.iter() {
            ip.parse::<IpAddr>()
                .map_err(|e| format!("Could not parse configured host IP address {ip}: {e}"))?;
        }

        Ok(())
    }

    pub fn create_server_udp_socket(&self, address: Option<String>) -> Result<UdpSocket, String> {
        match (
            env::var("LISTEN_PID").ok(),
            env::var("LISTEN_FDS").ok(),
            env::var("RUROCO_LISTEN_ADDRESS").ok(),
            address,
        ) {
            (_, _, _, Some(address)) => {
                info(&format!("UdpSocket bind to {address} - argument"));
                UdpSocket::bind(&address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
            (_, _, Some(address), _) => {
                info(&format!("UdpSocket bind to {address} - RUROCO_LISTEN_ADDRESS"));
                UdpSocket::bind(&address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
            (Some(listen_pid), Some(listen_fds), _, _)
                if listen_pid == std::process::id().to_string() && listen_fds == "1" =>
            {
                let fd: RawFd = 3;
                info(&format!("UdpSocket from_raw_fd {fd} (systemd socket activation)"));
                let sock = unsafe { UdpSocket::from_raw_fd(fd) };
                Ok(sock)
            }
            (Some(_), Some(listen_fds), _, _) if listen_fds != "1" => {
                Err(format!("LISTEN_FDS was set to {listen_fds}, expected 1"))
            }
            (Some(listen_pid), Some(_), _, _) if listen_pid != std::process::id().to_string() => {
                Err(format!("LISTEN_PID ({listen_pid}) does not match current PID"))
            }
            _ => {
                // port is calculated by using the alphabet indexes of the word ruroco:
                // r = 18, u = 21, r = 18, o = 15, c = 3, o = 15
                // and multiplying the distinct values with each other times two:
                // 18 * 21 * 15 * 3 * 2 = 34020
                let address = "[::]:34020";
                info(&format!("UdpSocket bind to {address} - fallback"));
                UdpSocket::bind(address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
        }
    }

    pub fn create_blocklist(&self) -> Blocklist {
        Blocklist::create(&self.resolve_config_dir())
    }

    pub fn create_crypto_handlers(&self) -> Result<HashMap<Vec<u8>, CryptoHandler>, String> {
        let key_paths = self.get_key_paths()?;
        info(&format!("Creating server, loading keys from {key_paths:?}, using {} ...", version()));

        let crypto_handlers = key_paths
            .into_iter()
            .map(|p| CryptoHandler::from_key_path(&p))
            .collect::<Result<Vec<CryptoHandler>, String>>()?;

        let hashmap_data = crypto_handlers
            .into_iter()
            .map(|h| {
                info(&format!("loading key with id {:X?}", &h.id));
                Ok((h.id.to_vec(), h))
            })
            .collect::<Result<Vec<(Vec<u8>, CryptoHandler)>, String>>()?;

        Ok(hashmap_data.into_iter().collect())
    }

    pub fn get_commander_unix_socket_path(&self) -> PathBuf {
        get_commander_unix_socket_path(&self.resolve_config_dir())
    }

    fn get_key_paths(&self) -> Result<Vec<PathBuf>, String> {
        let config_dir = self.resolve_config_dir();

        let entries: ReadDir = match fs::read_dir(&config_dir) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("Error reading directory {config_dir:?}: {e}")),
        };

        let key_files: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "key"))
            .collect();

        match key_files.len() {
            0 => Err(format!("Could not find any .key files in {config_dir:?}")),
            _ => Ok(key_files),
        }
    }

    fn resolve_config_dir(&self) -> PathBuf {
        resolve_path(&self.config_dir)
    }
}

impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            commands: HashMap::new(),
            ips: vec!["127.0.0.1".to_string()],
            ntp: default_ntp(),
            socket_user: "".to_string(),
            socket_group: "".to_string(),
            config_dir: env::current_dir().unwrap_or(PathBuf::from("/tmp")),
        }
    }
}

fn default_socket_user() -> String {
    "ruroco".to_string()
}

fn default_socket_group() -> String {
    "ruroco".to_string()
}

fn default_ntp() -> String {
    NTP_SYSTEM.to_string()
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

#[cfg(test)]
mod tests {
    use crate::server::config::{
        default_config_path, default_ntp, default_socket_group, default_socket_user, ConfigServer,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_get_key_path() {
        let config_server = ConfigServer {
            config_dir: PathBuf::from("/foo/bar/baz"),
            ..Default::default()
        };

        assert_eq!(
            config_server.get_key_paths().unwrap_err().to_string(),
            r#"Error reading directory "/foo/bar/baz": No such file or directory (os error 2)"#
        );
    }

    #[test]
    fn test_create_deserialize() {
        assert_eq!(
            ConfigServer::deserialize("ips = [\"127.0.0.1\"]\n[commands]").unwrap(),
            ConfigServer {
                commands: HashMap::new(),
                ips: vec!["127.0.0.1".to_string()],
                ntp: default_ntp(),
                config_dir: default_config_path(),
                socket_user: default_socket_user(),
                socket_group: default_socket_group(),
            }
        );
    }
}
