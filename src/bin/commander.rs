use clap::Parser;
use ruroco::server::config::CliServer;
use ruroco::server::run_commander;

fn main() -> anyhow::Result<()> {
    run_commander(CliServer::parse())
}
