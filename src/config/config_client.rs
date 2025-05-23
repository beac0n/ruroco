//! This module contains all data structs that are needed for the client binary.
//! The data that these structs and enums represent are used for invoking the client binary with CLI
//! (default) arguments.

#[cfg(target_os = "linux")]
use std::env;

#[cfg(target_os = "android")]
use crate::ui::android_util::{AndroidUtil, J_FILE, J_STRING};

use crate::common::NTP_SYSTEM;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub const DEFAULT_KEY_SIZE: u16 = 8192;
pub const DEFAULT_COMMAND: &str = "default";
pub const DEFAULT_DEADLINE: u16 = 5;
pub const MIN_KEY_SIZE: u16 = 4096;

#[derive(Parser, Debug)]
pub struct GenCommand {
    /// Path to the private PEM file
    #[arg(short = 'r', long, default_value = default_private_pem_path().into_os_string())]
    pub private_pem_path: PathBuf,
    /// Path to the public PEM file
    #[arg(short = 'u', long, default_value = default_public_pem_path().into_os_string())]
    pub public_pem_path: PathBuf,
    /// Key size for the PEM file
    #[arg(short = 'k', long, default_value_t = DEFAULT_KEY_SIZE.into(), value_parser = validate_key_size
    )]
    pub key_size: u32,
}

#[derive(Parser, Debug)]
pub struct SendCommand {
    /// Address to send the command to.
    #[arg(short, long)]
    pub address: String,
    /// Path to the private PEM file.
    #[arg(short, long, default_value = default_private_pem_path().into_os_string())]
    pub private_pem_path: PathBuf,
    /// Command to send
    #[arg(short, long, default_value = DEFAULT_COMMAND)]
    pub command: String,
    /// Deadline from now in seconds
    #[arg(short, long, default_value_t = DEFAULT_DEADLINE)]
    pub deadline: u16,
    #[arg(short = 'e', long)]
    /// Allow permissive IP validation - source IP does not have to match provided IP.
    pub permissive: bool,
    /// Optional IP address from which the command was sent.
    /// Use -6ei "dead:beef:dead:beef::/64" to allow you whole current IPv6 network.
    /// To do this automatically, use -6ei $(curl -s6 https://api64.ipify.org | awk -F: '{print $1":"$2":"$3":"$4"::/64"}')
    #[arg(short, long)]
    pub ip: Option<String>,
    /// NTP server (defaults to using the system time).
    #[arg(short, long, default_value = NTP_SYSTEM)]
    pub ntp: String,
    /// Connect via IPv4
    #[arg(short = '4', long)]
    pub ipv4: bool,
    /// Connect via IPv6
    #[arg(short = '6', long)]
    pub ipv6: bool,
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// Force update
    #[arg(short, long)]
    pub force: bool,
    /// Version
    #[arg(short, long)]
    pub version: Option<String>,
    /// Path where binaries are saved
    #[arg(short, long)]
    pub bin_path: Option<PathBuf>,
    /// Update for server side
    #[arg(short, long)]
    pub server: bool,
}

#[derive(Parser, Debug)]
pub struct WizardCommand {
    #[arg(short, long)]
    pub force: bool,
}

impl Default for SendCommand {
    fn default() -> SendCommand {
        SendCommand {
            address: "127.0.0.1:1234".to_string(),
            private_pem_path: default_private_pem_path(),
            command: DEFAULT_COMMAND.to_string(),
            deadline: 5,
            permissive: false,
            ip: None,
            ntp: NTP_SYSTEM.to_string(),
            ipv4: false,
            ipv6: false,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliClient {
    #[command(subcommand)]
    pub command: CommandsClient,
}

#[derive(Debug, Subcommand)]
pub enum CommandsClient {
    /// Generate a pair of private and public PEM keys.
    Gen(GenCommand),
    /// Send a command to a specific address.
    Send(SendCommand),
    /// Update the client binary
    Update(UpdateCommand),
    /// Run the wizard to set up the server side.
    Wizard(WizardCommand),
}

pub fn default_private_pem_path() -> PathBuf {
    get_default_pem_path("ruroco_private.pem")
}

pub fn default_public_pem_path() -> PathBuf {
    get_default_pem_path("ruroco_public.pem")
}

fn get_default_pem_path(pem_name: &str) -> PathBuf {
    get_conf_dir().join(pem_name)
}

pub fn get_conf_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let current_dir = PathBuf::from(".");
        match (env::var("HOME"), env::current_dir()) {
            (Ok(home_dir), _) => PathBuf::from(home_dir).join(".config").join("ruroco"),
            (_, Ok(current_dir)) => current_dir,
            (_, _) => current_dir,
        }
    }

    #[cfg(target_os = "android")]
    {
        let util = AndroidUtil::create();
        let files_dir_obj = util.call_ctx_method("getFilesDir", J_FILE, &[]).unwrap();
        let abs_path_ref =
            util.call_method(files_dir_obj, "getAbsolutePath", J_STRING, &[]).unwrap();
        PathBuf::from(util.global_ref_to_string(abs_path_ref).unwrap())
    }
}

fn validate_key_size(key_str: &str) -> Result<u32, String> {
    match key_str.parse() {
        Ok(size) if size >= MIN_KEY_SIZE as u32 => Ok(size),
        Ok(size) => {
            Err(format!("Key size must be at least {MIN_KEY_SIZE}, but {size} was provided"))
        }
        Err(e) => Err(format!("Could not parse {key_str} to u32: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::config_client::{default_private_pem_path, validate_key_size, CliClient};
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_validate_key_size() {
        assert!(validate_key_size("invalid").is_err());
        assert!(validate_key_size("1024").is_err());
        assert!(validate_key_size("2048").is_err());
        assert!(validate_key_size("4096").is_ok());
        assert!(validate_key_size("8192").is_ok());
    }

    #[test]
    fn test_default_private_pem_path() {
        assert_eq!(
            default_private_pem_path().into_os_string(),
            PathBuf::from(env::var("HOME").unwrap()).join(".config/ruroco/ruroco_private.pem")
        );
    }
}
