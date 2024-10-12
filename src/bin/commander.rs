use clap::Parser;
use ruroco::commander::run_commander;
use ruroco::config_server::CliServer;

fn main() -> Result<(), String> {
    run_commander(CliServer::parse())
}
