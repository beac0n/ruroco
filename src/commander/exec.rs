use super::Commander;
use crate::commander::CliCommander;
use crate::common::logging::error;
use crate::common::{change_file_ownership, info};
use anyhow::{bail, Context};
use std::fs::Permissions;
use std::net::IpAddr;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::{fs, str};

const ENV_PREFIX: &str = "RUROCO_";

impl Commander {
    pub(super) fn create_listener(&self) -> anyhow::Result<UnixListener> {
        let socket_dir = match self.socket_path.parent() {
            Some(socket_dir) => socket_dir,
            None => bail!("Could not get parent dir for {:?}", &self.socket_path),
        };
        fs::create_dir_all(socket_dir)
            .with_context(|| format!("Could not create parents for {socket_dir:?}"))?;

        let _ = fs::remove_file(&self.socket_path);

        let mode = 0o204; // only server should be able to write, everyone else can read
        info(format!("Binding Unix Listener on {:?} with permissions {mode:o}", &self.socket_path));
        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Could not bind to socket {:?}", self.socket_path))?;

        fs::set_permissions(&self.socket_path, Permissions::from_mode(mode)).with_context(
            || format!("Could not set permissions {mode:o} for {:?}", self.socket_path),
        )?;
        self.change_socket_ownership()?;

        Ok(listener)
    }

    pub(super) fn change_socket_ownership(&self) -> anyhow::Result<()> {
        change_file_ownership(&self.socket_path, self.socket_user.trim(), self.socket_group.trim())
    }

    pub(super) fn run_command(&self, command: &str, ip: IpAddr) {
        if Self::sanitize_ip(ip) {
            return;
        }

        match Command::new("sh")
            .arg("-c")
            .arg(command)
            .env(format!("{ENV_PREFIX}IP"), ip.to_string())
            .output()
        {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                let msg = format!("{command}\nstdout: {stdout}\nstderr: {stderr}");
                if result.status.success() {
                    info(format!("Execution was successful: {msg}"))
                } else {
                    error(format!("Execution was not successful: {msg}"))
                }
            }
            Err(e) => error(format!("Error executing {command}: {e}")),
        };
    }

    fn sanitize_ip(ip: IpAddr) -> bool {
        let ip_str = ip.to_string();
        if !ip_str.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':') {
            error(format!("refusing to execute with suspicious IP: {:?}", ip_str));
            true
        } else {
            false
        }
    }
}

pub fn run_commander(commander: CliCommander) -> anyhow::Result<()> {
    Commander::create_from_paths(&commander.config, &commander.commands)?.run()
}

#[cfg(test)]
mod tests {
    use super::run_commander;
    use crate::commander::CliCommander;
    use std::path::PathBuf;

    #[test]
    fn test_run_commander_invalid_path() {
        let commander = CliCommander {
            config: PathBuf::from("/nonexistent/ruroco_test_path.toml"),
            commands: PathBuf::from("/nonexistent/ruroco_test_commands.toml"),
        };
        assert!(run_commander(commander).is_err());
    }
}
