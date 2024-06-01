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
pub struct CommanderCommand {
    #[serde(default = "default_start")]
    pub start: String,
    #[serde(default = "default_stop")]
    pub stop: String,
    #[serde(default = "default_sleep")]
    pub sleep: u64,
}

impl CommanderCommand {
    pub fn create(start: String, stop: String, sleep: u64) -> CommanderCommand {
        CommanderCommand { start, stop, sleep }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub commands: HashMap<String, CommanderCommand>,
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_pem_path")]
    pub pem_path: PathBuf,
    #[serde(default = "default_max_delay_sec")]
    pub max_delay_sec: u16,
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

fn default_start() -> String {
    String::from("echo 'start'")
}

fn default_stop() -> String {
    String::from("echo 'stop'")
}

fn default_sleep() -> u64 {
    5
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

fn default_socket_path() -> PathBuf {
    PathBuf::from("/etc/ruroco/ruroco.socket")
}
