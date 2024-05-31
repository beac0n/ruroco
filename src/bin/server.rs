use std::{fs, str};
use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

use ruroco::common::init_logger;
use ruroco::server::Server;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "default_address")]
    address: String,
    #[serde(default = "default_pem_path")]
    pem_path: PathBuf,
    #[serde(default = "default_max_delay_sec")]
    max_delay_sec: u16,
}

fn default_address() -> String {
    String::from("127.0.0.1:8080")
}

fn default_pem_path() -> PathBuf {
    PathBuf::from("ruroco_public.pem")
}

fn default_max_delay_sec() -> u16 {
    5
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco-server/config.toml").into_os_string())]
    config: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let config_path = args.config;
    let config_str = fs::read_to_string(&config_path).map_err(|e| format!("Could not read {config_path:?}: {e}"))?;
    let config: Config = toml::from_str(&config_str).map_err(|e| format!("Could not create TOML from {config_path:?}: {e}"))?;
    Server::create(config.pem_path, config.address, config.max_delay_sec)?.run()
}
