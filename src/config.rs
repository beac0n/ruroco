use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub config: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub commands: HashMap<String, String>,
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
}

fn default_socket_user() -> String {
    String::from("ruroco")
}

fn default_socket_group() -> String {
    String::from("ruroco")
}

fn default_address() -> String {
    String::from("127.0.0.1:8080")
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}
