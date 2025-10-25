use crate::common::crypto_handler::CryptoHandler;
use crate::common::info;

pub struct Generator {}

impl Generator {
    /// Create a key generator
    pub fn create() -> Result<Self, String> {
        Ok(Self {})
    }

    /// Generate a key file with the provided arguments used in create
    pub fn gen(&self) -> Result<String, String> {
        info("Generating new aes-256 key. This might take a while...");
        let key = CryptoHandler::gen_key()?;
        info(&format!("Generated new aes-256 key: {:?}", key));

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    use crate::client::gen::Generator;
    use crate::config::config_client::CliClient;

    #[test]
    fn test_send_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "gen", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_gen() {
        let key = Generator::create().unwrap().gen().unwrap();
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()), "Key is not a valid hex string");
        assert_eq!(key.len(), 80, "Key length is not 256 bits + 8 bytes");
    }
}
