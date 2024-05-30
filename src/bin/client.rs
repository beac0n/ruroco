use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::str;

use clap::{Parser, Subcommand};

use ruroco::client::{gen, send};
use ruroco::common::init_logger;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Gen {
        #[arg(short = 'r', long, default_value = env::current_dir().unwrap().join("ruroco_private.pem").into_os_string())]
        private_pem_path: PathBuf,
        #[arg(short = 'u', long, default_value = env::current_dir().unwrap().join("ruroco_public.pem").into_os_string())]
        public_pem_path: PathBuf,
        #[arg(short = 'k', long, default_value_t = 8192, value_parser = validate_key_size)]
        key_size: u32,
    },

    Send {
        #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
        address: String,
        #[arg(short, long, default_value = PathBuf::from("ruroco_private.pem").into_os_string())]
        private_pem_path: PathBuf,
        #[arg(short, long, default_value = "default")]
        command: String,
    },
}

fn validate_key_size(val: &str) -> Result<u32, String> {
    let size: u32 = val.parse().map_err(|_| format!("'{}' is not a valid u32 value", val))?;
    if size >= 4096 {
        Ok(size)
    } else {
        Err(format!("Key size must be at least 4096, but '{}' was provided", size))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();

    return match Cli::parse().command {
        Commands::Gen {
            private_pem_path,
            public_pem_path,
            key_size,
        } => gen(private_pem_path, public_pem_path, key_size),
        Commands::Send {
            private_pem_path,
            address,
            command,
        } => send(private_pem_path, address, command),
    };
}
