use clap::Parser;
use ruroco::config::config_server::CliServer;
use ruroco::server::run_server;

fn main() -> Result<(), String> {
    run_server(CliServer::parse())
}
