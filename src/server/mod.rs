//! The network-facing server (unprivileged, `with-server`): owns the UDP socket, decrypts incoming
//! packets, runs replay/IP/rate validation, and forwards a `CommanderData` to the privileged
//! commander over the Unix socket. The server never replies.
//!
//! The commander it talks to lives in the top-level `commander` module; the shared config
//! (`ConfigServer`) and IPC contract (`CommanderData`, socket path) live in `common`.

/// persists the blocked list of deadlines
pub mod blocklist;
/// the server's view of `config.toml` (`ConfigServer`) and its CLI (`CliServer`)
pub mod config;
mod handler;
mod keys;
mod listener;
mod rate_limiter;
mod signal;
mod socket;

pub use listener::{run_server, Server};
