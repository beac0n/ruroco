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
    config: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let config_path = args.config;
    let config_str = fs::read_to_string(&config_path).map_err(|e| format!("Could not read {config_path:?}: {e}"))?;
    let config: Config = toml::from_str(&config_str).map_err(|e| format!("Could not create TOML from {config_path:?}: {e}"))?;
    Commander::create(config.commands).run()
}
