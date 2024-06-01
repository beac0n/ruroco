use std::error::Error;
use std::fs;

use clap::Parser;

use ruroco::common::init_logger;
use ruroco::config::{Cli, Config};
use ruroco::server::Server;

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let config_path = args.config;
    let config_str = fs::read_to_string(&config_path)
        .map_err(|e| format!("Could not read {config_path:?}: {e}"))?;
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| format!("Could not create TOML from {config_path:?}: {e}"))?;
    Server::create(config.pem_path, config.address, config.max_delay_sec, config.socket_path)?.run()
}
