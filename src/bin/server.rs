use clap::Parser;
use ruroco::server::config::CliServer;
use ruroco::server::run_server;

fn main() -> Result<(), String> {
    run_server(CliServer::parse())
}
