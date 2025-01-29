#[macro_use]
extern crate log;

mod block;
mod blockchain;
mod broadcaster;
mod logger;
mod miner;
mod node;
mod server;
mod sync;
mod tests;
mod transaction;
mod wallet;

use crate::logger::init_logger;
use crate::node::Node;
use clap::Parser;

/// Struct to define CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The hostname and port to run the application (e.g., 127.0.0.1:8080)
    #[arg(short, long)]
    bind: String,

    /// List of peer nodes (comma-separated, e.g., 127.0.0.1:8333,192.168.1.1:8333)
    #[arg(short, long, default_value = "")]
    peers: String,
}

#[tokio::main]
async fn main() {
    init_logger();

    // Parse command-line arguments
    let args = Args::parse();

    // Extract the bind address (host and port)
    let bind_addr = args.bind;

    // Extract peers
    let peers: Vec<String> = args
        .peers
        .split(',')
        .filter(|peer| !peer.is_empty()) // Remove empty strings from split
        .map(|peer| peer.to_string())
        .collect();

    let node = Node::new(peers);
    node.start(bind_addr).await;
}
