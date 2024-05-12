use std::{env, fs};
use std::error::Error;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::str;
use std::time::SystemTime;

use clap::Parser;
use log::{debug, error, info};
use openssl::pkey::{Private, Public};
use openssl::rsa::{Padding, Rsa};
use regex::Regex;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
    address: String,
    #[arg(short = 'v', long, default_value = get_default_pem_private().into_os_string())]
    pem_path_private: PathBuf,
    #[arg(short, long, default_value = get_default_pem_public().into_os_string())]
    pem_path_public: PathBuf,
    #[arg(short, long, default_value_t = false)]
    gen: bool,
    #[arg(short, long, default_value_t = false)]
    server: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    return match Cli::parse() {
        args if args.gen => gen_pem(),
        args if args.server && is_addr(&args.address) => {
            run_server(args.pem_path_public, args.address)
        }
        args if !args.server && is_addr(&args.address) => {
            run_client(args.pem_path_private, args.address)
        }
        _ => Err("Invalid arguments combination".into()),
    };
}

fn is_addr(addr: &str) -> bool {
    let regex = r"^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}:[1-9][0-9]*$";
    return Regex::new(regex).unwrap().is_match(addr);
}

fn gen_pem() -> Result<(), Box<dyn Error>> {
    let key_size = 8192;
    return match (get_default_pem_private(), get_default_pem_public()) {
        (pem_path_private, pem_path_public)
            if pem_path_private.exists() || pem_path_public.exists() =>
        {
            let msg = format!(
                "Could not generate new rsa key with {key_size} bits, because {pem_path_private:?} or {pem_path_public:?} already exists"
            );
            Err(msg.into())
        }
        (pem_path_private, pem_path_public) => {
            debug!(
                "Generating new rsa key with {key_size} bits and saving it to {pem_path_private:?} and {pem_path_public:?}. This might take a while...",
            );
            let rsa = Rsa::generate(key_size)?;
            fs::write(&pem_path_private, rsa.private_key_to_pem()?)?;
            fs::write(&pem_path_public, rsa.public_key_to_pem()?)?;
            Ok(())
        }
    };
}

fn get_default_pem_public() -> PathBuf {
    get_default_pem_path("ruroco_public.pem")
}

fn get_default_pem_private() -> PathBuf {
    get_default_pem_path("ruroco_private.pem")
}

fn get_default_pem_path(pem_name: &str) -> PathBuf {
    return match env::current_dir() {
        Ok(dir) => dir.join(pem_name),
        Err(_) => PathBuf::from(pem_name),
    };
}

fn run_client(pem_path: PathBuf, address: String) -> Result<(), Box<dyn Error>> {
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

fn time() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}

fn run_server(pem_path: PathBuf, address: String) -> Result<(), Box<dyn Error>> {
    info!("Starting server on udp://{address}, loading PEM from {} ...", pem_path.display());
    let pem_data = fs::read(pem_path)?;
    let rsa: Rsa<Public> = Rsa::public_key_from_pem(&pem_data)?;
    let socket = UdpSocket::bind(&address)?;

    loop {
        run_server_iteration(&rsa, &address, &socket);
    }
}

fn run_server_iteration(rsa: &Rsa<Public>, address: &str, socket: &UdpSocket) {
    let expected_read_count = 1024;
    // make sure encrypted_data size == expected_read_count
    let mut encrypted_data = [0; 1024];
    return match socket.recv_from(&mut encrypted_data) {
        Ok((read_count, src)) if read_count < expected_read_count => {
            error!("Invalid read count {read_count}, expected {expected_read_count} - from {src}")
        }
        Ok(_) => validate_data(rsa, &encrypted_data),
        Err(_) => error!("Could not recv_from socket from udp://{address}"),
    };
}

fn validate_data(rsa: &Rsa<Public>, encrypted_data: &[u8; 1024]) {
    let mut decrypted_data = vec![0; rsa.size() as usize];
    return match rsa.public_decrypt(encrypted_data, &mut decrypted_data, Padding::PKCS1) {
        Ok(count) => validate_decrypted_data(&mut decrypted_data, count),
        Err(e) => error!("Could not public_decrypt {encrypted_data:X?}: {e}"),
    };
}

fn validate_decrypted_data(decrypted_data: &mut Vec<u8>, count: usize) {
    decrypted_data.truncate(count);
    let timestamp = vec_u8_to_u64(&decrypted_data);
    return match time() {
        Ok(now) if timestamp > now => error!("Invalid content {timestamp} is newer than now {now}"),
        Ok(now) if timestamp < now - 5 => {
            error!("Invalid content {timestamp} is older than now {now} - 5 = {}", now - 5)
        }
        Ok(_) => {
            // TODO: execute command executor
            info!("Successfully validated data - {timestamp} is not too old/new")
        }
        Err(e) => error!("Could not get current time: {e}"),
    };
}

fn vec_u8_to_u64(decrypted_data: &Vec<u8>) -> u64 {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&decrypted_data);
    u64::from_le_bytes(buffer)
}
