//! This module contains all data structs that are needed for the server side services to work.
//! The data that these structs represent are used for invoking the server binaries with CLI
//! (default) arguments or are used to deserialize configuration files

use crate::blocklist::Blocklist;
use crate::common::{error, get_socket_path, info, resolve_path, NTP_SYSTEM};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::net::{IpAddr, UdpSocket};

use openssl::pkey::Public;
use openssl::rsa::Rsa;
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
    #[serde(default = "default_address")]
    pub address: String,
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

    pub fn create_udp_socket(&self) -> Result<UdpSocket, String> {
        let address = &self.address;
        let pid = std::process::id().to_string();
        match env::var("LISTEN_PID") {
            Ok(listen_pid) if listen_pid == pid => {
                info(String::from(
                    "env var LISTEN_PID was set to our PID, creating socket from raw fd ...",
                ));
                let fd: RawFd = 3;
                Ok(unsafe { UdpSocket::from_raw_fd(fd) })
            }
            Ok(_) => {
                info(format!(
                    "env var LISTEN_PID was set, but not to our PID, binding to {address}"
                ));
                UdpSocket::bind(address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
            Err(_) => {
                info(format!("env var LISTEN_PID was not set, binding to {address}"));
                UdpSocket::bind(address)
                    .map_err(|e| format!("Could not UdpSocket bind {address:?}: {e}"))
            }
        }
    }

    pub fn create_blocklist(&self) -> Blocklist {
        Blocklist::create(&self.resolve_config_dir())
    }

    pub fn create_rsa(&self) -> Result<Rsa<Public>, String> {
        let pem_path = self.get_pem_path()?;
        info(format!(
            "Creating server, loading public PEM from {pem_path:?}, using {} ...",
            version()
        ));

        let pem_data =
            fs::read(&pem_path).map_err(|e| format!("Could not read {pem_path:?}: {e}"))?;

        Rsa::public_key_from_pem(&pem_data)
            .map_err(|e| format!("Could not load public key from {pem_path:?}: {e}"))
    }

    pub fn get_socket_path(&self) -> PathBuf {
        get_socket_path(&self.resolve_config_dir())
    }

    fn get_pem_path(&self) -> Result<PathBuf, String> {
        let config_dir = self.resolve_config_dir();
        let pem_files = Self::get_pem_files(&config_dir);

        match pem_files.len() {
            0 => Err(format!("Could not find any .pem files in {config_dir:?}")),
            1 => Ok(pem_files.first().unwrap().clone()),
            other => Err(format!("Only one public PEM is supported, found {other}")),
        }
    }

    fn resolve_config_dir(&self) -> PathBuf {
        resolve_path(&self.config_dir)
    }

    fn get_pem_files(config_dir: &PathBuf) -> Vec<PathBuf> {
        let entries: ReadDir = match fs::read_dir(config_dir) {
            Ok(entries) => entries,
            Err(e) => {
                error(format!("Error reading directory: {e}"));
                return vec![];
            }
        };

        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().is_some() && path.extension().unwrap() == "pem"
            })
            .collect()
    }
}

impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            commands: HashMap::new(),
            ips: vec![String::from("127.0.0.1")],
            ntp: default_ntp(),
            address: String::from(""),
            socket_user: String::from(""),
            socket_group: String::from(""),
            config_dir: env::current_dir().unwrap_or(PathBuf::from("/tmp")),
        }
    }
}

fn default_socket_user() -> String {
    String::from("ruroco")
}

fn default_socket_group() -> String {
    String::from("ruroco")
}

fn default_ntp() -> String {
    String::from(NTP_SYSTEM)
}

fn default_address() -> String {
    String::from("127.0.0.1:8080")
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

#[cfg(test)]
mod tests {
    use crate::config_server::{
        default_address, default_config_path, default_ntp, default_socket_group,
        default_socket_user, ConfigServer,
    };
    use std::collections::HashMap;

    #[test]
    fn test_create_deserialize() {
        assert_eq!(
            ConfigServer::deserialize("ips = [\"127.0.0.1\"]\n[commands]").unwrap(),
            ConfigServer {
                commands: HashMap::new(),
                ips: vec![String::from("127.0.0.1")],
                ntp: default_ntp(),
                address: default_address(),
                config_dir: default_config_path(),
                socket_user: default_socket_user(),
                socket_group: default_socket_group(),
            }
        );
    }
}
