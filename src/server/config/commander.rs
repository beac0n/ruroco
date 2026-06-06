//! Commander-only configuration: the `commands.toml` schema (`ConfigCommands`) and the commander
//! CLI (`CliCommander`). Kept separate from the server config so the network-facing server process
//! never loads the command set.

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
    use super::ConfigCommands;
    use std::collections::HashMap;
    use std::path::PathBuf;

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
