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
    #[serde(default = "default_max_delay")]
    max_delay: u128,
}

fn default_address() -> String {
    String::from("127.0.0.1:8080")
}

fn default_pem_path() -> PathBuf {
    PathBuf::from("ruroco_public.pem")
}

fn default_max_delay() -> u128 {
    5_000_000_000
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'c', long, default_value = PathBuf::from("/etc/ruroco-server/config.toml").into_os_string())]
    config_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    let config: Config = toml::from_str(&fs::read_to_string(args.config_path)?)?;
    Server::create(config.pem_path, config.address, config.max_delay)?.run()
}
