//! This module contains all data structs that are needed for the client binary.
//! The data that these structs and enums represent are used for invoking the client binary with CLI
//! (default) arguments.

use std::env;
use std::path::PathBuf;

use crate::common::NTP_SYSTEM;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct GenCommand {
    /// Path to the private PEM file
    #[arg(short = 'r', long, default_value = env::current_dir().unwrap().join("ruroco_private.pem").into_os_string())]
    pub private_pem_path: PathBuf,
    /// Path to the public PEM file
    #[arg(short = 'u', long, default_value = env::current_dir().unwrap().join("ruroco_public.pem").into_os_string())]
    pub public_pem_path: PathBuf,
    /// Key size for the PEM file
    #[arg(short = 'k', long, default_value_t = 8192, value_parser = validate_key_size)]
    pub key_size: u32,
}

#[derive(Parser, Debug)]
pub struct SendCommand {
    /// Address to send the command to.
    #[arg(short, long)]
    pub address: String,
    /// Path to the private PEM file.
    #[arg(short, long, default_value = default_private_pem_path())]
    pub private_pem_path: PathBuf,
    /// Command to send
    #[arg(short, long, default_value = "default")]
    pub command: String,
    /// Deadline from now in seconds
    #[arg(short, long, default_value = "5")]
    pub deadline: u16,
    #[arg(short, long, default_value_t = true)]
    /// Whether to enforce strict mode for source ip validation (defaults to true).
    pub strict: bool,
    /// Optional IP address from which the command was sent.
    #[arg(short, long)]
    pub ip: Option<String>,
    /// NTP server (defaults to using the system time).
    #[arg(short, long, default_value = NTP_SYSTEM)]
    pub ntp: String,
    /// Connect via IPv4 (defaults to true)
    #[arg(short = '4', long, default_value_t = true)]
    pub ipv4: bool,
}

impl Default for SendCommand {
    fn default() -> SendCommand {
        SendCommand {
            address: String::from("127.0.0.1:1234"),
            private_pem_path: PathBuf::from(default_private_pem_path()),
            command: String::from("default"),
            deadline: 5,
            strict: true,
            ip: None,
            ntp: String::from("system"),
            ipv4: false,
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
}

fn default_private_pem_path() -> std::ffi::OsString {
    let private_pem_name = "ruroco_private.pem";
    let private_pem_path = match env::var("HOME") {
        Ok(home_dir) => {
            PathBuf::from(home_dir).join(".config").join("ruroco").join(private_pem_name)
        }
        Err(_) => PathBuf::from(private_pem_name),
    };

    private_pem_path.into_os_string()
}

fn validate_key_size(key_str: &str) -> Result<u32, String> {
    let min_key_size = 4096;
    match key_str.parse() {
        Ok(size) if size >= min_key_size => Ok(size),
        Ok(size) => {
            Err(format!("Key size must be at least {min_key_size}, but {size} was provided"))
        }
        Err(e) => Err(format!("Could not parse {key_str} to u32: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use crate::config_client::{default_private_pem_path, validate_key_size};
    use std::env;
    use std::path::PathBuf;

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
            default_private_pem_path(),
            PathBuf::from(env::var("HOME").unwrap()).join(".config/ruroco/ruroco_private.pem")
        );
    }
}
