use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use crate::block::Block;

pub struct Server {
    address: String,
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        address: String,
        peers: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            address,
            blockchain,
            peers,
        }
    }

    pub async fn run_server(&mut self) {
        let listener = TcpListener::bind(self.address.clone()).await.unwrap();
        println!("Server listening on {}", self.address);

        while let Ok((mut socket, _)) = listener.accept().await {
            let blockchain = self.blockchain.clone();
            let peers = self.peers.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 1024];

                let n = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);

                println!("Received request: {}", request.as_ref());

                if request.trim() == "get_blockchain" {
                    let chain = blockchain.read().await.get_chain();
                    let response = serde_json::to_string(&chain).unwrap();
                    socket.write_all(response.as_bytes()).await.unwrap();
                }

                if request.starts_with("add_peer") {
                    let peer = request.trim()[9..].to_string();
                    peers.lock().await.push(peer);
                }
            });
        }
    }

    pub async fn handle_new_block(&mut self, block: Block) {
        // Validate the received block
        let mut chain = self.blockchain.write().await;
        if chain.is_valid_new_block(&block) {
            println!("Appending valid block: {:?}", block);
            chain.add_block(block.clone()); // Assume this adds without mining
        } else {
            println!("Invalid block received: {:?}", block);
        }

        // Broadcast the block to peers
        let peers_list = self.peers.lock().await.clone();
        for peer in peers_list {
            if let Ok(mut stream) = TcpStream::connect(&peer).await {
                let message = serde_json::to_string(&block).unwrap();
                let _ = stream.write_all(message.as_bytes()).await;
            }
        }
    }
}
