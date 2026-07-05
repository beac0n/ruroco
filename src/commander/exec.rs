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

        // Connecting to a Unix socket requires *write* permission on the socket file. Owner (the
        // server user, set via change_socket_ownership) gets write, so only it can connect; group
        // and others get no write and therefore cannot. The `r` bit for others is inert for a
        // socket (read perm grants nothing on connect) and is kept only for ls/debugging clarity.
        let mode = 0o204;
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
    use super::{run_commander, Commander};
    use crate::commander::{CliCommander, ConfigCommander, ConfigCommands};
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::path::PathBuf;

    fn create_commander(config_dir: PathBuf, allow_non_routable_ips: bool) -> Commander {
        Commander::create(
            ConfigCommander {
                config_dir,
                allow_non_routable_ips,
                ..Default::default()
            },
            ConfigCommands {
                commands: HashMap::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_run_commander_invalid_path() {
        let commander = CliCommander {
            config: PathBuf::from("/nonexistent/ruroco_test_path.toml"),
            commands: PathBuf::from("/nonexistent/ruroco_test_commands.toml"),
        };
        assert!(run_commander(commander).is_err());
    }

    #[test]
    fn test_is_ip_allowed_rejects_non_routable() {
        // Every category the guard rejects: unspecified, loopback, multicast, and the v4/v6
        // private/link-local/ULA/broadcast/documentation ranges. run_command only consults this
        // guard when allow_non_routable_ips is false; the flag bypasses it entirely.
        let rejected = [
            "0.0.0.0",         // unspecified v4
            "::",              // unspecified v6
            "127.0.0.1",       // loopback v4
            "::1",             // loopback v6
            "10.0.0.1",        // private v4 (10.0.0.0/8)
            "172.16.0.1",      // private v4 (172.16.0.0/12)
            "192.168.1.1",     // private v4 (192.168.0.0/16)
            "169.254.1.1",     // link-local v4
            "fe80::1",         // link-local v6
            "fc00::1",         // unique local v6 (fc00::/8)
            "fd00::1",         // unique local v6 (fd00::/8)
            "224.0.0.1",       // multicast v4
            "ff02::1",         // multicast v6
            "192.0.2.1",       // documentation v4 (TEST-NET-1)
            "198.51.100.1",    // documentation v4 (TEST-NET-2)
            "203.0.113.1",     // documentation v4 (TEST-NET-3)
            "255.255.255.255", // broadcast v4
        ];

        for ip in rejected {
            let addr: IpAddr = ip.parse().unwrap();
            assert!(!Commander::is_ip_allowed(addr), "expected {ip} to be rejected");
        }
    }

    #[test]
    fn test_is_ip_allowed_accepts_public_unicast() {
        let allowed = [
            "1.2.3.4",              // public unicast v4
            "8.8.8.8",              // public unicast v4
            "2606:4700:4700::1111", // public unicast v6
        ];

        for ip in allowed {
            let addr: IpAddr = ip.parse().unwrap();
            assert!(Commander::is_ip_allowed(addr), "expected {ip} to be accepted");
        }
    }

    #[test]
    fn test_run_command_rejects_loopback_when_not_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("rejected.txt");
        let output_path = output_file.to_str().unwrap();
        create_commander(dir.path().to_path_buf(), false)
            .run_command(&format!("touch {output_path}"), "127.0.0.1".parse().unwrap());
        assert!(!output_file.exists(), "command must not run for a rejected IP");
    }

    #[test]
    fn test_run_command_accepts_loopback_when_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("accepted.txt");
        let output_path = output_file.to_str().unwrap();
        create_commander(dir.path().to_path_buf(), true)
            .run_command(&format!("touch {output_path}"), "127.0.0.1".parse().unwrap());
        assert!(output_file.exists(), "command must run when non-routable IPs are allowed");
    }
}
