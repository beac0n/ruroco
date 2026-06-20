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

/// Test-only helper to pick a random port in the half-open range `[from, to)`. Lives here (not in
/// production code) because its only callers are the server's unit tests.
#[cfg(test)]
pub(crate) fn get_random_range(from: u16, to: u16) -> anyhow::Result<u16> {
    use anyhow::Context;
    use openssl::rand::rand_bytes;
    let span = u32::from(
        to.checked_sub(from).filter(|s| *s > 0).context("get_random_range: empty range")?,
    );
    let mut buf = [0u8; 4];
    rand_bytes(&mut buf).with_context(|| "Could not generate number")?;
    Ok(from + (u32::from_be_bytes(buf) % span) as u16)
}
