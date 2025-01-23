use crate::node::Node;

mod block;
mod blockchain;
mod client;
mod consensus;
mod miner;
mod network;
mod node;
mod server;
mod tests;
mod wallet;

#[tokio::main]
async fn main() {
    let node = Node::new();
    node.start(String::from("127.0.0.1:8080")).await;
}
