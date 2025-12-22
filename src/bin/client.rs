use clap::Parser;

use ruroco::client::config::CliClient;
use ruroco::client::run_client;

fn main() -> anyhow::Result<()> {
    run_client(CliClient::parse())
}
