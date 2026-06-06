use clap::Parser;
use ruroco::server::config::CliCommander;
use ruroco::server::run_commander;

fn main() -> anyhow::Result<()> {
    run_commander(CliCommander::parse())
}
