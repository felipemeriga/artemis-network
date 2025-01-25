use crate::blockchain::Blockchain;
use crate::client::Client;
use crate::miner::mine;
use crate::server;
use crate::server::Server;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc::channel};
use crate::block::Block;

pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>, // List of peers (IP:PORT)
}

impl Node {
    pub fn new() -> Self {
        Node {
            blockchain: Arc::new(RwLock::new(Blockchain::new())),
            peers: Arc::new(Mutex::new(vec!["127.0.0.1:8080".parse().unwrap()])),
        }
    }

    pub async fn start(&self, address: String) {
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();

        let (block_tx, block_rx) = channel::<Option<Block>>(20);

        let tx = Arc::new(Mutex::new(block_tx));
        let server_tx = tx.clone();
        let server_address = address.clone();
        let mut server = Server::new(blockchain, server_address, peers, server_tx);
        // Spawn server task
        let server_handle = tokio::spawn(async move {
            server.run_server().await;
        });

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();
        let client_tx = tx.clone();

        let mut client = Client::new(blockchain, peers, client_tx);
        let client_handle = tokio::spawn(async move {
            client.sync_with_peers().await;
        });

        let blockchain = self.blockchain.clone();
        // Spawn a task for mining new blocks
        let miner_handle = tokio::spawn(async move {
            mine(blockchain, block_rx).await;
        });

        println!("Node started at {}", address);

        // Wait for both client and server to finish
        let _ = tokio::try_join!(server_handle, client_handle, miner_handle);
    }
}
