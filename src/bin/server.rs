use clap::Parser;

use ruroco::config_server::CliServer;
use ruroco::server::Server;

fn main() -> Result<(), String> {
    Server::create_from_path(CliServer::parse().config)?.run()
}
