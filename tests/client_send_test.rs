#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use ruroco::client::gen;
    use ruroco::client::send;
    use ruroco::common::time;
    use ruroco::config_client::{CliClient, SendCommand};
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use rand::distr::{Alphanumeric, SampleString};

    const IP: &str = "192.168.178.123";

    #[test]
    fn test_send_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "send", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_send_no_such_file() {
        let pem_file_name = gen_file_name(".pem");

        let result = send(
            SendCommand {
                private_pem_path: PathBuf::from(&pem_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not load \"{pem_file_name}\": No such file or directory (os error 2)")
        );
    }

    #[test]
    fn test_send_invalid_pem() {
        let pem_file_name = gen_file_name(".pem");
        File::create(&pem_file_name).unwrap();

        let result = send(
            SendCommand {
                private_pem_path: PathBuf::from(&pem_file_name),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&pem_file_name);

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No supported data to decode. Input type: PEM"));
    }

    #[test]
    fn test_send_invalid_port_value() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let address = "127.0.0.1:asd".to_string();
        let result = send(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not resolve hostname for {address}: invalid port value")
        );
    }

    #[test]
    fn test_send_unknown_service() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let address = "999.999.999.999:9999".to_string();
        let result = send(
            SendCommand {
                address: address.clone(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!(
                "Could not resolve hostname for {address}: \
                failed to lookup address information: Name or service not known"
            )
        );
    }

    #[test]
    fn test_send_command_too_long() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024).unwrap();

        let result = send(
            SendCommand {
                private_pem_path,
                command: "#".repeat(66),
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            "Too much data, must be at most 117 bytes, but was 132 bytes. \
                Reduce command name length or create a bigger RSA key size."
                .to_string()
        );
    }

    #[test]
    fn test_send_ipv4() {
        assert!(send_test("127.0.0.1:1234").is_ok());
    }

    #[test]
    fn test_send_ipv6() {
        assert!(send_test("::1:1234").is_ok());
    }

    fn send_test(address: &str) -> Result<(), String> {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(&private_pem_path, &public_pem_path, 1024)?;

        let result = send(
            SendCommand {
                address: address.to_string(),
                private_pem_path,
                ip: Some(IP.to_string()),
                ..Default::default()
            },
            time()?,
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        result
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }
}
