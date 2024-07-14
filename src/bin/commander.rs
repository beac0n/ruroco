use clap::Parser;

use ruroco::commander::Commander;
use ruroco::common::init_logger;
use ruroco::config_server::CliServer;

fn main() -> Result<(), String> {
    init_logger();
    Commander::create_from_path(CliServer::parse().config)?.run()
}
