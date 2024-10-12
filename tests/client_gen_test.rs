#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;
    use rand::distributions::{Alphanumeric, DistString};
    use ruroco::client::run_client;
    use ruroco::config_client::CliClient;
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
            gen(&private_file_name, &public_file_name).unwrap_err().to_string(),
            format!("Could not read PEM file: {private_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_bad_public_path() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name("");

        assert_eq!(
            gen(&private_file_name, &public_file_name).unwrap_err().to_string(),
            format!("Could not read PEM file: {public_file_name} does not end with .pem")
        );
    }

    #[test]
    fn test_gen_private_path_exists() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        File::create(&private_file_name).unwrap();

        let result = gen(&private_file_name, &public_file_name);

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

        let result = gen(&private_file_name, &public_file_name);
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

        let result = gen(&private_file_name, &public_file_name);
        let _ = fs::remove_file(&private_file_name);
        let _ = fs::remove_file(&public_file_name);

        assert!(result.is_ok());
    }

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("{rand_str}{suffix}")
    }

    fn gen(private_file_name: &String, public_file_name: &String) -> Result<(), String> {
        run_client(CliClient::parse_from(vec![
            "ruroco",
            "gen",
            "-r",
            PathBuf::from(&private_file_name).to_str().unwrap(),
            "-u",
            PathBuf::from(&public_file_name).to_str().unwrap(),
            "-k",
            "8192",
        ]))
    }
}
