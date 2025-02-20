#[macro_use]
extern crate log;

mod block;
mod blockchain;
mod broadcaster;
mod config;
mod db;
mod discover;
mod error;
mod handler;
mod logger;
mod miner;
mod node;
mod pool;
mod server;
mod sync;
mod tests;
mod transaction;
mod utils;
mod wallet;

use crate::config::load_config;
use crate::logger::init_logger;
use crate::node::Node;
use clap::Parser;

/// Struct to define CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The hostname and port to run the application (e.g., 127.0.0.1:8080)
    #[arg(short, long)]
    config: String,
}

#[tokio::main]
async fn main() {
    init_logger();

    // Parse command-line arguments
    let args = Args::parse();

    // Extract the config path from cli arguments
    let config_path = args.config;

    let config = load_config(config_path.as_str()).expect("Failed to load config file.");

    let node = Node::new();
    node.start(config).await;
}
