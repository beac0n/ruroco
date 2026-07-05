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
use std::time::Duration;

/// Default execution timeout for a command that doesn't specify `timeout_sec`.
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 30;

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

#[cfg(any(test, feature = "testing"))]
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

/// A single entry in `commands.toml`: either the plain shell command string (default timeout), or
/// a table overriding the timeout. `#[serde(untagged)]` lets both forms live in the same map.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum CommandValue {
    Plain(String),
    Detailed {
        cmd: String,
        #[serde(default = "default_timeout_sec")]
        timeout_sec: u64,
    },
}

impl CommandValue {
    fn cmd(&self) -> &str {
        match self {
            CommandValue::Plain(cmd) => cmd,
            CommandValue::Detailed { cmd, .. } => cmd,
        }
    }

    fn timeout(&self) -> Duration {
        match self {
            CommandValue::Plain(_) => Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            CommandValue::Detailed { timeout_sec, .. } => Duration::from_secs(*timeout_sec),
        }
    }
}

fn default_timeout_sec() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

/// A resolved command: the shell command to run plus how long it may run before being killed.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct CommandSpec {
    pub(crate) cmd: String,
    pub(crate) timeout: Duration,
}

/// Commander-only configuration: the map of command name -> shell command. Kept in a separate
/// file (`commands.toml`) so the network-facing server process never loads it.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigCommands {
    pub(crate) commands: HashMap<String, CommandValue>,
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

    /// Build a `ConfigCommands` from plain name -> shell command pairs, all at the default
    /// timeout. A one-line migration path for call sites that used to build the struct literal
    /// directly, before per-command timeouts existed.
    pub fn from_map(commands: HashMap<String, String>) -> ConfigCommands {
        ConfigCommands {
            commands: commands.into_iter().map(|(k, v)| (k, CommandValue::Plain(v))).collect(),
        }
    }

    pub(crate) fn get_hash_to_cmd(&self) -> anyhow::Result<HashMap<u64, CommandSpec>> {
        self.commands
            .iter()
            .map(|(k, v)| {
                let hash = blake2b_u64(k).with_context(|| format!("Could not hash {k}"))?;
                Ok((
                    hash,
                    CommandSpec {
                        cmd: v.cmd().to_string(),
                        timeout: v.timeout(),
                    },
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigCommander, ConfigCommands, DEFAULT_TIMEOUT_SECS};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

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
        let config = ConfigCommands::from_map(commands);
        let hash_map = config.get_hash_to_cmd().unwrap();
        assert_eq!(hash_map.len(), 2);
        assert!(hash_map.values().any(|v| v.cmd == "echo hello"));
        assert!(hash_map.values().any(|v| v.cmd == "systemctl restart foo"));
        assert!(hash_map.values().all(|v| v.timeout == Duration::from_secs(DEFAULT_TIMEOUT_SECS)));
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
        assert_eq!(config.commands.get("default").unwrap().cmd(), "echo hello");
        assert_eq!(
            config.commands.get("default").unwrap().timeout(),
            Duration::from_secs(DEFAULT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn test_deserialize_commands_with_timeout_override() {
        let toml = r#"
            [commands]
            default = "echo hello"
            slow = { cmd = "sleep 5", timeout_sec = 1 }
        "#;
        let config = ConfigCommands::deserialize(toml).unwrap();
        let slow = config.commands.get("slow").unwrap();
        assert_eq!(slow.cmd(), "sleep 5");
        assert_eq!(slow.timeout(), Duration::from_secs(1));

        let hash_map = config.get_hash_to_cmd().unwrap();
        assert!(hash_map
            .values()
            .any(|v| v.cmd == "sleep 5" && v.timeout == Duration::from_secs(1)));
    }

    #[test]
    fn test_deserialize_commands_table_without_timeout_uses_default() {
        let toml = r#"
            [commands]
            default = { cmd = "echo hello" }
        "#;
        let config = ConfigCommands::deserialize(toml).unwrap();
        let entry = config.commands.get("default").unwrap();
        assert_eq!(entry.cmd(), "echo hello");
        assert_eq!(entry.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    }

    #[test]
    fn test_from_map_uses_default_timeout() {
        let mut commands = HashMap::new();
        commands.insert("default".to_string(), "echo hi".to_string());
        let config = ConfigCommands::from_map(commands);
        assert_eq!(
            config.commands.get("default").unwrap().timeout(),
            Duration::from_secs(DEFAULT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn test_create_commands_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commands.toml");
        std::fs::write(&path, "[commands]\ndefault = \"echo hi\"\n").unwrap();
        let commands = ConfigCommands::create_from_path(&path).unwrap();
        assert_eq!(commands.commands.get("default").unwrap().cmd(), "echo hi");
    }

    #[test]
    fn test_create_commands_from_path_missing() {
        let path = PathBuf::from("/tmp/path/does/not/exist/commands.toml");
        let err = ConfigCommands::create_from_path(&path).unwrap_err().to_string();
        assert!(err.contains("Could not read"), "unexpected error: {err}");
    }
}
