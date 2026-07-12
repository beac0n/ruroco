use super::Commander;
use crate::commander::ip_filter;
use crate::commander::CliCommander;
use crate::common::logging::error;
use crate::common::{change_file_ownership, info};
use anyhow::{bail, Context};
use nix::sys::stat::{umask, Mode};
use std::fs::Permissions;
use std::net::IpAddr;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, process, str, thread};

const ENV_PREFIX: &str = "RUROCO_";
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// How a spawned command finished: on its own, or killed at the timeout deadline.
enum CommandExit {
    Completed(std::process::ExitStatus),
    TimedOut,
}

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

        // bind() creates the socket file at a permissive default (subject to umask), and we can
        // only chmod it to `mode` afterwards - leaving a window where a connectable, wide-open
        // socket exists (worse under a lax umask; command names like "default" are guessable).
        // Tighten the umask first so bind() itself creates the file owner-only, then restore it
        // regardless of outcome so no other file this process creates is affected.
        let previous_umask = umask(Mode::from_bits_truncate(0o077));
        let bind_result = UnixListener::bind(&self.socket_path);
        umask(previous_umask);
        let listener = bind_result
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

    /// Run `command` via `sh -c`, killing it (SIGKILL, the `sh` process only) once `timeout`
    /// elapses. Output is captured through temp files instead of pipes so a chatty command can
    /// never dead-lock on a full pipe buffer while we poll for its exit. Errors are logged, never
    /// returned: one bad command must not take down the accept loop.
    pub(super) fn run_command(&self, command: &str, timeout: Duration, ip: IpAddr) {
        if !self.allow_non_routable_ips && !Self::is_ip_allowed(ip) {
            return;
        }

        match Self::execute_with_timeout(command, timeout, ip) {
            Ok((CommandExit::Completed(status), stdout, stderr)) => {
                let msg = format!("{command} for {ip}\nstdout: {stdout}\nstderr: {stderr}");
                if status.success() {
                    info(format!("Execution was successful: {msg}"))
                } else {
                    error(format!("Execution was not successful: {msg}"))
                }
            }
            Ok((CommandExit::TimedOut, stdout, stderr)) => error(format!(
                "Execution timed out after {timeout:?} and was killed: {command} for {ip}\n\
                 stdout: {stdout}\nstderr: {stderr}"
            )),
            Err(e) => error(format!("Error executing {command} for {ip}: {e}")),
        }
    }

    fn execute_with_timeout(
        command: &str,
        timeout: Duration,
        ip: IpAddr,
    ) -> anyhow::Result<(CommandExit, String, String)> {
        let stdout_path = Self::temp_output_path("out")?;
        let stderr_path = Self::temp_output_path("err")?;

        let result = Self::spawn_and_wait(command, timeout, ip, &stdout_path, &stderr_path);

        // Whatever happened, collect the partial output and clean the temp files up.
        let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
        let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
        let _ = fs::remove_file(&stdout_path);
        let _ = fs::remove_file(&stderr_path);

        result.map(|exit| (exit, stdout, stderr))
    }

    fn spawn_and_wait(
        command: &str,
        timeout: Duration,
        ip: IpAddr,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> anyhow::Result<CommandExit> {
        let stdout_file = fs::File::create(stdout_path)
            .with_context(|| format!("Could not create stdout capture {stdout_path:?}"))?;
        let stderr_file = fs::File::create(stderr_path)
            .with_context(|| format!("Could not create stderr capture {stderr_path:?}"))?;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .env(format!("{ENV_PREFIX}IP"), ip.to_string())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .spawn()
            .with_context(|| format!("Could not spawn {command}"))?;

        // checked_add is None only when timeout_sec is so large that Instant::now() + timeout
        // would overflow; treat that as "no deadline" rather than panicking or silently capping
        // the admin-configured timeout to some arbitrary value.
        let deadline = Instant::now().checked_add(timeout);
        loop {
            match child.try_wait().with_context(|| format!("Could not poll {command}"))? {
                Some(status) => return Ok(CommandExit::Completed(status)),
                None if deadline.is_some_and(|d| Instant::now() >= d) => {
                    // SIGKILL the `sh` process only (not its process group); then reap it so no
                    // zombie is left behind.
                    child.kill().with_context(|| format!("Could not kill {command}"))?;
                    child.wait().with_context(|| format!("Could not reap {command}"))?;
                    return Ok(CommandExit::TimedOut);
                }
                None => thread::sleep(POLL_INTERVAL),
            }
        }
    }

    /// A unique per-invocation path in the temp dir for capturing one output stream.
    fn temp_output_path(kind: &str) -> anyhow::Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .with_context(|| "system clock before epoch")?
            .as_nanos();
        Ok(env::temp_dir().join(format!("ruroco-cmd-{}-{nanos}-{kind}", process::id())))
    }

    /// Returns `true` if the command may run for this IP. The IP reaches the executed command via
    /// `$RUROCO_IP`, so only allow globally-routable unicast peers: reject loopback, private, and
    /// other non-routable addresses a client must not be able to whitelist.
    fn is_ip_allowed(ip: IpAddr) -> bool {
        let allowed = ip_filter::is_routable(ip);
        if !allowed {
            error(format!("refusing to execute with non-routable IP: {ip}"));
        }
        allowed
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
    use std::path::PathBuf;
    use std::time::Duration;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    fn create_commander(config_dir: PathBuf, allow_non_routable_ips: bool) -> Commander {
        Commander::create(
            ConfigCommander {
                config_dir,
                allow_non_routable_ips,
                ..Default::default()
            },
            ConfigCommands::from_map(HashMap::new()),
        )
        .unwrap()
    }

    #[test]
    fn test_create_listener_sets_final_permissions_and_restores_umask() {
        use nix::sys::stat::{umask, Mode};
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let commander = create_commander(dir.path().to_path_buf(), false);

        // Reading the current umask requires setting one; restore it immediately after.
        let before = umask(Mode::from_bits_truncate(0o022));
        umask(before);

        let _listener = commander.create_listener().unwrap();

        let after = umask(Mode::from_bits_truncate(0o022));
        umask(after);
        assert_eq!(
            before, after,
            "create_listener must restore the process umask, not leave it tightened"
        );

        let mode = std::fs::metadata(&commander.socket_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o204, "socket must end up at the documented final permissions");
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
    fn test_run_command_rejects_loopback_when_not_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("rejected.txt");
        let output_path = output_file.to_str().unwrap();
        create_commander(dir.path().to_path_buf(), false).run_command(
            &format!("touch {output_path}"),
            TEST_TIMEOUT,
            "127.0.0.1".parse().unwrap(),
        );
        assert!(!output_file.exists(), "command must not run for a rejected IP");
    }

    #[test]
    fn test_run_command_accepts_loopback_when_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("accepted.txt");
        let output_path = output_file.to_str().unwrap();
        create_commander(dir.path().to_path_buf(), true).run_command(
            &format!("touch {output_path}"),
            TEST_TIMEOUT,
            "127.0.0.1".parse().unwrap(),
        );
        assert!(output_file.exists(), "command must run when non-routable IPs are allowed");
    }
}
