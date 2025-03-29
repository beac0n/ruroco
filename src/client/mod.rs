//! This module is responsible for sending data to the server and for generating PEM files

use crate::common::time_from_ntp;
use crate::config::config_client::{CliClient, CommandsClient};

pub mod gen;
mod github_api_definition;
pub mod send;
pub mod update;

pub fn run_client(client: CliClient) -> Result<(), String> {
    match client.command {
        CommandsClient::Gen(gen_command) => gen::gen(
            &gen_command.private_pem_path,
            &gen_command.public_pem_path,
            gen_command.key_size,
        ),
        CommandsClient::Send(send_command) => {
            let ntp = send_command.ntp.clone();
            send::send(send_command, time_from_ntp(&ntp)?)
        }
        CommandsClient::Update => update::update(false),
    }
}

#[cfg(test)]
mod tests {
    use crate::common::data::ClientData;
    use crate::config::config_client::CliClient;
    use clap::error::ErrorKind::DisplayHelp;
    use clap::Parser;

    #[test]
    fn test_print_help() {
        let result = CliClient::try_parse_from(vec!["ruroco", "--help"]);
        assert_eq!(result.unwrap_err().kind(), DisplayHelp);
    }

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            5,
            false,
            Some("192.168.178.123".to_string()),
            "192.168.178.124".to_string(),
            1725821510 * 1_000_000_000,
        )
        .serialize()
        .unwrap();
        let server_data_str = String::from_utf8_lossy(&server_data).to_string();

        assert_eq!(server_data_str, "c=\"some_kind_of_long_but_not_really_that_long_command\"\nd=\"1725821515000000000\"\ns=0\ni=\"192.168.178.123\"\nh=\"192.168.178.124\"");
        assert_eq!(
            ClientData::deserialize(&server_data).unwrap(),
            ClientData {
                c: "some_kind_of_long_but_not_really_that_long_command".to_string(),
                d: 1725821515000000000,
                s: 0,
                i: Some("192.168.178.123".to_string()),
                h: "192.168.178.124".to_string()
            }
        );
    }
}
