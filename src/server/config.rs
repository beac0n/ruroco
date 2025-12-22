//! This module contains all data structs that are needed for the server side services to work.
//! The data that these structs represent are used for invoking the server binaries with CLI
//! (default) arguments or are used to deserialize configuration files

use crate::common::{blake2b_u64, info, resolve_path};
use crate::server::blocklist::Blocklist;
use anyhow::{anyhow, Context};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::net::{IpAddr, UdpSocket};

use crate::common::crypto_handler::{CryptoHandler, KEY_ID_SIZE};
use crate::server::util::get_commander_unix_socket_path;
use openssl::version::version;
use std::os::fd::{FromRawFd, RawFd};
use std::path::PathBuf;
use std::{env, fs};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliServer {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub(crate) config: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    pub commands: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_ips")]
    pub ips: Vec<IpAddr>,
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
}

fn deserialize_ips<'de, D>(d: D) -> Result<Vec<IpAddr>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<String> = Vec::<String>::deserialize(d)?;
    v.into_iter().map(|s| s.parse().map_err(serde::de::Error::custom)).collect()
}

impl ConfigServer {
    pub(crate) fn get_hash_to_cmd(&self) -> anyhow::Result<HashMap<u64, String>> {
        self.commands
            .iter()
            .map(|(k, v)| {
                let hash = blake2b_u64(k).with_context(|| format!("Could not hash {k}"))?;
                Ok((hash, v.clone()))
            })
            .collect()
    }
    pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigServer> {
        toml::from_str::<ConfigServer>(data)
            .with_context(|| format!("Could not create ConfigServer from {data}"))
    }

    pub(crate) fn create_server_udp_socket(
        &self,
        address: Option<String>,
    ) -> anyhow::Result<UdpSocket> {
        match (
            env::var("LISTEN_PID").ok(),
            env::var("LISTEN_FDS").ok(),
            env::var("RUROCO_LISTEN_ADDRESS").ok(),
            address,
        ) {
            (_, _, _, Some(address)) => {
                info(&format!("UdpSocket bind to {address} - argument"));
                UdpSocket::bind(&address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
            }
            (_, _, Some(address), _) => {
                info(&format!("UdpSocket bind to {address} - RUROCO_LISTEN_ADDRESS"));
                UdpSocket::bind(&address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
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
                Err(anyhow!("LISTEN_FDS was set to {listen_fds}, expected 1"))
            }
            (Some(listen_pid), Some(_), _, _) if listen_pid != std::process::id().to_string() => {
                Err(anyhow!("LISTEN_PID ({listen_pid}) does not match current PID"))
            }
            _ => {
                // port is calculated by using the alphabet indexes of the word ruroco:
                // r = 18, u = 21, r = 18, o = 15, c = 3, o = 15
                // and multiplying the distinct values with each other times two:
                // 18 * 21 * 15 * 3 * 2 = 34020
                let address = "[::]:34020";
                info(&format!("UdpSocket bind to {address} - fallback"));
                UdpSocket::bind(address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
            }
        }
    }

    pub(crate) fn create_blocklist(&self) -> anyhow::Result<Blocklist> {
        Blocklist::create(&self.resolve_config_dir())
    }

    pub(crate) fn create_crypto_handlers(
        &self,
    ) -> anyhow::Result<HashMap<[u8; KEY_ID_SIZE], CryptoHandler>> {
        let key_paths = self.get_key_paths()?;
        info(&format!("Creating server, loading keys from {key_paths:?}, using {} ...", version()));

        let crypto_handlers = key_paths
            .into_iter()
            .map(|p| CryptoHandler::from_key_path(&p))
            .collect::<anyhow::Result<Vec<CryptoHandler>>>()?;

        let hashmap_data = crypto_handlers
            .into_iter()
            .map(|h| {
                info(&format!("loading key with id {:X?}", &h.id));
                Ok((h.id, h))
            })
            .collect::<anyhow::Result<Vec<([u8; KEY_ID_SIZE], CryptoHandler)>>>()?;

        Ok(hashmap_data.into_iter().collect())
    }

    pub(crate) fn get_commander_unix_socket_path(&self) -> PathBuf {
        get_commander_unix_socket_path(&self.resolve_config_dir())
    }

    fn get_key_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let config_dir = self.resolve_config_dir();

        let entries: ReadDir = match fs::read_dir(&config_dir) {
            Ok(entries) => entries,
            Err(e) => return Err(anyhow!("Error reading directory {config_dir:?}: {e}")),
        };

        let key_files: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "key"))
            .collect();

        match key_files.len() {
            0 => Err(anyhow!("Could not find any .key files in {config_dir:?}")),
            _ => Ok(key_files),
        }
    }

    pub(crate) fn resolve_config_dir(&self) -> PathBuf {
        resolve_path(&self.config_dir)
    }
}

impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            commands: HashMap::new(),
            ips: vec!["127.0.0.1".parse().unwrap()],
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

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

#[cfg(test)]
mod tests {
    use crate::server::config::{
        default_config_path, default_socket_group, default_socket_user, ConfigServer,
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
                ips: vec!["127.0.0.1".parse().unwrap()],
                config_dir: default_config_path(),
                socket_user: default_socket_user(),
                socket_group: default_socket_group(),
            }
        );
    }
}
