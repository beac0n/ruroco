//! This module contains all data structs that are needed for the client binary.
//! The data that these structs and enums represent are used for invoking the client binary with CLI
//! (default) arguments.

use std::env;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliClient {
    #[command(subcommand)]
    pub command: CommandsClient,
}

#[derive(Debug, Subcommand)]
pub enum CommandsClient {
    Gen {
        #[arg(short = 'r', long, default_value = env::current_dir().unwrap().join("ruroco_private.pem").into_os_string())]
        private_pem_path: PathBuf,
        #[arg(short = 'u', long, default_value = env::current_dir().unwrap().join("ruroco_public.pem").into_os_string())]
        public_pem_path: PathBuf,
        #[arg(short = 'k', long, default_value_t = 8192, value_parser = validate_key_size)]
        key_size: u32,
    },

    Send {
        #[arg(short, long)]
        address: String,
        #[arg(short, long, default_value = default_private_pem_path())]
        private_pem_path: PathBuf,
        #[arg(short, long, default_value = "default")]
        command: String,
        #[arg(short, long, default_value = "5")]
        deadline: u16,
    },
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
