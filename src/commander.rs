use std::{fs, io, str, thread};
use std::collections::HashMap;
use std::error::Error;
use std::fs::Permissions;
use std::io::Read;
use std::os::unix::fs::{chown, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Duration;

use log::{error, info, warn};
use users::{get_group_by_name, get_user_by_name};

use crate::config::CommanderCommand;

pub struct Commander {
    config: HashMap<String, CommanderCommand>,
    socket_group: String,
    socket_user: String,
    socket_path: PathBuf,
}

impl Commander {
    pub fn create(
        config: HashMap<String, CommanderCommand>,
        socket_user: String,
        socket_group: String,
        socket_path: PathBuf,
    ) -> Commander {
        Commander {
            config,
            socket_user,
            socket_group,
            socket_path,
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        for stream in self.create_listener()?.incoming() {
            match stream {
                Ok(mut stream) => match Self::read_string(&mut stream) {
                    Ok(msg) => self.run_cycle(msg),
                    Err(e) => error!("Failed to read command message: {e}"),
                },
                Err(e) => error!("Connection for {:?} failed: {e}", &self.socket_path),
            }
        }

        let _ = fs::remove_file(&self.socket_path);
        Ok(())
    }

    fn create_listener(&self) -> Result<UnixListener, Box<dyn Error>> {
        let socket_dir = match self.socket_path.parent() {
            Some(socket_dir) => socket_dir,
            _ => return Err(format!("Could not get parent dir for {:?}", &self.socket_path).into()),
        };
        info!("Creating ruroco socket dir {socket_dir:?}");
        fs::create_dir_all(socket_dir)?;

        info!("Removing already existing socket file {:?}", &self.socket_path);
        let _ = fs::remove_file(&self.socket_path);

        let mode = 0o204; // only server should be able to write, everyone else can read
        info!("Binding Unix Listener on {:?} with permissions {mode:o}", &self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)?;

        fs::set_permissions(&self.socket_path, Permissions::from_mode(mode))?;
        self.change_socket_ownership()?;

        Ok(listener)
    }

    fn change_socket_ownership(&self) -> Result<(), Box<dyn Error>> {
        let user_name = self.socket_user.trim();
        let group_name = self.socket_group.trim();

        let user_id = match get_user_by_name(user_name) {
            Some(user) => Some(user.uid()),
            _ if user_name == "" => None,
            _ => return Err(format!("Could not find user {user_name}").into()),
        };

        let group_id = match get_group_by_name(group_name) {
            Some(group) => Some(group.gid()),
            _ if group_name == "" => None,
            _ => return Err(format!("Could not find group {group_name}").into()),
        };

        chown(&self.socket_path, user_id, group_id)?;
        Ok(())
    }

    fn read_string(stream: &mut UnixStream) -> Result<String, Box<dyn Error>> {
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer)?;
        return Ok(buffer);
    }

    fn run_cycle(&self, msg: String) {
        match self.config.get(&msg) {
            Some(config) => {
                info!("Starting cycle");
                self.run_command(&config.start);
                info!("Sleeping for {} seconds", config.sleep);
                thread::sleep(Duration::from_secs(config.sleep));
                self.run_command(&config.stop);
                info!("Finished cycle");
            }
            _ => warn!("Unknown command message {msg}"),
        }
    }

    fn run_command(&self, command: &str) {
        match Self::execute_command(command) {
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

    fn execute_command(command: &str) -> io::Result<Output> {
        let split = command.split(' ').collect::<Vec<_>>();
        Command::new(&split[0]).args(&split[1..]).output()
    }
}
