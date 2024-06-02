use std::fs;

use clap::Parser;

use ruroco::commander::Commander;
use ruroco::common::init_logger;
use ruroco::config::{Cli, Config};

fn main() -> Result<(), String> {
    init_logger();
    let args = Cli::parse();
    let config_path = args.config;
    let config_str = fs::read_to_string(&config_path)
        .map_err(|e| format!("Could not read {config_path:?}: {e}"))?;
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| format!("Could not create TOML from {config_path:?}: {e}"))?;
    Commander::create(config.commands, config.socket_user, config.socket_group, config.socket_path)
        .run()
}
