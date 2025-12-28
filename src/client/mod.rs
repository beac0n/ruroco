//! This module is responsible for sending data to the server and for generating key file

use crate::client::config::{CliClient, CommandsClient};
use crate::client::gen::Generator;
use crate::client::lock::ClientLock;
use crate::client::send::Sender;
use crate::client::update::Updater;
use crate::client::wizard::Wizard;

/// data structures for using CLI arguments for the client binary
pub mod config;
pub mod counter;
pub mod gen;
pub(crate) mod lock;
pub mod send;
pub(crate) mod update;
pub(crate) mod util;
mod wizard;

pub fn run_client_send(client: CliClient) -> anyhow::Result<()> {
    match client.command {
        CommandsClient::Send(send_command) => Sender::create(send_command)?.send(),
        _ => Err(anyhow::anyhow!("Invalid command for run_client_send")),
    }
}

pub fn run_client(client: CliClient) -> anyhow::Result<()> {
    let conf_dir = config::get_conf_dir()?;
    let _lock = ClientLock::acquire(conf_dir.join("client.lock"))?;

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
    use tempfile::TempDir;

    fn set_test_conf_dir() -> TempDir {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        dir
    }

    #[test]
    fn test_gen() {
        let _conf_dir = set_test_conf_dir();
        let result = run_client(CliClient::parse_from(vec!["ruroco", "gen"]));
        assert!(result.is_ok());
    }

    #[test]
    fn test_send() {
        let _conf_dir = set_test_conf_dir();
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
        let _conf_dir = set_test_conf_dir();
        let bin_dir = tempfile::tempdir().expect("failed to create temp dir");
        let bin_path = bin_dir.path().to_str().expect("temp dir not valid utf8");
        let result =
            run_client(CliClient::parse_from(vec!["ruroco", "update", "--bin-path", bin_path]));

        assert!(result.is_ok());
    }
}
