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
    use base64::engine::general_purpose;
    use base64::Engine;
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
        let key_decoded = general_purpose::STANDARD.decode(key).unwrap();
        assert_eq!(key_decoded.len(), 40, "Key length is not 256 bits + 8 bytes");
    }
}
