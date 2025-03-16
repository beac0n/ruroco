use clap::Parser;

use ruroco::client::client::run_client;
use ruroco::config::config_client::CliClient;

fn main() -> Result<(), String> {
    run_client(CliClient::parse())
}
