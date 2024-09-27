//! This module contains all data structs that are needed for the server side services to work.
//! The data that these structs represent are used for invoking the server binaries with CLI
//! (default) arguments or are used to deserialize configuration files

use crate::common::NTP_SYSTEM;
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliServer {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub config: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    pub commands: HashMap<String, String>,
    pub ip: String,
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
}

impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            commands: HashMap::new(),
            ip: String::from("127.0.0.1"),
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
            ConfigServer::deserialize("ip = \"127.0.0.1\"\n[commands]").unwrap(),
            ConfigServer {
                commands: HashMap::new(),
                ip: String::from("127.0.0.1"),
                ntp: default_ntp(),
                address: default_address(),
                config_dir: default_config_path(),
                socket_user: default_socket_user(),
                socket_group: default_socket_group(),
            }
        );
    }
}
