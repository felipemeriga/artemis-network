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
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer).await {
            let request: Result<Request, _> = serde_json::from_slice(&buffer[..n]);
            if let Ok(req) = request {
                match req.command.as_str() {
                    "new_block" => {
                        if let Ok(block) = serde_json::from_str::<Block>(&req.data) {
                            let latest_block =
                                { self.blockchain.read().await.get_last_block().clone() };
                            // Checking if the received block, has already been received by this node
                            // avoiding extra checks, and broadcasting it again.
                            if latest_block.index >= block.index || latest_block.hash == block.hash
                            {
                                return;
                            }
                            self.handle_new_block(block).await;
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

    async fn handle_new_block(&self, block: Block) {
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
            self.broadcaster
                .lock()
                .await
                .broadcast_new_block(&block.clone())
                .await;
            self.block_tx
                .lock()
                .await
                .send(Some(block.clone()))
                .await
                .expect("TODO: panic message");
        }
    }
}
