use std::error::Error;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::str;

use clap::Parser;
use log::info;
use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};

use ruroco::lib;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
    address: String,
    #[arg(short, long, default_value = lib::get_default_pem_private().into_os_string())]
    pem_path: PathBuf,
    #[arg(short, long, default_value_t = false)]
    gen: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    lib::init_logger();

    return match Cli::parse() {
        Cli {
            address: _,
            pem_path: _,
            gen,
        } if gen => lib::gen_pem(),
        Cli {
            address,
            pem_path,
            gen: _,
        } => run(pem_path, address),
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
    let now = lib::time()?;
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
