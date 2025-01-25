use crate::block::Block;
use crate::blockchain::Blockchain;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub command: String,
    pub data: String, // This can be serialized block data, blockchain data, etc.
}

pub async fn run_server(
    blockchain: Arc<RwLock<Blockchain>>,
    address: String,
    peers: Arc<Mutex<Vec<String>>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
) {
    let listener = TcpListener::bind(address.clone()).await.unwrap();

    println!("Server listening on {}", address);

    let sever_handler = ServerHandler {
        blockchain,
        peers,
        block_tx,
    };

    while let Ok((stream, _)) = listener.accept().await {
        let handler = sever_handler.clone();
        tokio::spawn(async move {
            handler.handle_connection(stream).await;
        });
    }
}

#[derive(Clone)]
pub struct ServerHandler {
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
}

impl ServerHandler {
    pub async fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer).await {
            let request: Result<Request, _> = serde_json::from_slice(&buffer[..n]);
            if let Ok(req) = request {
                match req.command.as_str() {
                    "new_block" => {
                        if let Ok(block) = serde_json::from_str::<Block>(&req.data) {
                            self.handle_new_block(block).await;
                        } else {
                            eprintln!("Invalid block received.");
                        }
                    }
                    "get_blockchain" => {
                        let chain = self.blockchain.read().await.get_chain();
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

    async fn handle_new_block(&self, block: Block) {
        let is_valid_block = {
            // Acquire the write lock and validate the block
            let mut chain = self.blockchain.write().await;
            if chain.is_valid_new_block(&block) {
                println!("Appending valid block: {:?}", block);
                chain.add_block(block.clone()); // Append the block
                true // Block is valid
            } else {
                println!("Invalid block received: {:?}", block);
                false // Block is invalid
            }
            // The lock is released here since it goes out of scope
        };

        // Broadcast the block only after releasing the lock
        if is_valid_block {
            self.broadcast_new_block(&block).await;
        }
    }

    async fn broadcast_new_block(&self, block: &Block) {
        let peers_list = self.peers.lock().await.clone();
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
