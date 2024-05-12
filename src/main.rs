use std::error::Error;
use std::path::PathBuf;
use std::str;
use clap::{Parser, Subcommand};

mod client;
mod server;
mod util;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Gen {},
    Server {
        #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
        address: String,
        #[arg(short, long, default_value = util::get_default_pem_public().into_os_string())]
        pem_path: PathBuf,
    },
    Client {
        #[arg(short, long, default_value_t = String::from("127.0.0.1:8080"))]
        address: String,
        #[arg(short, long, default_value = util::get_default_pem_private().into_os_string())]
        pem_path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Cli::parse();
    return match args.command {
        Commands::Gen {} => util::gen_pem(),
        Commands::Server { address, pem_path } => server::run(pem_path, address),
        Commands::Client { address, pem_path } => client::run(pem_path, address),
    };
}
