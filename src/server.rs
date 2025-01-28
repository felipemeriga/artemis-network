use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::{server_error, server_info, server_warn};
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
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
) {
    let listener = TcpListener::bind(address.clone()).await.unwrap();

    let sever_handler = ServerHandler {
        blockchain,
        block_tx,
        broadcaster,
    };

    server_info!("Listening on {}", address);
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
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
}

impl ServerHandler {
    pub async fn handle_connection(&self, mut stream: TcpStream) {
        // Get the remote peer's address (hostname and port)
        match stream.peer_addr() {
            Ok(peer_addr) => {
                let hostname = peer_addr.ip().to_string();
                let port = peer_addr.port();
                let peer_addr = format!("{}:{}", hostname, port);
                server_info!("New connection from {}", peer_addr);

                // Handle the connection as usual
                let mut buffer = [0; 1024];
                if let Ok(n) = stream.read(&mut buffer).await {
                    let request: Result<Request, _> = serde_json::from_slice(&buffer[..n]);
                    if let Ok(req) = request {
                        match req.command.as_str() {
                            "new_block" => {
                                if let Ok(block) = serde_json::from_str::<Block>(&req.data) {
                                    self.handle_new_block(block, Some(vec![peer_addr])).await;
                                } else {
                                    server_warn!("Invalid block received")
                                }
                            }
                            "get_blockchain" => {
                                let chain = self.blockchain.read().await.get_chain();
                                let response = serde_json::to_string(&chain).unwrap();
                                let _ = stream.write_all(response.as_bytes()).await;
                            }
                            _ => server_error!("Unknown command: {}", req.command),
                        }
                    } else {
                        server_error!("Failed to parse request.");
                    }
                }
            }
            Err(e) => {
                server_error!("Failed to get remote peer address: {}", e);
            }
        }
    }

    async fn handle_new_block(&self, block: Block, excluded_peers: Option<Vec<String>>) {
        let is_valid_block = {
            // Acquire the write lock and validate the block
            let mut chain = self.blockchain.write().await;
            if chain.is_valid_new_block(&block) {
                server_info!("Appending valid block: {:?}", block);
                chain.add_block(block.clone()); // Append the block
                true // Block is valid
            } else {
                server_warn!("Invalid block received: {:?}", block);
                false // Block is invalid
            }
            // The lock is released here since it goes out of scope
        };

        // Broadcast the block only after releasing the lock
        if is_valid_block {
            // Commenting this step, since we don't have too many nodes working together right now
            // self.broadcaster
            //     .lock()
            //     .await
            //     .broadcast_new_block(&block.clone(), excluded_peers)
            //     .await;
            self.block_tx
                .lock()
                .await
                .send(Some(block.clone()))
                .await
                .expect("TODO: panic message");
        }
    }
}
