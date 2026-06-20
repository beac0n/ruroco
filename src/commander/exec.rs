use super::Commander;
use crate::commander::CliCommander;
use crate::common::logging::error;
use crate::common::{change_file_ownership, info};
use anyhow::{bail, Context};
use std::fs::Permissions;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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
        if !self.allow_non_routable_ips && !Self::is_ip_allowed(ip) {
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
                let msg = format!("{command} for {ip}\nstdout: {stdout}\nstderr: {stderr}");
                if result.status.success() {
                    info(format!("Execution was successful: {msg}"))
                } else {
                    error(format!("Execution was not successful: {msg}"))
                }
            }
            Err(e) => error(format!("Error executing {command} for {ip}: {e}")),
        };
    }

    /// Returns `true` if the command may run for this IP. The IP reaches the executed command via
    /// `$RUROCO_IP`, so only allow globally-routable unicast peers: reject loopback, private, and
    /// other non-routable addresses a client must not be able to whitelist.
    fn is_ip_allowed(ip: IpAddr) -> bool {
        let reject = ip.is_unspecified()
            || ip.is_loopback()
            || ip.is_multicast()
            || match ip {
                IpAddr::V4(v4) => Self::is_ipv4_rejected(v4),
                IpAddr::V6(v6) => Self::is_ipv6_rejected(v6),
            };

        if reject {
            error(format!("refusing to execute with non-routable IP: {ip}"));
        }

        !reject
    }

    fn is_ipv6_rejected(v6: Ipv6Addr) -> bool {
        v6.is_unique_local() || v6.is_unicast_link_local()
    }

    fn is_ipv4_rejected(v4: Ipv4Addr) -> bool {
        v4.is_broadcast() || v4.is_private() || v4.is_link_local() || v4.is_documentation()
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
