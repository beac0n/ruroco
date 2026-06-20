//! Commander configuration, in two parts:
//!
//! - `ConfigCommander`: the commander's view of the shared `config.toml`. It reads only what it
//!   needs (`config_dir`, plus `socket_user`/`socket_group` for the Unix socket it owns) and ignores
//!   the server-only fields (`ips`, rate limit, clock skew). `config_dir` is the one value shared
//!   with the server (`ConfigServer`): both must agree so they resolve the same `ruroco.socket`.
//! - `ConfigCommands`: the `commands.toml` schema. Kept in a separate file so the network-facing
//!   server process never loads the command set; installed `root`-owned `0600` and relocatable via
//!   `--commands` independently of `config.toml`.

use crate::common::blake2b_u64;
use anyhow::{anyhow, Context};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// The commander's view of `config.toml`. Holds only the fields the commander uses; server-only
/// fields present in the same file are ignored by serde.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigCommander {
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    /// Directory holding the Unix socket (`ruroco.socket`) the commander binds. When unset it
    /// defaults to `config_dir`. Point it at a systemd `RuntimeDirectory` (e.g. `/run/ruroco`)
    /// shared with the server; both sides MUST resolve the same path. See
    /// `systemd/ruroco-commander.service`.
    #[serde(default)]
    pub socket_dir: Option<PathBuf>,
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
    /// Allow non-routable client IPs (loopback, private, link-local, etc.) to reach the executed
    /// command. Off by default: the `$RUROCO_IP` a command sees is meant to be an outside peer, so
    /// a client must not be able to name `127.0.0.1` or an internal address. Mainly for local
    /// testing where the only available source address is loopback.
    #[serde(default)]
    pub allow_non_routable_ips: bool,
}

impl ConfigCommander {
    pub(crate) fn create_from_path(path: &Path) -> anyhow::Result<ConfigCommander> {
        match fs::read_to_string(path) {
            Ok(data) => Self::deserialize(&data),
            Err(e) => Err(anyhow!("Could not read {path:?}: {e}")),
        }
    }

    pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigCommander> {
        toml::from_str::<ConfigCommander>(data)
            .with_context(|| format!("Could not create ConfigCommander from {data}"))
    }
}

impl Default for ConfigCommander {
    fn default() -> ConfigCommander {
        ConfigCommander {
            config_dir: std::env::current_dir().unwrap_or(PathBuf::from("/tmp")),
            socket_dir: None,
            socket_user: "".to_string(),
            socket_group: "".to_string(),
            allow_non_routable_ips: false,
        }
    }
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

fn default_socket_user() -> String {
    "ruroco".to_string()
}

fn default_socket_group() -> String {
    "ruroco".to_string()
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

#[cfg(test)]
mod tests {
    use super::{ConfigCommander, ConfigCommands};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_config_commander_reads_shared_fields_and_ignores_server_fields() {
        // A real config.toml carries server-only fields too; the commander view ignores them.
        let toml = r#"
            ips = ["127.0.0.1"]
            config_dir = "/etc/ruroco"
            socket_user = "ruroco"
            socket_group = "ruroco"
            max_requests_per_second = 5
        "#;
        let config = ConfigCommander::deserialize(toml).unwrap();
        assert_eq!(config.config_dir, PathBuf::from("/etc/ruroco"));
        assert_eq!(config.socket_user, "ruroco");
        assert_eq!(config.socket_group, "ruroco");
    }

    #[test]
    fn test_config_commander_defaults() {
        let config = ConfigCommander::deserialize("ips = [\"127.0.0.1\"]").unwrap();
        assert_eq!(config.config_dir, PathBuf::from("/etc/ruroco"));
        assert_eq!(config.socket_dir, None);
        assert_eq!(config.socket_user, "ruroco");
        assert_eq!(config.socket_group, "ruroco");
    }

    #[test]
    fn test_config_commander_reads_socket_dir() {
        let config =
            ConfigCommander::deserialize("ips = [\"127.0.0.1\"]\nsocket_dir = \"/run/ruroco\"")
                .unwrap();
        assert_eq!(config.socket_dir, Some(PathBuf::from("/run/ruroco")));
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
