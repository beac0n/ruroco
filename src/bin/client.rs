use std::env;
use std::error::Error;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::str;

use clap::{Parser, Subcommand};
use log::info;
use openssl::rsa::{Padding, Rsa};

use ruroco::lib::{get_path, init_logger, time};

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
        #[arg(short, long, default_value_t = 8192)]
        key_size: u32,
    },

    Send {
        #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
        address: String,
        #[arg(short, long, default_value = get_path("ruroco_private.pem").into_os_string())]
        private_pem_path: PathBuf,
        #[arg(short, long, default_value = "default")]
        command: String,
    },
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

fn send(pem_path: PathBuf, address: String, command: String) -> Result<(), Box<dyn Error>> {
    info!("Running client, connecting to udp://{address}, loading PEM from {pem_path:?} ...");
    let pem_data = fs::read(pem_path)?;
    let rsa = Rsa::private_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    let now = time()?;
    let mut now_bytes_and_command_bytes = now.to_le_bytes().to_vec();
    now_bytes_and_command_bytes.extend(command.as_bytes().to_vec());

    let mut encrypted_data = vec![0; rsa.size() as usize];
    rsa.private_encrypt(&now_bytes_and_command_bytes, &mut encrypted_data, Padding::PKCS1)?;
    socket.connect(&address)?;
    socket.send(&encrypted_data)?;
    info!("Sent command {command} and timestamp {now} to udp://{address}");
    Ok(())
}

fn gen(private: PathBuf, public: PathBuf, key_size: u32) -> Result<(), Box<dyn Error>> {
    let public_string = public.to_string_lossy().into_owned();
    let private_string = private.to_string_lossy().into_owned();

    if !private_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate private PEM file: {private_string} does not end with .pem"
        )
        .into());
    }

    if !public_string.ends_with(".pem") {
        return Err(format!(
            "Could not generate private PEM file: {public_string} does not end with .pem"
        )
        .into());
    }

    if private.exists() {
        return Err(format!(
            "Could not generate private PEM file: {private_string} already exists"
        )
        .into());
    }

    if public.exists() {
        return Err(
            format!("Could not generate public PEM file: {public_string} already exists").into()
        );
    }

    info!("Generating new rsa key with {key_size} bits and saving it to {private:?} and {public:?}. This might take a while...");
    let rsa = Rsa::generate(key_size)?;
    fs::write(&private, rsa.private_key_to_pem()?)?;
    fs::write(&public, rsa.public_key_to_pem()?)?;
    Ok(())
}
