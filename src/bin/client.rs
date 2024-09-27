use clap::Parser;

use ruroco::client::{gen, send};
use ruroco::common::time_from_ntp;
use ruroco::config_client::{CliClient, CommandsClient};

fn main() -> Result<(), String> {
    match CliClient::parse().command {
        CommandsClient::Gen {
            private_pem_path,
            public_pem_path,
            key_size,
        } => gen(private_pem_path, public_pem_path, key_size),
        CommandsClient::Send {
            private_pem_path,
            address,
            command,
            deadline,
            strict,
            ip,
            ntp,
        } => send(private_pem_path, address, command, deadline, strict, ip, time_from_ntp(&ntp)?),
    }
}
