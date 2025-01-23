use tokio::sync::Mutex;
use std::sync::Arc;
use crate::blockchain::Blockchain;
use crate::client::{Client};
use crate::server;
use crate::server::{Server};

pub struct Node {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>, // List of peers (IP:PORT)
}

impl Node {
    pub fn new() -> Self {
        Node {
            blockchain: Arc::new(Mutex::new(Blockchain::new())),
            peers: Arc::new(Mutex::new(vec!["127.0.0.1:8080".parse().unwrap()])),
        }
    }

    pub async fn start(&self, address: String) {
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();

        let server_address = address.clone();
        let mut server = Server::new(blockchain, server_address, peers);
        // Spawn server task
        let server_handle = tokio::spawn(async move {
           server.run_server().await;
        });

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();

        let mut client = Client::new(blockchain, peers);
        let client_handle = tokio::spawn(async move {
            client.sync_with_peers().await;
        });



        println!("Node started at {}", address);

        // Wait for both client and server to finish
        let _ = tokio::try_join!(server_handle, client_handle);
    }
}