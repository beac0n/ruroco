use std::{env, fs, str};
use std::error::Error;
use std::path::PathBuf;
use std::time::SystemTime;

use log::debug;
use openssl::rsa::Rsa;

pub fn time() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}

pub fn gen_pem() -> Result<(), Box<dyn Error>> {
    let key_size = 8192;

    return match (get_default_pem_private(), get_default_pem_public()) {
        (private, public) if private.exists() || public.exists() => {
            let msg = format!(
                "Could not generate new rsa key with {key_size} bits, because {private:?} or {public:?} already exists"
            );
            Err(msg.into())
        }
        (private, public) => {
            debug!(
                "Generating new rsa key with {key_size} bits and saving it to {private:?} and {public:?}. This might take a while...",
            );
            let rsa = Rsa::generate(key_size)?;
            fs::write(&private, rsa.private_key_to_pem()?)?;
            fs::write(&public, rsa.public_key_to_pem()?)?;
            Ok(())
        }
    };
}

pub fn get_default_pem_public() -> PathBuf {
    get_default_pem_path("ruroco_public.pem")
}

pub fn get_default_pem_private() -> PathBuf {
    get_default_pem_path("ruroco_private.pem")
}

fn get_default_pem_path(pem_name: &str) -> PathBuf {
    return match env::current_dir() {
        Ok(dir) => dir.join(pem_name),
        Err(_) => PathBuf::from(pem_name),
    };
}
