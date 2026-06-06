//! Server-side configuration, split by process.
//!
//! `server` holds the shared `config.toml` schema (`ConfigServer`, `CliServer`) used by both the
//! server and the commander. `commander` holds the commander-only `commands.toml` schema
//! (`ConfigCommands`, `CliCommander`), which the network-facing server never loads.

mod commander;
mod server;

pub use commander::{CliCommander, ConfigCommands};
pub use server::{CliServer, ConfigServer};
