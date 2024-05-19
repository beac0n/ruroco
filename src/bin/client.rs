use std::env;
use std::error::Error;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::str;

use clap::{Parser, Subcommand};
use log::info;
use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};

use ruroco::lib::{get_path, init_logger, PEM_DIR_ERR_PREFIX, time};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Gen {
        #[arg(short, long, default_value = env::current_dir().unwrap().into_os_string())]
        pem_dir: PathBuf,
        #[arg(short, long, default_value_t = 8192)]
        key_size: u32,
    },

    Send {
        #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
        address: String,
        #[arg(short, long, default_value = get_path("ruroco_private.pem").into_os_string())]
        pem_path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();

    return match Cli::parse().command {
        Commands::Gen { pem_dir, key_size } => gen_pem(pem_dir, key_size),
        Commands::Send { pem_path, address } => run(pem_path, address),
    };
}

fn run(pem_path: PathBuf, address: String) -> Result<(), Box<dyn Error>> {
    info!(
        "Running client, connecting to udp://{address}, loading PEM from {} ...",
        pem_path.display()
    );
    let pem_data = fs::read(pem_path)?;
    let rsa: Rsa<Private> = Rsa::private_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let now = time()?;
    let now_bytes = now.to_le_bytes().to_vec();

    let mut encrypted_data = vec![0; rsa.size() as usize];
    return match rsa.private_encrypt(&now_bytes, &mut encrypted_data, Padding::PKCS1) {
        Ok(_) => {
            socket.connect(&address)?;
            socket.send(&encrypted_data)?;
            info!("Successfully encrypted {now_bytes:X?}, {now} and sent to udp://{address}");
            Ok(())
        }
        Err(e) => Err(format!("Could not private_encrypt {encrypted_data:X?}: {e}").into()),
    };
}

fn gen_pem(pem_dir: PathBuf, key_size: u32) -> Result<(), Box<dyn Error>> {
    let pem_dir_display = pem_dir.display();

    if !pem_dir.is_dir() {
        return Err(format!("{PEM_DIR_ERR_PREFIX} {pem_dir_display} is not a directory").into());
    }

    if !pem_dir.exists() {
        return Err(format!("{PEM_DIR_ERR_PREFIX} {pem_dir_display} does not exist").into());
    }

    let private = pem_dir.join("ruroco_private.pem");
    let public = pem_dir.join("ruroco_public.pem");

    if private.exists() || public.exists() {
        return Err(format!(
            "Could not generate new rsa key with {key_size} bits, because {private:?} or {public:?} already exists"
        ).into());
    }

    info!("Generating new rsa key with {key_size} bits and saving it to {private:?} and {public:?}. This might take a while...");
    let rsa = Rsa::generate(key_size)?;
    fs::write(&private, rsa.private_key_to_pem()?)?;
    fs::write(&public, rsa.public_key_to_pem()?)?;
    Ok(())
}
