use crate::common::crypto_handler::CryptoHandler;
use crate::common::info;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Generator {
    key_path: PathBuf,
}

impl Generator {
    /// Create a key file generator
    ///
    /// * `key_path` - Path to the key file which needs to be created
    pub fn create(key_path: &Path) -> Result<Self, String> {
        Self::validate_key_path(key_path)?;

        Ok(Self {
            key_path: key_path.to_path_buf(),
        })
    }

    /// Generate a key file with the provided arguments used in create
    pub fn gen(&self) -> Result<(), String> {
        info(&format!(
            "Generating new aes-256 key and saving it to {:?}. This might take a while...",
            self.key_path
        ));

        Self::write_key_data(&self.key_path, CryptoHandler::gen_key()?)?;

        info(&format!("Generated new aes-256 key and saved it to {:?}", self.key_path));

        Ok(())
    }

    fn write_key_data(path: &Path, data: Vec<u8>) -> Result<(), String> {
        match path.parent() {
            Some(p) => fs::create_dir_all(p)
                .map_err(|e| format!("Could not create directory ({e}) {p:?}"))?,
            None => Err(format!("Could not get parent directory of {path:?}"))?,
        }

        fs::write(path, data).map_err(|e| format!("Could not write key to {path:?}: {e}"))?;
        Ok(())
    }

    fn validate_key_path(path: &Path) -> Result<(), String> {
        match path.to_str() {
            Some(s) if s.ends_with(".key") && !path.exists() => Ok(()),
            Some(s) if path.exists() => {
                Err(format!("Could not create key file: {s} already exists"))
            }
            Some(s) => Err(format!("Could not create key file: {s} does not end with .key")),
            None => Err(format!("Could not convert key path {path:?} to string")),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};

    use crate::client::gen::Generator;
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
    fn test_gen_bad_key_path() {
        let key_file_name = gen_file_name(".xyz");

        assert_eq!(
            gen(&key_file_name).unwrap_err().to_string(),
            format!("Could not create key file: {key_file_name} does not end with .key")
        );
    }

    #[test]
    fn test_gen_key_path_exists() {
        let key_file_name = gen_file_name(".key");
        File::create(&key_file_name).unwrap();
        let result = gen(&key_file_name);
        let _ = fs::remove_file(&key_file_name);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not create key file: {key_file_name} already exists")
        );
    }

    #[test]
    fn test_gen() {
        let key_file_name = gen_file_name(".key");
        let result = gen(&key_file_name);
        let _ = fs::remove_file(&key_file_name);

        assert!(result.is_ok());
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }

    fn gen(key_file_name: &String) -> Result<(), String> {
        Generator::create(&PathBuf::from(&key_file_name))?.gen()
    }
}
