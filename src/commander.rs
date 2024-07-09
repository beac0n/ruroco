use std::{fs, str};
use std::collections::HashMap;
use std::fs::Permissions;
use std::io::Read;
use std::os::unix::fs::{chown, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;

use log::{error, info, warn};
use users::{get_group_by_name, get_user_by_name};

use crate::common::get_socket_path;

pub struct Commander {
    config: HashMap<String, String>,
    socket_group: String,
    socket_user: String,
    socket_path: PathBuf,
}

impl Commander {
    pub fn create(
        config: HashMap<String, String>,
        socket_user: String,
        socket_group: String,
        config_dir: PathBuf,
    ) -> Commander {
        Commander {
            config,
            socket_user,
            socket_group,
            socket_path: get_socket_path(&config_dir),
        }
    }

    pub fn run(&self) -> Result<(), String> {
        for stream in self.create_listener()?.incoming() {
            match stream {
                Ok(mut stream) => match Self::read_string(&mut stream) {
                    Ok(msg) => self.run_cycle(msg),
                    Err(e) => error!("{e}"),
                },
                Err(e) => error!("Connection for {:?} failed: {e}", &self.socket_path),
            }
        }

        let _ = fs::remove_file(&self.socket_path);
        Ok(())
    }

    fn create_listener(&self) -> Result<UnixListener, String> {
        let socket_dir = match self.socket_path.parent() {
            Some(socket_dir) => socket_dir,
            None => {
                return Err(format!("Could not get parent dir for {:?}", &self.socket_path).into())
            }
        };
        fs::create_dir_all(&socket_dir)
            .map_err(|e| format!("Could not create parents for {socket_dir:?}: {e}"))?;

        let _ = fs::remove_file(&self.socket_path);

        let mode = 0o204; // only server should be able to write, everyone else can read
        info!("Binding Unix Listener on {:?} with permissions {mode:o}", &self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| format!("Could not bind to socket {:?}: {e}", self.socket_path))?;

        fs::set_permissions(&self.socket_path, Permissions::from_mode(mode)).map_err(|e| {
            format!("Could not set permissions {mode:o} for {:?}: {e}", self.socket_path)
        })?;
        self.change_socket_ownership()?;

        Ok(listener)
    }

    fn change_socket_ownership(&self) -> Result<(), String> {
        let user_name = self.socket_user.trim();
        let group_name = self.socket_group.trim();

        let user_id = match get_user_by_name(user_name) {
            Some(user) => Some(user.uid()),
            None if user_name == "" => None,
            None => return Err(format!("Could not find user {user_name}").into()),
        };

        let group_id = match get_group_by_name(group_name) {
            Some(group) => Some(group.gid()),
            None if group_name == "" => None,
            None => return Err(format!("Could not find group {group_name}").into()),
        };

        chown(&self.socket_path, user_id, group_id).map_err(|e| {
            format!(
                "Could not change ownership of {:?} to {user_id:?}:{group_id:?}: {e}",
                self.socket_path
            )
        })?;
        Ok(())
    }

    fn read_string(stream: &mut UnixStream) -> Result<String, String> {
        let mut buffer = String::new();
        stream
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Could not read command from Unix Stream to string: {e}"))?;
        return Ok(buffer);
    }

    fn run_cycle(&self, msg: String) {
        match self.config.get(&msg) {
            Some(command) => self.run_command(command),
            None => warn!("Unknown command {msg}"),
        }
    }

    fn run_command(&self, command: &str) {
        info!("Running command {command}");
        match Command::new("sh").arg("-c").arg(command).output() {
            Ok(result) => {
                info!(
                    "Successfully executed {command}\nstdout: {}\nstderr: {}",
                    Self::vec_to_str(&result.stdout),
                    Self::vec_to_str(&result.stderr)
                )
            }
            Err(e) => error!("Error executing {command}: {e}"),
        };
    }

    fn vec_to_str(stdout: &Vec<u8>) -> &str {
        return str::from_utf8(stdout).unwrap_or("");
    }
}
