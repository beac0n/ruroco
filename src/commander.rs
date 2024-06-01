use std::{fs, io, str, thread};
use std::collections::HashMap;
use std::error::Error;
use std::fs::Permissions;
use std::io::Read;
use std::os::unix::fs::{chown, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use log::{error, info, warn};
use serde::Deserialize;
use users::{get_group_by_name, get_user_by_name};

use crate::common::{SOCKET_DIR, socket_file_path};

#[derive(Debug, Deserialize)]
pub struct CommanderCommand {
    #[serde(default = "default_start")]
    start: String,
    #[serde(default = "default_stop")]
    stop: String,
    #[serde(default = "default_sleep")]
    sleep: u64,
}

fn default_start() -> String {
    String::from("echo 'start'")
}

fn default_stop() -> String {
    String::from("echo 'stop'")
}

fn default_sleep() -> u64 {
    5
}

impl CommanderCommand {
    pub fn create(start: String, stop: String, sleep: u64) -> CommanderCommand {
        CommanderCommand { start, stop, sleep }
    }
}

pub struct Commander {
    config: HashMap<String, CommanderCommand>,
}

impl Commander {
    pub fn create(config: HashMap<String, CommanderCommand>) -> Commander {
        Commander { config }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        for stream in Self::create_listener()?.incoming() {
            match stream {
                Ok(mut stream) => match Self::read_string(&mut stream) {
                    Ok(msg) => self.run_cycle(msg),
                    Err(e) => error!("Failed to read command message: {e}"),
                },
                Err(e) => error!("Connection for {} failed: {e}", socket_file_path()),
            }
        }

        let _ = fs::remove_file(socket_file_path());
        Ok(())
    }

    fn create_listener() -> Result<UnixListener, Box<dyn Error>> {
        info!("Creating ruroco socket dir {SOCKET_DIR}");
        fs::create_dir_all(SOCKET_DIR)?;

        info!("Removing already existing socket file {}", socket_file_path());
        let socket_file_path = socket_file_path();
        let _ = fs::remove_file(&socket_file_path);

        let mode = 0o204; // only server should be able to write, everyone else can read
        info!("Binding Unix Listener on {socket_file_path} with permissions {mode:o}");
        let listener = UnixListener::bind(&socket_file_path)?;

        let path = Path::new(&socket_file_path);

        let user = match get_user_by_name("ruroco") {
            Some(user) => user,
            _ => return Err("Could not find user ruroco".to_string().into()),
        };

        let group = match get_group_by_name("ruroco") {
            Some(group) => group,
            _ => return Err("Could not find group ruroco".to_string().into()),
        };

        fs::set_permissions(&socket_file_path, Permissions::from_mode(mode))?;

        chown(path, Some(user.uid()), Some(group.gid()))?;

        Ok(listener)
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
            Err(e) => {
                error!("Error executing {command} - {e}")
            }
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
