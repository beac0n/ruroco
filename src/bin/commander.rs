use std::error::Error;
use std::str;

use clap::Parser;

use ruroco::commander::Commander;
use ruroco::common::init_logger;

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
    Commander::create(args.start, args.stop, args.sleep).run()
}
