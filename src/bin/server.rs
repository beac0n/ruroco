use clap::Parser;

use ruroco::common::init_logger;
use ruroco::config_server::CliServer;
use ruroco::server::Server;

fn main() -> Result<(), String> {
    init_logger();
    Server::create_from_path(CliServer::parse().config)?.run()
}
