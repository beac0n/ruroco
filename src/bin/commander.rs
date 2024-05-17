use std::{fs, str, thread};
use std::error::Error;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::time::Duration;

use clap::Parser;

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
    let args = Cli::parse();
    let start_command = args.start;
    let stop_command = args.stop;
    let sleep_time = args.sleep;

    let socket_dir = "/tmp/ruroco/";
    println!("Creating ruroco socket dir {socket_dir}");
    fs::create_dir_all(socket_dir)?;

    let socket_file_path = socket_dir.to_owned() + "ruroco.socket";
    println!("Removing already existing socket file {socket_file_path}");
    let _ = fs::remove_file(&socket_file_path);

    let mode = 0o600;
    println!("Listing Unix Listener on {socket_file_path} with permissions {mode:o}");
    let listener = UnixListener::bind(&socket_file_path)?;
    fs::set_permissions(&socket_file_path, Permissions::from_mode(mode))?;
    for stream in listener.incoming() {
        match stream {
            Ok(_) => {
                println!("Starting cycle");
                run_command(&start_command);
                println!("Sleeping for {sleep_time} seconds");
                thread::sleep(Duration::from_secs(sleep_time));
                run_command(&stop_command);
                println!("Finished cycle");
            }
            Err(err) => {
                println!("Connection failed: {err}");
            }
        }
    }
    let _ = fs::remove_file(&socket_file_path);
    Ok(())
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
            println!("Successfully executed {command}\nstdout: {stdout}\nstderr: {stderr}")
        }
        Err(err) => {
            println!("Error executing {command} - {err}")
        }
    };
}
