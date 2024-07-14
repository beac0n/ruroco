use std::env;
use std::path::PathBuf;
use std::str;

use clap::{Parser, Subcommand};

use ruroco::client::{gen, send};
use ruroco::common::{init_logger, time};

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
            let mut private_pem_path = PathBuf::from(home_dir);
            private_pem_path.push(".config");
            private_pem_path.push("ruroco");
            private_pem_path.push(private_pem_name);
            private_pem_path
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

fn main() -> Result<(), String> {
    init_logger();

    match Cli::parse().command {
        Commands::Gen {
            private_pem_path,
            public_pem_path,
            key_size,
        } => gen(private_pem_path, public_pem_path, key_size),
        Commands::Send {
            private_pem_path,
            address,
            command,
            deadline,
        } => send(private_pem_path, address, command, deadline, time()?),
    }
}
