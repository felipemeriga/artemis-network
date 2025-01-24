use crate::block::Block;
use crate::blockchain::Blockchain;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch::Sender;
use tokio::sync::{Mutex, RwLock};

pub struct Server {
    address: String,
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
    watch_tx: Arc<Mutex<Sender<Option<Block>>>>,
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub command: String,
    pub data: String, // This can be serialized block data, blockchain data, etc.
}

impl Server {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        address: String,
        peers: Arc<Mutex<Vec<String>>>,
        watch_tx: Arc<Mutex<Sender<Option<Block>>>>,
    ) -> Self {
        Self {
            address,
            blockchain,
            peers,
            watch_tx,
        }
    }

    pub async fn run_server(&mut self) {
        let listener = TcpListener::bind(self.address.clone()).await.unwrap();
        println!("Server listening on {}", self.address);

        while let Ok((mut stream, _)) = listener.accept().await {
            let blockchain = self.blockchain.clone();
            let peers = self.peers.clone();

            tokio::spawn(async move {
                Server::handle_connection(stream, blockchain, peers).await;
            });
        }
    }

    pub async fn handle_connection(
        mut stream: TcpStream,
        blockchain: Arc<RwLock<Blockchain>>,
        peers: Arc<Mutex<Vec<String>>>,
    ) {
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer).await {
            let request: Result<Request, _> = serde_json::from_slice(&buffer[..n]);
            if let Ok(req) = request {
                match req.command.as_str() {
                    "new_block" => {
                        if let Ok(block) = serde_json::from_str::<Block>(&req.data) {
                            Server::handle_new_block(block, blockchain, peers).await;
                        } else {
                            eprintln!("Invalid block received.");
                        }
                    }
                    "get_blockchain" => {
                        let chain = blockchain.read().await.get_chain();
                        let response = serde_json::to_string(&chain).unwrap();
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                    _ => eprintln!("Unknown command: {}", req.command),
                }
            } else {
                eprintln!("Failed to parse request.");
            }
        }
    }

    pub async fn handle_new_block(
        block: Block,
        blockchain: Arc<RwLock<Blockchain>>,
        peers: Arc<Mutex<Vec<String>>>,
    ) {
        // Validate the received block
            let mut chain = blockchain.write().await;
            if chain.is_valid_new_block(&block) {
                println!("Appending valid block: {:?}", block);
                chain.add_block(block.clone()); // Assume this adds without mining

                // Broadcast the block to peers only if valid
                Server::broadcast_new_block(&block, peers).await;
            } else {
                println!("Invalid block received: {:?}", block);
            }

    }

    async fn broadcast_new_block(block: &Block, peers: Arc<Mutex<Vec<String>>>) {
        let peers_list = peers.lock().await.clone();
        for peer in peers_list {
            if let Ok(mut stream) = TcpStream::connect(&peer).await {
                let request = Request {
                    command: "new_block".to_string(),
                    data: serde_json::to_string(&block).unwrap(),
                };

                let serialized_request = serde_json::to_string(&request).unwrap();
                if let Err(e) = stream.write_all(serialized_request.as_bytes()).await {
                    eprintln!("Failed to send block to {}: {}", peer, e);
                }
            }
        }
    }
}
