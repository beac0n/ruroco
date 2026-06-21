//! Server configuration: the `config.toml` fields the network-facing server reads, plus its CLI
//! (`CliServer`). The commander reads the *same* `config.toml` file but through its own
//! `ConfigCommander` view (only `config_dir` is shared between the two; it must agree so both sides
//! resolve the same `ruroco.socket`). Server-only fields (`ips`, rate limit, clock skew) live here;
//! commander-only fields (`socket_user`/`socket_group`) live in `commander::config`.
//!
//! The inherent methods that act on `config_dir` (keys, UDP socket, blocklist) are separate
//! `impl ConfigServer` blocks in `keys.rs` and `socket.rs`.

use anyhow::{anyhow, Context};
use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliServer {
    #[arg(short, long, default_value = PathBuf::from("/etc/ruroco/config.toml").into_os_string())]
    pub(crate) config: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    /// Destination IPs the server accepts (the `dst_ip` carried in a packet must be one of these).
    /// Set to the server's own public address(es). Parsed and `normalize_ip`'d on load, so an
    /// IPv6-mapped IPv4 entry collapses to plain IPv4.
    #[serde(deserialize_with = "deserialize_ips")]
    pub ips: Vec<IpAddr>,
    /// Address the server binds when systemd socket activation is NOT used. Lower priority than an
    /// explicit CLI/arg address, `RUROCO_LISTEN_ADDRESS`, and systemd socket activation; higher than
    /// the built-in `[::]:DEFAULT_PORT` fallback. Ignored under socket activation (the inherited fd
    /// wins), so the shipped systemd deployment is unaffected.
    #[serde(default)]
    pub address: Option<String>,
    /// Directory the server reads its `.key` files from (and the default location for the blocklist
    /// and socket when their dedicated dirs are unset). Shared with the commander, which must agree
    /// on it so both resolve the same `ruroco.socket`. Defaults to `/etc/ruroco`.
    #[serde(default = "default_config_path")]
    pub config_dir: PathBuf,
    /// Directory holding the persisted blocklist (`blocklist.msgpck`). When unset it defaults to
    /// `config_dir`. Point it at a dedicated systemd `StateDirectory` (e.g. `/var/lib/ruroco`) so
    /// the rest of `config_dir` (keys, config) can be mounted read-only. See
    /// `systemd/ruroco.service`.
    #[serde(default)]
    pub blocklist_dir: Option<PathBuf>,
    /// Directory holding the commander Unix socket (`ruroco.socket`). When unset it defaults to
    /// `config_dir`. Point it at a systemd `RuntimeDirectory` (e.g. `/run/ruroco`) shared with the
    /// commander. Server and commander MUST resolve the same path.
    #[serde(default)]
    pub socket_dir: Option<PathBuf>,
    /// Per-source-IP cap on accepted requests per second (~1s sliding window). Throttles a single
    /// chatty or abusive peer; see `max_requests_per_second_global` for the all-sources cap that
    /// covers spoofed-IP floods. In-memory only, so it resets on restart and is throttling, not
    /// replay defense. Defaults to 2.
    #[serde(default = "default_max_requests_per_second")]
    pub max_requests_per_second: u32,
    /// Global cap on accepted requests per second across ALL source IPs. Bounds total work (mainly
    /// decrypt attempts) under a spoofed-source-IP flood, which the per-IP limit cannot stop because
    /// each spoofed address looks like a fresh peer. Keep comfortably above expected legitimate
    /// aggregate traffic.
    #[serde(default = "default_max_requests_per_second_global")]
    pub max_requests_per_second_global: u32,
    /// Upper bound, in seconds, by which an accepted counter (a nanosecond timestamp) may exceed
    /// server-local `now`. A future-dated packet beyond this is rejected without touching the
    /// blocklist, so it can't permanently lock out a key; see `default_max_clock_skew_seconds`.
    /// Defaults to 3600.
    #[serde(default = "default_max_clock_skew_seconds")]
    pub max_clock_skew_seconds: u64,
}

fn deserialize_ips<'de, D>(d: D) -> Result<Vec<IpAddr>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<String> = Vec::<String>::deserialize(d)?;
    v.into_iter()
        .map(|s| {
            let ip: IpAddr = s.parse().map_err(serde::de::Error::custom)?;
            Ok(crate::common::normalize_ip(ip))
        })
        .collect()
}

