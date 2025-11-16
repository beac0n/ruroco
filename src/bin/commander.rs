use clap::Parser;
use ruroco::server::commander::run_commander;
use ruroco::server::config::CliServer;

fn main() -> Result<(), String> {
    run_commander(CliServer::parse())
}
