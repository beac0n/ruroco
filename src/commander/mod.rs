//! The privileged executor (runs as root). Owns the Unix socket, reads a 24-byte `CommanderData`
//! off it, looks the `cmd_hash` up in its `cmds` map (built from `ConfigCommands`), and runs the
//! configured shell command. Never touches crypto, keys, or the network: it trusts the Unix socket
//! (see the threat-model discussion in `.todo/03`) and links neither OpenSSL nor the decrypt path.

mod config;
mod exec;
#[cfg(test)]
mod tests;

pub use config::{CliCommander, ConfigCommander, ConfigCommands};
pub use exec::run_commander;

use crate::commander::config::CommandSpec;
use crate::common::info;
use crate::common::ipc::{get_commander_unix_socket_path, CommanderData, CMDR_DATA_SIZE};
use crate::common::logging::error;
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct Commander {
    pub(super) socket_path: PathBuf,
    pub(super) cmds: HashMap<u64, CommandSpec>,
    pub(super) socket_user: String,
    pub(super) socket_group: String,
    pub(super) allow_non_routable_ips: bool,
}

impl Commander {
    pub(super) fn create_from_paths(
        config_path: &Path,
        commands_path: &Path,
    ) -> anyhow::Result<Commander> {
        let config = ConfigCommander::create_from_path(config_path)?;
        let commands = ConfigCommands::create_from_path(commands_path)?;
        Commander::create(config, commands)
    }

    pub fn create(config: ConfigCommander, commands: ConfigCommands) -> anyhow::Result<Commander> {
        Ok(Commander {
            cmds: commands.get_hash_to_cmd()?,
            socket_path: get_commander_unix_socket_path(
                config.socket_dir.as_ref().unwrap_or(&config.config_dir),
            ),
            socket_user: config.socket_user,
            socket_group: config.socket_group,
            allow_non_routable_ips: config.allow_non_routable_ips,
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        for stream in self.create_listener()?.incoming() {
            match stream {
                Ok(mut stream) => {
                    if let Err(e) = self.run_cycle(&mut stream) {
                        error(e)
                    }
                }
                Err(e) => error(format!("Connection for {:?} failed: {e}", &self.socket_path)),
            }
        }

        Ok(())
    }

    fn run_cycle(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .with_context(|| format!("Could not set read timeout for {:?}", &self.socket_path))?;

        let msg = Commander::read(stream)?;
        let cmdr_data: CommanderData = msg.into();
        let cmd_hash = &cmdr_data.cmd_hash;
        let spec =
            self.cmds.get(cmd_hash).ok_or_else(|| anyhow!("Unknown command name: {cmd_hash}"))?;

        info(format!("Running command ({cmd_hash}) {}", spec.cmd));
        self.run_command(&spec.cmd, spec.timeout, cmdr_data.ip);
        Ok(())
    }

    fn read(stream: &mut UnixStream) -> anyhow::Result<[u8; CMDR_DATA_SIZE]> {
        let mut buffer = [0u8; CMDR_DATA_SIZE];
        stream
            .read_exact(&mut buffer)
            .with_context(|| "Could not read command from Unix Stream to string")?;
        Ok(buffer)
    }
}
