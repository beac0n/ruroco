//! This module is responsible for sending data to the server and for generating PEM files

use crate::client::send::Sender;
use crate::client::update::Updater;
use crate::client::wizard::Wizard;
use crate::common::time_from_ntp;
use crate::config::config_client::{CliClient, CommandsClient};

pub mod gen;
pub mod send;
pub mod update;
mod wizard;

pub fn run_client(client: CliClient) -> Result<(), String> {
    match client.command {
        CommandsClient::Gen(gen_command) => gen::gen(
            &gen_command.private_pem_path,
            &gen_command.public_pem_path,
            gen_command.key_size,
        ),
        CommandsClient::Send(send_command) => {
            let ntp = send_command.ntp.clone();
            Sender::create(send_command, time_from_ntp(&ntp)?)?.send()
        }
        CommandsClient::Update(update_command) => {
            let updater = Updater::create(
                update_command.force,
                update_command.version,
                update_command.bin_path,
                update_command.server,
            )?;
            updater.update()
        }
        CommandsClient::Wizard(wizard_command) => {
            Wizard::create(wizard_command.force);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::gen::gen;
    use crate::client::run_client;
    use crate::config::config_client::CliClient;
    use clap::Parser;
    use rand::distr::{Alphanumeric, SampleString};
    use std::fs;
    use std::path::PathBuf;

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }

    #[test]
    fn test_gen() {
        let private_file_name = gen_file_name(".pem");
        let public_file_name = gen_file_name(".pem");

        let result = run_client(CliClient::parse_from(vec![
            "ruroco",
            "gen",
            "-r",
            &private_file_name,
            "-u",
            &public_file_name,
            "-k",
            "4096",
        ]));

        let _ = fs::remove_file(&private_file_name);
        let _ = fs::remove_file(&public_file_name);

        assert!(result.is_ok());
    }

    #[test]
    fn test_send() {
        let private_file = gen_file_name(".pem");
        let public_file = gen_file_name(".pem");

        gen(&PathBuf::from(&private_file), &PathBuf::from(&public_file), 1024).unwrap();

        let result = run_client(CliClient::parse_from(vec![
            "ruroco",
            "send",
            "-a",
            "127.0.0.1:1234",
            "-p",
            &private_file,
            "-i",
            "192.168.178.123",
        ]));

        let _ = fs::remove_file(&private_file);
        let _ = fs::remove_file(&public_file);

        assert!(result.is_ok());
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_update() {
        let result = run_client(CliClient::parse_from(vec!["ruroco", "update"]));

        assert!(result.is_ok());
    }
}
