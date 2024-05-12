use std::path::PathBuf;
use std::error::Error;
use log::info;
use std::fs;
use openssl::rsa::{Padding, Rsa};
use openssl::pkey::Private;
use std::net::UdpSocket;
use crate::util;

pub fn run(pem_path: PathBuf, address: String) -> Result<(), Box<dyn Error>> {
    info!(
        "Running client, connecting to udp://{address}, loading PEM from {} ...",
        pem_path.display()
    );
    let pem_data = fs::read(pem_path)?;
    let rsa: Rsa<Private> = Rsa::private_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let now = util::time()?;
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
