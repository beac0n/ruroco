use clap::Parser;
use ruroco::config::config_server::CliServer;
use ruroco::server::commander::run_commander;

fn main() -> Result<(), String> {
    run_commander(CliServer::parse())
}
