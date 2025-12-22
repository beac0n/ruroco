use clap::Parser;
use ruroco::server::config::CliServer;
use ruroco::server::run_server;

fn main() -> anyhow::Result<()> {
    run_server(CliServer::parse())
}
