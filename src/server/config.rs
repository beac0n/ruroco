//! This module contains all data structs that are needed for the server side services to work.
//! The data that these structs represent are used for invoking the server binaries with CLI
//! (default) arguments or are used to deserialize configuration files

use crate::common::blake2b_u64;
use anyhow::{anyhow, Context};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliServer {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub(crate) config: PathBuf,
}

/// The commander reads two files: the shared `config.toml` and its own `commands.toml`. Both paths
/// are configurable so the command set can be relocated independently of the server config.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliCommander {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub(crate) config: PathBuf,
    #[arg(long, default_value = PathBuf::from("/etc/ruroco/commands.toml").into_os_string())]
    pub(crate) commands: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    #[serde(deserialize_with = "deserialize_ips")]
    pub ips: Vec<IpAddr>,
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
    #[serde(default = "default_max_requests_per_second")]
    pub max_requests_per_second: u32,
    #[serde(default = "default_max_clock_skew_seconds")]
    pub max_clock_skew_seconds: u64,
}

fn deserialize_ips<'de, D>(d: D) -> Result<Vec<IpAddr>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<String> = Vec::<String>::deserialize(d)?;
    v.into_iter()
        .map(|s| {
            let ip: IpAddr = s.parse().map_err(serde::de::Error::custom)?;
            Ok(crate::common::normalize_ip(ip))
        })
        .collect()
}

impl ConfigServer {
    pub(crate) fn create_from_path(path: &Path) -> anyhow::Result<ConfigServer> {
        match fs::read_to_string(path) {
            Ok(data) => Self::deserialize(&data),
            Err(e) => Err(anyhow!("Could not read {path:?}: {e}")),
        }
    }

    pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigServer> {
        toml::from_str::<ConfigServer>(data)
            .with_context(|| format!("Could not create ConfigServer from {data}"))
    }
}

/// Commander-only configuration: the map of command name -> shell command. Kept in a separate
/// file (`commands.toml`) so the network-facing server process never loads it.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigCommands {
    pub commands: HashMap<String, String>,
}

impl ConfigCommands {
    /// Load the command set from `path` (typically `commands.toml`). This file must be installed
    /// `root`-readable only so the unprivileged server process cannot read the commands.
    pub(crate) fn create_from_path(path: &Path) -> anyhow::Result<ConfigCommands> {
        match fs::read_to_string(path) {
            Ok(data) => Self::deserialize(&data),
            Err(e) => Err(anyhow!("Could not read {path:?}: {e}")),
        }
    }

    pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigCommands> {
        toml::from_str::<ConfigCommands>(data)
            .with_context(|| format!("Could not create ConfigCommands from {data}"))
    }

    pub(crate) fn get_hash_to_cmd(&self) -> anyhow::Result<HashMap<u64, String>> {
        self.commands
            .iter()
            .map(|(k, v)| {
                let hash = blake2b_u64(k).with_context(|| format!("Could not hash {k}"))?;
                Ok((hash, v.clone()))
            })
            .collect()
    }
}

impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            ips: vec!["127.0.0.1".parse().unwrap()],
            socket_user: "".to_string(),
            socket_group: "".to_string(),
            config_dir: env::current_dir().unwrap_or(PathBuf::from("/tmp")),
            max_requests_per_second: default_max_requests_per_second(),
            max_clock_skew_seconds: default_max_clock_skew_seconds(),
        }
    }
}

fn default_socket_user() -> String {
    "ruroco".to_string()
}

fn default_socket_group() -> String {
    "ruroco".to_string()
}

fn default_max_requests_per_second() -> u32 {
    2
}

