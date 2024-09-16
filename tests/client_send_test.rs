use std::fs;
use std::path::PathBuf;

use ruroco::client::send;

#[cfg(test)]
mod tests {
    use std::fs::File;

    use rand::distributions::{Alphanumeric, DistString};

    use super::*;
    use ruroco::client::gen;
    use ruroco::common::time;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }

    #[test]
    fn test_send_no_such_file() {
        let pem_file_name = gen_file_name(".pem");
        let pem_path = PathBuf::from(&pem_file_name);
        let result = send(
            pem_path,
            String::from("127.0.0.1:1234"),
            String::from("default"),
            5,
            true,
            Some(String::from("192.168.178.123")),
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

        let pem_path = PathBuf::from(&pem_file_name);
        let result = send(
            pem_path,
            String::from("127.0.0.1:1234"),
            String::from("default"),
            5,
            true,
            Some(String::from("192.168.178.123")),
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
        gen(private_pem_path.clone(), public_pem_path, 1024).unwrap();

        let address = String::from("127.0.0.1:asd");
        let result = send(
            private_pem_path,
            address.clone(),
            String::from("default"),
            5,
            true,
            Some(String::from("192.168.178.123")),
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
        gen(private_pem_path.clone(), public_pem_path, 1024).unwrap();

        let address = String::from("999.999.999.999:9999");
        let result = send(
            private_pem_path,
            address.clone(),
            String::from("default"),
            5,
            true,
            Some(String::from("192.168.178.123")),
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
        gen(private_pem_path.clone(), public_pem_path, 1024).unwrap();

        let result = send(
            private_pem_path,
            String::from("127.0.0.1:1234"),
            "#".repeat(66),
            5,
            true,
            Some(String::from("192.168.178.123")),
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert_eq!(
            result.unwrap_err().to_string(),
            String::from(
                "Too much data, must be at most 117 bytes, but was 132 bytes. \
                Reduce command name length or create a bigger RSA key size."
            )
        );
    }

    #[test]
    fn test_send() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        let private_pem_path = PathBuf::from(&private_file);
        let public_pem_path = PathBuf::from(&public_file);
        gen(private_pem_path.clone(), public_pem_path, 1024).unwrap();

        let result = send(
            private_pem_path,
            String::from("127.0.0.1:1234"),
            String::from("default"),
            5,
            true,
            Some(String::from("192.168.178.123")),
            time().unwrap(),
        );

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert!(result.is_ok());
    }
}
