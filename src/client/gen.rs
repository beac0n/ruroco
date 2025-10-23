use crate::common::crypto_handler::CryptoHandler;
use crate::common::info;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Generator {}

impl Generator {
    /// Create a key generator
    pub fn create() -> Result<Self, String> {
        Ok(Self {})
    }

    /// Generate a key file with the provided arguments used in create
    pub fn gen(&self) -> Result<(), String> {
        info(&"Generating new aes-256 key. This might take a while...".to_string());
        info(&format!("Generated new aes-256 key: {:?}", CryptoHandler::gen_key()?));

        Ok(())
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
    fn test_gen() {
        let result = gen();
        assert!(result.is_ok());
    }

    fn gen() -> Result<(), String> {
        Generator::create()?.gen()
    }
}
