use std::{fs, str};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

use ruroco::commander::{Commander, CommanderCommand};
use ruroco::common::init_logger;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(flatten)]
    commands: HashMap<String, CommanderCommand>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'c', long, default_value = PathBuf::from("/etc/ruroco-commander/config.toml").into_os_string())]
    config_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let config: Config = toml::from_str(&fs::read_to_string(args.config_path)?)?;
    Commander::create(config.commands).run()
}
