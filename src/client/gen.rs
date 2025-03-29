use crate::common::info;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use std::fs;
use std::path::Path;

/// Generate a public and private PEM file with the provided key_size
///
/// * `private_path` - Path to the private PEM file which needs to be created
/// * `public_path` - Path to the public PEM file which needs to be created
/// * `key_size` - key size
pub fn gen(private_path: &Path, public_path: &Path, key_size: u32) -> Result<(), String> {
    validate_pem_path(public_path)?;
    validate_pem_path(private_path)?;

    info(&format!("Generating new rsa key with {key_size} bits and saving it to {private_path:?} and {public_path:?}. This might take a while..."));
    let rsa = Rsa::generate(key_size)
        .map_err(|e| format!("Could not generate rsa for key size {key_size}: {e}"))?;

    let private_key_pem = get_pem_data(&rsa, "private")?;
    let public_key_pem = get_pem_data(&rsa, "public")?;

    write_pem_data(private_path, private_key_pem, "private")?;
    write_pem_data(public_path, public_key_pem, "public")?;

    info(&format!("Generated new rsa key with {key_size} bits and saved it to {private_path:?} and {public_path:?}"));

    Ok(())
}

fn get_pem_data(rsa: &Rsa<Private>, name: &str) -> Result<Vec<u8>, String> {
    let data = match name {
        "public" => rsa.public_key_to_pem(),
        "private" => rsa.private_key_to_pem(),
        _ => return Err(format!("Invalid pem data name {name}")),
    };

    data.map_err(|e| format!("Could not create {name} key pem: {e}"))
}

fn write_pem_data(path: &Path, data: Vec<u8>, name: &str) -> Result<(), String> {
    match path.parent() {
        Some(p) => {
            fs::create_dir_all(p).map_err(|e| format!("Could not create directory ({e}) {p:?}"))?
        }
        None => Err(format!("Could not get parent directory of {path:?}"))?,
    }

    fs::write(path, data).map_err(|e| format!("Could not write {name} key to {path:?}: {e}"))?;
    Ok(())
}

fn validate_pem_path(path: &Path) -> Result<(), String> {
    match path.to_str() {
        Some(s) if s.ends_with(".pem") && !path.exists() => Ok(()),
        Some(s) if path.exists() => Err(format!("Could not create PEM file: {s} already exists")),
        Some(s) => Err(format!("Could not read PEM file: {s} does not end with .pem")),
        None => Err(format!("Could not convert PEM path {path:?} to string")),
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};

    use crate::client::gen::gen;
    use crate::config::config_client::CliClient;
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_send_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "gen", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_gen_bad_private_path() {
        let private_file_name = gen_file_name("");
        let public_file_name = gen_file_name(".pem");

        assert_eq!(
            gen_8192(&private_file_name, &public_file_name).unwrap_err().to_string(),
            format!("Could not read PEM file: {private_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_bad_public_path() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name("");

        assert_eq!(
            gen_8192(&private_file_name, &public_file_name).unwrap_err().to_string(),
            format!("Could not read PEM file: {public_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_private_path_exists() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        File::create(&private_file_name).unwrap();

        let result = gen_8192(&private_file_name, &public_file_name);

        let _ = fs::remove_file(&private_file_name);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not create PEM file: {private_file_name} already exists")
        );
    }

    #[test]
    fn test_gen_public_path_exists() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        File::create(&public_file_name).unwrap();

        let result = gen_8192(&private_file_name, &public_file_name);
        let _ = fs::remove_file(&public_file_name);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not create PEM file: {public_file_name} already exists")
        );
    }

    #[test]
    fn test_gen() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        let result = gen_8192(&private_file_name, &public_file_name);
        let _ = fs::remove_file(&private_file_name);
        let _ = fs::remove_file(&public_file_name);

        assert!(result.is_ok());
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }

    fn gen_8192(private_file_name: &String, public_file_name: &String) -> Result<(), String> {
        gen(&PathBuf::from(&private_file_name), &PathBuf::from(&public_file_name), 8192)
    }
}