/// Upper bound, in seconds, by which an accepted counter (a nanosecond timestamp) may exceed
/// server-local `now`. Bounds how far a future-dated packet can push `last_seen`, turning a
/// permanent lockout into one recoverable by a client reseed. Only needs to cover client-vs-server
/// clock disagreement at counter seed time, not counter growth (the client counter increments by 1
/// per send, so it lags wall-clock).
fn default_max_clock_skew_seconds() -> u64 {
    3600
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

#[cfg(test)]
mod tests {
    use crate::server::config::{
        default_config_path, default_max_clock_skew_seconds, default_max_requests_per_second,
        default_socket_group, default_socket_user, ConfigCommands, ConfigServer,
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
            ConfigServer::deserialize("ips = [\"127.0.0.1\"]").unwrap(),
            ConfigServer {
                ips: vec!["127.0.0.1".parse().unwrap()],
                config_dir: default_config_path(),
                socket_user: default_socket_user(),
                socket_group: default_socket_group(),
                max_requests_per_second: default_max_requests_per_second(),
                max_clock_skew_seconds: default_max_clock_skew_seconds(),
            }
        );
    }

    #[test]
    fn test_deserialize_invalid_toml() {
        let result = ConfigServer::deserialize("this is not valid toml {{{}}}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not create ConfigServer from"));
    }

    #[test]
    fn test_deserialize_invalid_ip() {
        let result = ConfigServer::deserialize("ips = [\"not_an_ip\"]");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_hash_to_cmd() {
        let mut commands = HashMap::new();
        commands.insert("default".to_string(), "echo hello".to_string());
        commands.insert("restart".to_string(), "systemctl restart foo".to_string());
        let config = ConfigCommands { commands };
        let hash_map = config.get_hash_to_cmd().unwrap();
        assert_eq!(hash_map.len(), 2);
        assert!(hash_map.values().any(|v| v == "echo hello"));
        assert!(hash_map.values().any(|v| v == "systemctl restart foo"));
    }

    #[test]
    fn test_get_key_paths_no_key_files() {
        let dir = tempfile::tempdir().unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = config.get_key_paths();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find any .key files"));
    }

    #[test]
    fn test_get_key_paths_with_key_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.key"), "key_content").unwrap();
        std::fs::write(dir.path().join("test.txt"), "not_a_key").unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let paths = config.get_key_paths().unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].extension().unwrap() == "key");
    }

    #[test]
    fn test_create_crypto_handlers_duplicate_keys() {
        let dir = tempfile::tempdir().unwrap();
        let content = "duplicate_key_content";
        std::fs::write(dir.path().join("a.key"), content).unwrap();
        std::fs::write(dir.path().join("b.key"), content).unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let err = config.create_crypto_handlers().unwrap_err().to_string();
        assert!(err.contains("Duplicate key files detected"), "unexpected: {err}");
    }

    #[test]
    fn test_create_server_udp_socket_with_address_arg() {
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_PID");
        std::env::remove_var("RUROCO_LISTEN_ADDRESS");
        let config = ConfigServer::default();
        let port = crate::common::get_random_range(1024, 65535).unwrap();
        let socket = config.create_server_udp_socket(Some(format!("127.0.0.1:{port}"))).unwrap();
        let addr = socket.local_addr().unwrap();
        assert_eq!(addr.port(), port);
    }

    #[test]
    fn test_create_server_udp_socket_with_env_var() {
        let port = crate::common::get_random_range(1024, 65535).unwrap();
        std::env::set_var("RUROCO_LISTEN_ADDRESS", format!("127.0.0.1:{port}"));
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_PID");
        let config = ConfigServer::default();
        let socket = config.create_server_udp_socket(None).unwrap();
        let addr = socket.local_addr().unwrap();
        assert_eq!(addr.port(), port);
    }

    #[test]
    fn test_create_server_udp_socket_invalid_listen_fds() {
        std::env::set_var("LISTEN_PID", std::process::id().to_string());
        std::env::set_var("LISTEN_FDS", "2");
        std::env::remove_var("RUROCO_LISTEN_ADDRESS");
        let config = ConfigServer::default();
        let result = config.create_server_udp_socket(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("LISTEN_FDS was set to 2"));
    }

    #[test]
    fn test_create_blocklist() {
        let dir = tempfile::tempdir().unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let blocklist = config.create_blocklist().unwrap();
        assert!(blocklist.get().is_empty());
    }

    #[test]
    fn test_get_commander_unix_socket_path() {
        let config = ConfigServer {
            config_dir: PathBuf::from("/tmp/ruroco_test"),
            ..Default::default()
        };
        let path = config.get_commander_unix_socket_path();
        assert!(path.to_str().unwrap().contains("ruroco.socket"));
    }

    #[test]
    fn test_deserialize_ipv6_mapped_ip_is_normalized_to_ipv4() {
        let config = ConfigServer::deserialize("ips = [\"::ffff:127.0.0.1\"]").unwrap();
        assert_eq!(config.ips, vec!["127.0.0.1".parse::<std::net::IpAddr>().unwrap()]);
    }

    #[test]
    fn test_deserialize_commands() {
        let toml = r#"
            [commands]
            default = "echo hello"
            restart = "systemctl restart foo"
        "#;
        let config = ConfigCommands::deserialize(toml).unwrap();
        assert_eq!(config.commands.len(), 2);
        assert_eq!(config.commands.get("default").unwrap(), "echo hello");
    }

    #[test]
    fn test_create_commands_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commands.toml");
        std::fs::write(&path, "[commands]\ndefault = \"echo hi\"\n").unwrap();
        let commands = ConfigCommands::create_from_path(&path).unwrap();
        assert_eq!(commands.commands.get("default").unwrap(), "echo hi");
    }

    #[test]
    fn test_create_commands_from_path_missing() {
        let path = PathBuf::from("/tmp/path/does/not/exist/commands.toml");
        let err = ConfigCommands::create_from_path(&path).unwrap_err().to_string();
        assert!(err.contains("Could not read"), "unexpected error: {err}");
    }
}
