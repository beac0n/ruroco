use clap::Parser;

use ruroco::client::exec_cli_client;
use ruroco::config_client::CliClient;

fn main() -> Result<(), String> {
    exec_cli_client(CliClient::parse())
}
