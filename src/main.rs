use crate::node::Node;

mod block;
mod blockchain;
mod network;
mod wallet;
mod consensus;
mod server;
mod client;
mod tests;
mod node;

#[tokio::main]
async fn main() {
    let node = Node::new();
    node.start(String::from("127.0.0.1:8080")).await;
}
