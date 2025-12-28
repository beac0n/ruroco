//! This module contains all data structs that are needed for the client binary.
//! The data that these structs and enums represent are used for invoking the client binary with CLI
//! (default) arguments.

#[cfg(not(any(target_os = "linux", target_os = "android")))]
use anyhow::anyhow;
#[cfg(target_os = "linux")]
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub(crate) const DEFAULT_COMMAND: &str = "default";

#[derive(Parser, Debug)]
pub(crate) struct GenCommand {}

#[derive(Parser, Debug)]
pub struct SendCommand {
    /// Address to send the command to.
    #[arg(short, long)]
    pub address: String,
    /// Base64 key with id (output of `ruroco-client gen` or the UI)
    #[arg(short, long)]
    pub key: String,
    /// Command to send
    #[arg(short, long, default_value = DEFAULT_COMMAND)]
    pub command: String,
    #[arg(short = 'e', long)]
    /// Allow permissive IP validation - source IP does not have to match provided IP.
    pub permissive: bool,
    /// Optional IP address from which the command was sent.
    /// Use -6ei "dead:beef:dead:beef::/64" to allow you whole current IPv6 network.
    /// To do this automatically, use -6ei $(curl -s6 https://api64.ipify.org | awk -F: '{print $1":"$2":"$3":"$4"::/64"}')
    #[arg(short, long)]
    pub ip: Option<String>,
    /// Connect via IPv4
    #[arg(short = '4', long)]
    pub ipv4: bool,
    /// Connect via IPv6
    #[arg(short = '6', long)]
    pub ipv6: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct UpdateCommand {
    /// Force update
    #[arg(short, long)]
    pub(crate) force: bool,
    /// Version
    #[arg(short, long)]
    pub(crate) version: Option<String>,
    /// Path where binaries are saved
    #[arg(short, long)]
    pub(crate) bin_path: Option<PathBuf>,
    /// Update for server side
    #[arg(short, long)]
    pub(crate) server: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct WizardCommand {
    #[arg(short, long)]
    pub(crate) force: bool,
}

impl Default for SendCommand {
    fn default() -> SendCommand {
        SendCommand {
            address: "127.0.0.1:1234".to_string(),
            key: "FFFFFFFFFFFFFFFFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF"
                .to_string(),
            command: DEFAULT_COMMAND.to_string(),
            permissive: false,
            ip: None,
            ipv4: false,
            ipv6: false,
        }
    }
}

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
    use crate::common::android_util::AndroidUtil;
    AndroidUtil::create()?.get_conf_dir()
}

#[cfg(test)]
mod tests {
    use crate::client::config::CliClient;
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    #[test]
    fn test_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }
}
