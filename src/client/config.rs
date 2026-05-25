//! This module contains all data structs that are needed for the client binary.
//! The data that these structs and enums represent are used for invoking the client binary with CLI
//! (default) arguments.

pub use crate::client::config_commands::SendCommand;
pub(crate) use crate::client::config_commands::{GenCommand, UpdateCommand, WizardCommand};

#[cfg(not(any(target_os = "linux", target_os = "android")))]
use anyhow::anyhow;
#[cfg(target_os = "linux")]
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub(crate) const DEFAULT_COMMAND: &str = "default";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliClient {
    #[command(subcommand)]
    pub(crate) command: CommandsClient,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CommandsClient {
    /// Generate a shared AES key (base64 with embedded key id).
    Gen(GenCommand),
    /// Send a command to a specific address.
    Send(SendCommand),
    /// Update the client binary
    Update(UpdateCommand),
    /// Run the wizard to set up the server side.
    Wizard(WizardCommand),
}

pub(crate) fn get_conf_dir() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "linux")]
    return get_conf_dir_linux();

    #[cfg(target_os = "android")]
    return get_conf_dir_android();

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    Err(anyhow!("unsupported platform"))
}

#[cfg(target_os = "linux")]
fn get_conf_dir_linux() -> anyhow::Result<PathBuf> {
    use std::env;
    use std::fs;

    let path = if let Ok(p) = env::var("RUROCO_CONF_DIR") {
        PathBuf::from(p)
    } else if let Ok(home_dir) = env::var("HOME") {
        PathBuf::from(home_dir).join(".config").join("ruroco")
    } else {
        env::current_dir().with_context(|| "Could not determine config dir")?
    };

    fs::create_dir_all(&path).with_context(|| "Could not create config dir")?;
    Ok(path)
}

#[cfg(target_os = "android")]
fn get_conf_dir_android() -> anyhow::Result<PathBuf> {
    use crate::common::android::AndroidUtil;
    AndroidUtil::create()?.get_conf_dir()
}

#[cfg(test)]
mod tests {
    use crate::client::config::{get_conf_dir, CliClient, SendCommand, DEFAULT_COMMAND};
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    #[test]
    fn test_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_get_conf_dir_with_env_var() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        let result = get_conf_dir().unwrap();
        assert_eq!(result, dir.path());
        std::env::remove_var("RUROCO_CONF_DIR");
    }

    #[test]
    fn test_get_conf_dir_with_home() {
        std::env::remove_var("RUROCO_CONF_DIR");
        let result = get_conf_dir().unwrap();
        // Should fall back to $HOME/.config/ruroco
        assert!(result.to_str().unwrap().contains("ruroco"));
    }

    #[test]
    fn test_send_command_default() {
        let cmd = SendCommand::default();
        assert_eq!(cmd.command, DEFAULT_COMMAND);
        assert_eq!(cmd.address, "127.0.0.1:1234");
        assert!(!cmd.permissive);
        assert!(!cmd.ipv4);
        assert!(!cmd.ipv6);
        assert!(cmd.ip.is_none());
    }
}
