use clap::Parser;

use ruroco::commander::Commander;
use ruroco::config_server::CliServer;

fn main() -> Result<(), String> {
    Commander::create_from_path(CliServer::parse().config)?.run()
}
