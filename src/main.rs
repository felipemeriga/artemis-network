#[macro_use]
extern crate log;

mod block;
mod blockchain;
mod broadcaster;
mod client;
mod logger;
mod miner;
mod node;
mod server;
mod tests;
mod wallet;

use crate::logger::init_logger;
use crate::node::Node;

#[tokio::main]
async fn main() {
    init_logger();
    let node = Node::new();
    node.start(String::from("127.0.0.1:8080")).await;
}
