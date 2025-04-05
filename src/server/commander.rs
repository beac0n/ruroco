use crate::common::data::CommanderData;
use crate::common::{change_file_ownership, error, get_commander_unix_socket_path, info};
use crate::config::config_server::{CliServer, ConfigServer};
use std::fs::Permissions;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, str};

const ENV_PREFIX: &str = "RUROCO_";

#[derive(Debug, PartialEq)]
pub struct Commander {
    config: ConfigServer,
    socket_path: PathBuf,
}

impl Commander {
    pub fn create_from_path(path: &Path) -> Result<Commander, String> {
        match fs::read_to_string(path) {
            Ok(config) => Ok(Commander::create(ConfigServer::deserialize(&config)?)),
            Err(e) => Err(format!("Could not read {path:?}: {e}")),
        }
    }

    pub fn create(config: ConfigServer) -> Commander {
        Commander {
            socket_path: get_commander_unix_socket_path(&config.config_dir),
            config,
        }
    }

    pub fn run(&self) -> Result<(), String> {
        for stream in self.create_listener()?.incoming() {
            match stream {
                Ok(mut stream) => {
                    if let Err(e) = self.run_cycle(&mut stream) {
                        error(&e)
                    }
                }
                Err(e) => error(&format!("Connection for {:?} failed: {e}", &self.socket_path)),
            }
        }

        let _ = fs::remove_file(&self.socket_path);
        Ok(())
    }

    fn create_listener(&self) -> Result<UnixListener, String> {
        let socket_dir = match self.socket_path.parent() {
            Some(socket_dir) => socket_dir,
            None => return Err(format!("Could not get parent dir for {:?}", &self.socket_path)),
        };
        fs::create_dir_all(socket_dir)
            .map_err(|e| format!("Could not create parents for {socket_dir:?}: {e}"))?;

        let _ = fs::remove_file(&self.socket_path);

        let mode = 0o204; // only server should be able to write, everyone else can read
        info(&format!(
            "Binding Unix Listener on {:?} with permissions {mode:o}",
            &self.socket_path
        ));
        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| format!("Could not bind to socket {:?}: {e}", self.socket_path))?;

        fs::set_permissions(&self.socket_path, Permissions::from_mode(mode)).map_err(|e| {
            format!("Could not set permissions {mode:o} for {:?}: {e}", self.socket_path)
        })?;
        self.change_socket_ownership()?;

        Ok(listener)
    }

    fn change_socket_ownership(&self) -> Result<(), String> {
        change_file_ownership(
            &self.socket_path,
            self.config.socket_user.trim(),
            self.config.socket_group.trim(),
        )
    }

    fn run_cycle(&self, stream: &mut UnixStream) -> Result<(), String> {
        let msg = Commander::read_string(stream)?;
        let commander_data: CommanderData = CommanderData::deserialize(&msg)?;
        let command_name = &commander_data.command_name;
        let command = self
            .config
            .commands
            .get(command_name)
            .ok_or(format!("Unknown command name: {}", command_name))?;

        self.run_command(command, commander_data.ip);
        Ok(())
    }

    fn run_command(&self, command: &str, ip_str: String) {
        info(&format!("Running command {command}"));
        match Command::new("sh")
            .arg("-c")
            .arg(command)
            .env(format!("{ENV_PREFIX}IP"), ip_str)
            .output()
        {
            Ok(result) => info(&format!(
                "Successfully executed {command}\nstdout: {}\nstderr: {}",
                Commander::vec_to_str(&result.stdout),
                Commander::vec_to_str(&result.stderr)
            )),
            Err(e) => error(&format!("Error executing {command}: {e}")),
        };
    }

    fn read_string(stream: &mut UnixStream) -> Result<String, String> {
        let mut buffer = String::new();
        stream
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Could not read command from Unix Stream to string: {e}"))?;
        Ok(buffer)
    }

    fn vec_to_str(stdout: &[u8]) -> &str {
        str::from_utf8(stdout).unwrap_or("")
    }
}

pub fn run_commander(server: CliServer) -> Result<(), String> {
    Commander::create_from_path(&server.config)?.run()
}

#[cfg(test)]
mod tests {
    use crate::config::config_server::ConfigServer;
    use crate::server::commander::Commander;
    use rand::distr::{Alphanumeric, SampleString};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{env, fs, thread};

    fn gen_file_name(suffix: &str) -> String {
        let rand_str = Alphanumeric.sample_string(&mut rand::rng(), 16);
        format!("{rand_str}{suffix}")
    }

    #[test]
    fn test_create_from_invalid_path() {
        let path = env::current_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("tests")
            .join("files")
            .join("config_invalid.toml");

        let result = Commander::create_from_path(&path);

        assert!(result.is_err());

        assert!(result.err().unwrap().contains("TOML parse error at line 1, column 1"));
    }

    #[test]
    fn test_create_from_invalid_toml_path() {
        let result = Commander::create_from_path(&PathBuf::from("/tmp/path/does/not/exist"));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
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

        let path = env::current_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("tests")
            .join("files")
            .join("config.toml");

        assert_eq!(
            Commander::create_from_path(&path),
            Ok(Commander::create(ConfigServer {
                ips: vec!["127.0.0.1".to_string()],
                ntp: "system".to_string(),
                config_dir: PathBuf::from("tests/conf_dir"),
                socket_user: "ruroco".to_string(),
                socket_group: "ruroco".to_string(),
                commands,
            }))
        );
    }

    #[test]
    fn test_run() {
        let socket_file_path = Path::new("/tmp/ruroco/ruroco.socket");
        let _ = fs::remove_file(socket_file_path);
        assert!(!socket_file_path.exists());

        let mut commands = HashMap::new();
        commands.insert("default".to_string(), format!("touch {}", gen_file_name(".test")));
        thread::spawn(move || {
            Commander::create(ConfigServer {
                commands,
                config_dir: PathBuf::from("/tmp/ruroco"),
                ..Default::default()
            })
            .run()
            .expect("commander terminated")
        });

        thread::sleep(Duration::from_secs(1));

        assert!(socket_file_path.exists());
    }
}
