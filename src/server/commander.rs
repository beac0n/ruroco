use crate::common::data::CommanderData;
use crate::common::{error, get_commander_unix_socket_path, info};
use crate::config::config_server::{CliServer, ConfigServer};
use std::fs::Permissions;
use std::io::Read;
use std::os::unix::fs::{chown, PermissionsExt};
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
        let user_name = self.config.socket_user.trim();
        let group_name = self.config.socket_group.trim();

        let user_id = match Commander::get_id_by_name_and_flag(user_name, "-u") {
            Some(id) => Some(id),
            None if user_name.is_empty() => None,
            None => return Err(format!("Could not find user {user_name}")),
        };

        let group_id = match Commander::get_id_by_name_and_flag(group_name, "-g") {
            Some(id) => Some(id),
            None if group_name.is_empty() => None,
            None => return Err(format!("Could not find group {group_name}")),
        };

        chown(&self.socket_path, user_id, group_id).map_err(|e| {
            format!(
                "Could not change ownership of {:?} to {user_id:?}:{group_id:?}: {e}",
                self.socket_path
            )
        })?;
        Ok(())
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

    fn get_id_by_name_and_flag(name: &str, flag: &str) -> Option<u32> {
        if name.is_empty() {
            return None;
        }

        match Command::new("id").arg(flag).arg(name).output() {
            Ok(output) => match String::from_utf8_lossy(&output.stdout).trim().parse::<u32>() {
                Ok(uid) => Some(uid),
                Err(e) => {
                    error(&format!(
                        "Error parsing id from id command output: {} {} {e}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                    None
                }
            },
            Err(e) => {
                error(&format!("Error getting id via id command: {e}"));
                None
            }
        }
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
    use crate::server::commander::Commander;

    #[test]
    fn test_get_id_by_name_and_flag() {
        assert_eq!(Commander::get_id_by_name_and_flag("root", "-u"), Some(0));
        assert_eq!(Commander::get_id_by_name_and_flag("root", "-g"), Some(0));
    }

    #[test]
    fn test_get_id_by_name_and_flag_unknown_user() {
        assert_eq!(Commander::get_id_by_name_and_flag("barfoobaz", "-u"), None);
        assert_eq!(Commander::get_id_by_name_and_flag("barfoobaz", "-g"), None);
    }
}
