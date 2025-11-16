use crate::common::crypto_handler::CryptoHandler;

pub struct Generator {}

impl Generator {
    /// Create a key generator
    pub fn create() -> Result<Self, String> {
        Ok(Self {})
    }

    /// Generate a key file with the provided arguments used in create
    pub fn gen(&self) -> Result<String, String> {
        let key = CryptoHandler::gen_key()?;
        print!("{}", key);
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    use crate::client::config::CliClient;
    use crate::client::gen::Generator;

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
