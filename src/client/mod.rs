//! This module is responsible for sending data to the server and for generating key file

use crate::client::config::{CliClient, CommandsClient};
use crate::client::gen::Generator;
use crate::client::send::Sender;
use crate::client::update::Updater;
use crate::client::wizard::Wizard;

/// data structures for using CLI arguments for the client binary
pub mod config;
pub mod counter;
pub mod gen;
pub mod send;
pub(crate) mod update;
pub(crate) mod util;
mod wizard;

pub fn run_client(client: CliClient) -> Result<(), String> {
    match client.command {
        CommandsClient::Gen(_) => {
            Generator::create()?.gen()?;
            Ok(())
        }
        CommandsClient::Send(send_command) => Sender::create(send_command)?.send(),
        CommandsClient::Update(update_command) => Updater::create(
            update_command.force,
            update_command.version,
            update_command.bin_path,
            update_command.server,
        )?
        .update(),
        CommandsClient::Wizard(_) => Wizard::create().run(),
    }
}

#[cfg(test)]
mod tests {
    use crate::client::config::CliClient;
    use crate::client::gen::Generator;
    use crate::client::run_client;
    use clap::Parser;

    #[test]
    fn test_gen() {
        let result = run_client(CliClient::parse_from(vec!["ruroco", "gen"]));
        assert!(result.is_ok());
    }

    #[test]
    fn test_send() {
        let key = Generator::create().unwrap().gen().unwrap();

        let result = run_client(CliClient::parse_from(vec![
            "ruroco",
            "send",
            "-a",
            "127.0.0.1:1234",
            "-k",
            &key,
            "-i",
            "192.168.178.123",
        ]));

        assert!(result.is_ok());
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_update() {
        let result = run_client(CliClient::parse_from(vec!["ruroco", "update"]));

        assert!(result.is_ok());
    }
}
