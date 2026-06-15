//! The privileged executor (runs as root). Owns the Unix socket, reads a 24-byte `CommanderData`
//! off it, looks the `cmd_hash` up in its `cmds` map (built from `ConfigCommands`), and runs the
//! configured shell command. Never touches crypto, keys, or the network: it trusts the Unix socket
//! (see the threat-model discussion in `.todo/03`) and links neither OpenSSL nor the decrypt path.

mod config;
mod exec;

pub use config::{CliCommander, ConfigCommander, ConfigCommands};
pub use exec::run_commander;

use crate::common::info;
use crate::common::ipc::{get_commander_unix_socket_path, CommanderData, CMDR_DATA_SIZE};
use crate::common::logging::error;
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Commander {
    pub(super) socket_path: PathBuf,
    pub(super) cmds: HashMap<u64, String>,
    pub(super) socket_user: String,
    pub(super) socket_group: String,
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
        let msg = Commander::read(stream)?;
        let cmdr_data: CommanderData = msg.into();
        let cmd_hash = &cmdr_data.cmd_hash;
        let cmd =
            self.cmds.get(cmd_hash).ok_or_else(|| anyhow!("Unknown command name: {cmd_hash}"))?;

        info(format!("Running command ({cmd_hash}) {cmd}"));
        self.run_command(cmd, cmdr_data.ip);
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

#[cfg(test)]
mod tests {
    use crate::commander::{Commander, ConfigCommander, ConfigCommands};
    use crate::common::ipc::{CommanderData, CMDR_DATA_SIZE};
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{env, fs, thread};

    fn create_commander(commands: HashMap<String, String>, config_dir: PathBuf) -> Commander {
        Commander::create(
            ConfigCommander {
                config_dir,
                ..Default::default()
            },
            ConfigCommands { commands },
        )
        .unwrap()
    }

    fn wait_for_socket(socket_path: &Path) {
        for _ in 0..50 {
            if socket_path.exists() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("socket was not created at {socket_path:?}");
    }

    fn send_to_socket(socket_path: &Path, data: CommanderData) {
        let mut stream = UnixStream::connect(socket_path).unwrap();
        let bytes: [u8; CMDR_DATA_SIZE] = data.into();
        stream.write_all(&bytes).unwrap();
        stream.flush().unwrap();
    }

    #[test]
    fn test_create_from_invalid_path() {
        // ConfigCommander's fields are all optional, so config_invalid.toml (which merely omits the
        // server-only `ips`) parses fine here; a malformed-syntax file is what the commander rejects.
        let path = env::current_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("tests")
            .join("files")
            .join("config_invalid_syntax.toml");

        let commands = PathBuf::from("/tmp/unused_commands.toml");
        let msg = Commander::create_from_paths(&path, &commands).unwrap_err().to_string();
        assert!(msg.contains("Could not create ConfigCommander from"), "unexpected error: {msg}");
    }

    #[test]
    fn test_create_from_invalid_toml_path() {
        let commands = PathBuf::from("/tmp/unused_commands.toml");
        assert_eq!(
            Commander::create_from_paths(&PathBuf::from("/tmp/path/does/not/exist"), &commands)
                .unwrap_err()
                .to_string(),
            r#"Could not read "/tmp/path/does/not/exist": No such file or directory (os error 2)"#
        );
    }

    #[test]
    fn test_create_from_path() {
        let mut commands = HashMap::new();
        commands.insert(
            "default".to_string(),
            "touch /tmp/ruroco_test/start.test /tmp/ruroco_test/stop.test".to_string(),
        );

        let base = env::current_dir().unwrap_or(PathBuf::from("/tmp"));
        let path = base.join("tests").join("files").join("config.toml");
        let commands_path = base.join("tests").join("conf_dir").join("commands.toml");

        assert_eq!(
            Commander::create_from_paths(&path, &commands_path).unwrap(),
            Commander::create(
                ConfigCommander {
                    config_dir: PathBuf::from("tests/conf_dir"),
                    socket_dir: None,
                    socket_user: "ruroco".to_string(),
                    socket_group: "ruroco".to_string(),
                },
                ConfigCommands { commands },
            )
            .unwrap()
        );
    }

    #[test]
    fn test_run() {
        let socket_file_path = Path::new("/tmp/ruroco/ruroco.socket");
        let _ = fs::remove_file(socket_file_path);

        let mut commands = HashMap::new();
        commands.insert("default".to_string(), "touch /tmp/ruroco_test.test".to_string());
        thread::spawn(move || {
            create_commander(commands, PathBuf::from("/tmp/ruroco"))
                .run()
                .expect("commander terminated")
        });

        thread::sleep(Duration::from_secs(1));
        assert!(socket_file_path.exists());
    }

    #[test]
    fn test_create_with_empty_commands() {
        let commander = create_commander(HashMap::new(), PathBuf::from("/tmp/ruroco_test_empty"));
        assert!(commander.cmds.is_empty());
    }

    #[test]
    fn test_create_with_multiple_commands() {
        let mut commands = HashMap::new();
        commands.insert("cmd1".to_string(), "echo 1".to_string());
        commands.insert("cmd2".to_string(), "echo 2".to_string());
        assert_eq!(
            create_commander(commands, PathBuf::from("/tmp/ruroco_test_multi")).cmds.len(),
            2
        );
    }

    #[test]
    fn test_run_command_success() {
        create_commander(HashMap::new(), PathBuf::from("/tmp/ruroco_test_cmd"))
            .run_command("echo hello", "127.0.0.1".parse().unwrap());
    }

    #[test]
    fn test_run_command_failure() {
        create_commander(HashMap::new(), PathBuf::from("/tmp/ruroco_test_cmd_fail"))
            .run_command("false", "127.0.0.1".parse().unwrap());
    }

    #[test]
    fn test_run_command_sets_env_var() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("env_output.txt");
        let output_path = output_file.to_str().unwrap();
        create_commander(HashMap::new(), PathBuf::from("/tmp/ruroco_test_env")).run_command(
            &format!("echo $RUROCO_IP > {output_path}"),
            "192.168.1.100".parse().unwrap(),
        );
        thread::sleep(Duration::from_millis(100));
        assert!(fs::read_to_string(&output_file)
            .unwrap_or_default()
            .trim()
            .contains("192.168.1.100"));
    }

    #[test]
    fn test_run_cycle_over_socket() {
        use crate::common::blake2b_u64;

        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("cycle_output.txt");
        let output_path_str = output_file.to_str().unwrap().to_string();

        let cmd_name = "test_cycle_cmd";
        let cmd_hash = blake2b_u64(cmd_name).unwrap();
        let mut commands = HashMap::new();
        commands.insert(cmd_name.to_string(), format!("touch {output_path_str}"));

        let socket_dir = dir.path().to_path_buf();
        let commander = create_commander(commands, socket_dir.clone());
        thread::spawn(move || commander.run());

        let socket_path = socket_dir.join("ruroco.socket");
        wait_for_socket(&socket_path);
        send_to_socket(
            &socket_path,
            CommanderData {
                cmd_hash,
                ip: "127.0.0.1".parse().unwrap(),
            },
        );

        thread::sleep(Duration::from_millis(500));
        assert!(output_file.exists(), "command was not executed");
        let _ = fs::remove_file(&socket_path);
    }

    #[test]
    fn test_run_cycle_unknown_command() {
        let dir = tempfile::tempdir().unwrap();
        let socket_dir = dir.path().to_path_buf();
        let commander = create_commander(HashMap::new(), socket_dir.clone());
        thread::spawn(move || commander.run());

        let socket_path = socket_dir.join("ruroco.socket");
        wait_for_socket(&socket_path);
        send_to_socket(
            &socket_path,
            CommanderData {
                cmd_hash: 99999,
                ip: "127.0.0.1".parse().unwrap(),
            },
        );

        thread::sleep(Duration::from_millis(200));
        assert!(socket_path.exists(), "commander should still be running");
        let _ = fs::remove_file(&socket_path);
    }

    #[test]
    fn test_read_from_stream() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test_read.socket");

        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
        let socket_path_clone = socket_path.clone();
        let writer = thread::spawn(move || {
            send_to_socket(
                &socket_path_clone,
                CommanderData {
                    cmd_hash: 42,
                    ip: "10.0.0.1".parse().unwrap(),
                },
            );
        });

        let (mut stream, _) = listener.accept().unwrap();
        let parsed: CommanderData = Commander::read(&mut stream).unwrap().into();
        assert_eq!(parsed.cmd_hash, 42);
        writer.join().unwrap();
    }

    #[test]
    fn test_create_listener_no_parent_dir() {
        let commander = Commander {
            socket_path: PathBuf::from("/"),
            cmds: HashMap::new(),
            socket_user: String::new(),
            socket_group: String::new(),
        };
        assert!(commander
            .create_listener()
            .unwrap_err()
            .to_string()
            .contains("Could not get parent dir"));
    }
}
