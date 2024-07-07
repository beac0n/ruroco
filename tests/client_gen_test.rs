use std::fs;
use std::path::PathBuf;

use ruroco::client::gen;

#[cfg(test)]
mod tests {
    use std::fs::File;

    use rand::distributions::{Alphanumeric, DistString};

    use super::*;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        return format!("{rand_str}{suffix}");
    }

    #[test]
    fn test_gen_bad_private_path() {
        let private_file_name = gen_file_name("");
        let public_file_name = gen_file_name(".pem");

        let private_path = PathBuf::from(&private_file_name);
        let public_path = PathBuf::from(&public_file_name);

        let result = gen(private_path, public_path, 8192);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not read PEM file: {private_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_bad_public_path() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name("");

        let private_path = PathBuf::from(&private_file_name);
        let public_path = PathBuf::from(&public_file_name);

        let result = gen(private_path, public_path, 8192);

        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Could not read PEM file: {public_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_private_path_exists() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        File::create(&private_file_name).unwrap();

        let private_path = PathBuf::from(&private_file_name);
        let public_path = PathBuf::from(&public_file_name);

        let result = gen(private_path, public_path, 8192);

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

        let private_path = PathBuf::from(&private_file_name);
        let public_path = PathBuf::from(&public_file_name);
        let result = gen(private_path, public_path, 8192);

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

        let private_path = PathBuf::from(&private_file_name);
        let public_path = PathBuf::from(&public_file_name);
        let result = gen(private_path, public_path, 1024);

        let _ = fs::remove_file(&private_file_name);
        let _ = fs::remove_file(&public_file_name);

        assert!(result.is_ok());
    }
}
