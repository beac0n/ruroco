use clap::Parser;

use ruroco::client::run_client;
use ruroco::config_client::CliClient;

fn main() -> Result<(), String> {
    run_client(CliClient::parse())
}
