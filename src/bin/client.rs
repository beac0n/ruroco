use clap::Parser;

use ruroco::client::config::CliClient;
use ruroco::client::run_client;

fn main() -> Result<(), String> {
    run_client(CliClient::parse())
}
