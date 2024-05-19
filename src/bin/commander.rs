use std::{fs, str, thread};
use std::error::Error;
use std::fs::Permissions;
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;
use std::time::Duration;

use clap::Parser;
use log::{error, info, warn};

use ruroco::lib::{init_logger, SOCKET_DIR, SOCKET_FILE_PATH};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'a', long, default_value_t = String::from("/usr/bin/echo -n 'start'"))]
    start: String,
    #[arg(short = 'o', long, default_value_t = String::from("/usr/bin/echo -n 'stop'"))]
    stop: String,
    #[arg(short = 'e', long, default_value_t = 5)]
    sleep: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();

    info!("Creating ruroco socket dir {}", SOCKET_DIR);
    fs::create_dir_all(SOCKET_DIR)?;

    info!("Removing already existing socket file {}", SOCKET_FILE_PATH);
    let _ = fs::remove_file(SOCKET_FILE_PATH);

    let mode = 0o600;
    info!("Listing Unix Listener on {SOCKET_FILE_PATH} with permissions {mode:o}");
    let listener = UnixListener::bind(SOCKET_FILE_PATH)?;
    fs::set_permissions(SOCKET_FILE_PATH, Permissions::from_mode(mode))?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match get_command_msg(&mut stream) {
                Ok(msg) if msg == "default" => {
                    run_command_cycle(&args.start, &args.stop, args.sleep);
                }
                Ok(msg) => {
                    warn!("Unknown command message {msg}");
                }
                Err(e) => {
                    error!("Failed to read command message: {e}")
                }
            },
            Err(e) => {
                error!("Connection failed: {e}");
            }
        }
    }
    let _ = fs::remove_file(SOCKET_FILE_PATH);
    Ok(())
}

fn get_command_msg(stream: &mut UnixStream) -> Result<String, Box<dyn Error>> {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn run_command_cycle(start: &str, stop: &str, sleep: u64) {
    info!("Starting cycle");
    run_command(start);
    info!("Sleeping for {sleep} seconds");
    thread::sleep(Duration::from_secs(sleep));
    run_command(stop);
    info!("Finished cycle");
}

fn run_command(command: &str) {
    let command_split = command.split(' ').collect::<Vec<_>>();
    match Command::new(command_split[0])
        .args(&command_split[1..])
        .output()
    {
        Ok(result) => {
            let stdout = str::from_utf8(&result.stdout).unwrap_or("");
            let stderr = str::from_utf8(&result.stderr).unwrap_or("");
            info!("Successfully executed {command}\nstdout: {stdout}\nstderr: {stderr}")
        }
        Err(e) => {
            error!("Error executing {command} - {e}")
        }
    };
}
