use clap::Parser;

use ruroco::client::{gen, send};
use ruroco::common::time_from_ntp;
use ruroco::config_client::{CliClient, CommandsClient};

fn main() -> Result<(), String> {
    match CliClient::parse().command {
        CommandsClient::Gen(gen_command) => {
            gen(gen_command.private_pem_path, gen_command.public_pem_path, gen_command.key_size)
        }
        CommandsClient::Send(send_command) => {
            let ntp = send_command.ntp.clone();
            send(send_command, time_from_ntp(&ntp)?)
        }
    }
}
