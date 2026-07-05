use crate::client::config::DEFAULT_COMMAND;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub(crate) struct GenCommand {}

#[derive(Parser, Debug)]
pub(crate) struct ReseedCommand {}

#[derive(Parser, Debug)]
pub struct SendCommand {
    /// Address to send the command to.
    #[arg(short, long)]
    pub address: String,
    /// Path to a file containing the base64 key with id (output of `ruroco-client gen` or the
    /// UI). The key itself is never accepted directly (CLI arg or otherwise): that would leak it
    /// via `ps`, shell history, or in-memory copies outliving their need.
    #[arg(short = 'k', long = "key-file")]
    pub key_file: PathBuf,
    /// Command to send
    #[arg(short, long, default_value = DEFAULT_COMMAND)]
    pub command: String,
    #[arg(short = 'e', long)]
    /// Allow permissive IP validation - source IP does not have to match provided IP.
    pub permissive: bool,
    /// Optional IP address from which the command was sent.
    /// Use -6ei "dead:beef:dead:beef::/64" to allow you whole current IPv6 network.
    /// To do this automatically, use -6ei $(curl -s6 https://api64.ipify.org | awk -F: '{print $1":"$2":"$3":"$4"::/64"}')
    #[arg(short, long)]
    pub ip: Option<String>,
    /// Connect via IPv4
    #[arg(short = '4', long)]
    pub ipv4: bool,
    /// Connect via IPv6
    #[arg(short = '6', long)]
    pub ipv6: bool,
    /// Delay in milliseconds between sending to multiple destinations (IPv4 + IPv6)
    #[arg(short = 'd', long, default_value = "50")]
    pub send_delay_ms: u64,
}

#[derive(Parser, Debug)]
pub(crate) struct UpdateCommand {
    /// Force update
    #[arg(short, long)]
    pub(crate) force: bool,
    /// Version
    #[arg(short, long)]
    pub(crate) version: Option<String>,
    /// Path where binaries are saved
    #[arg(short, long)]
    pub(crate) bin_path: Option<PathBuf>,
    /// Update for server side
    #[arg(short, long)]
    pub(crate) server: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct WizardCommand {}

impl Default for SendCommand {
    fn default() -> SendCommand {
        SendCommand {
            address: "127.0.0.1:80".to_string(),
            key_file: PathBuf::new(),
            command: DEFAULT_COMMAND.to_string(),
            permissive: false,
            ip: None,
            ipv4: false,
            ipv6: false,
            send_delay_ms: 50,
        }
    }
}
