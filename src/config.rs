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
    #[serde(default = "default_pem_path")]
    pub pem_path: PathBuf, // TODO: add pem directory instead of path, so that multiple PEMs can be used
    #[serde(default = "default_socket_user")]
    pub socket_user: String,
    #[serde(default = "default_socket_group")]
    pub socket_group: String,
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,
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

fn default_pem_path() -> PathBuf {
    PathBuf::from("ruroco_public.pem")
}

fn default_socket_path() -> PathBuf {
    PathBuf::from("/etc/ruroco/ruroco.socket")
}