impl ConfigServer {
    pub(crate) fn create_from_path(path: &Path) -> anyhow::Result<ConfigServer> {
        match fs::read_to_string(path) {
            Ok(data) => Self::deserialize(&data),
            Err(e) => Err(anyhow!("Could not read {path:?}: {e}")),
        }
    }

    pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigServer> {
        toml::from_str::<ConfigServer>(data).with_context(|| "Could not parse server config")
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for ConfigServer {
    fn default() -> ConfigServer {
        ConfigServer {
            ips: vec![IpAddr::from([127, 0, 0, 1])],
            address: None,
            config_dir: std::env::current_dir().unwrap_or(PathBuf::from("/tmp")),
            blocklist_dir: None,
            socket_dir: None,
            max_requests_per_second: default_max_requests_per_second(),
            max_requests_per_second_global: default_max_requests_per_second_global(),
            max_clock_skew_seconds: default_max_clock_skew_seconds(),
        }
    }
}

fn default_max_requests_per_second() -> u32 {
    2
}

fn default_max_requests_per_second_global() -> u32 {
    100
}

/// Upper bound, in seconds, by which an accepted counter (a nanosecond timestamp) may exceed
/// server-local `now`. Bounds how far a future-dated packet can push `last_seen`, turning a
/// permanent lockout into one recoverable by a client reseed. Only needs to cover client-vs-server
/// clock disagreement at counter seed time, not counter growth (the client counter increments by 1
/// per send, so it lags wall-clock).
fn default_max_clock_skew_seconds() -> u64 {
    3600
}

fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/ruroco")
}

#[cfg(test)]
mod tests {
    use super::{
        default_config_path, default_max_clock_skew_seconds, default_max_requests_per_second,
        default_max_requests_per_second_global, ConfigServer,
    };

    #[test]
    fn test_create_deserialize() {
        assert_eq!(
            ConfigServer::deserialize("ips = [\"127.0.0.1\"]").unwrap(),
            ConfigServer {
                ips: vec!["127.0.0.1".parse().unwrap()],
                address: None,
                config_dir: default_config_path(),
                blocklist_dir: None,
                socket_dir: None,
                max_requests_per_second: default_max_requests_per_second(),
                max_requests_per_second_global: default_max_requests_per_second_global(),
                max_clock_skew_seconds: default_max_clock_skew_seconds(),
            }
        );
    }

    #[test]
    fn test_deserialize_state_and_socket_dirs() {
        use std::path::PathBuf;
        let config = ConfigServer::deserialize(
            "ips = [\"127.0.0.1\"]\nblocklist_dir = \"/var/lib/ruroco\"\nsocket_dir = \"/run/ruroco\"",
        )
        .unwrap();
        assert_eq!(config.blocklist_dir, Some(PathBuf::from("/var/lib/ruroco")));
        assert_eq!(config.socket_dir, Some(PathBuf::from("/run/ruroco")));
    }

    #[test]
    fn test_dirs_default_to_none_when_absent() {
        let config = ConfigServer::deserialize("ips = [\"127.0.0.1\"]").unwrap();
        assert_eq!(config.blocklist_dir, None);
        assert_eq!(config.socket_dir, None);
    }

    #[test]
    fn test_deserialize_invalid_toml() {
        let result = ConfigServer::deserialize("this is not valid toml {{{}}}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not parse server config"));
    }

    #[test]
    fn test_deserialize_invalid_ip() {
        let result = ConfigServer::deserialize("ips = [\"not_an_ip\"]");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_ipv6_mapped_ip_is_normalized_to_ipv4() {
        let config = ConfigServer::deserialize("ips = [\"::ffff:127.0.0.1\"]").unwrap();
        assert_eq!(config.ips, vec!["127.0.0.1".parse::<std::net::IpAddr>().unwrap()]);
    }

    #[test]
    fn test_ignores_commander_only_fields() {
        // socket_user / socket_group belong to the commander's view of config.toml; the server's
        // ConfigServer must simply ignore them rather than fail to parse.
        let config = ConfigServer::deserialize(
            "ips = [\"127.0.0.1\"]\nsocket_user = \"ruroco\"\nsocket_group = \"ruroco\"",
        )
        .unwrap();
        assert_eq!(config.ips, vec!["127.0.0.1".parse::<std::net::IpAddr>().unwrap()]);
    }
}
