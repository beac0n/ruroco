use std::error::Error;
use std::path::PathBuf;
use std::str;

use clap::Parser;

use ruroco::common::init_logger;
use ruroco::server::Server;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'a', long, default_value_t = String::from("127.0.0.1:8080"))]
    address: String,
    #[arg(short = 'p', long, default_value = PathBuf::from("ruroco_public.pem").into_os_string())]
    pem_path: PathBuf,
    #[arg(short = 'd', long, default_value_t = 5_000_000_000)]
    max_delay: u128,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    let args = Cli::parse();
    Server::create(args.pem_path, args.address, args.max_delay)?.run()
}
