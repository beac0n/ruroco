use clap::Parser;
use ruroco::commander::{run_commander, CliCommander};

fn main() -> anyhow::Result<()> {
    run_commander(CliCommander::parse())
}
