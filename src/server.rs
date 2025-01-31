use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::handler::{create_wallet, health_check, post_transaction};
use crate::transaction::Transaction;
use crate::{server_error, server_info, server_warn};
use actix_web::{web, App, HttpServer};
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

#[derive(Clone)]
pub struct ServerHandler {
    blockchain: Arc<RwLock<Blockchain>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
}

impl ServerHandler {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        block_tx: Arc<Mutex<Sender<Option<Block>>>>,
        broadcaster: Arc<Mutex<Broadcaster>>,
    ) -> Self {
        Self {
            blockchain,
            block_tx,
            broadcaster,
        }
    }

    /// Starts the Actix Web server for handling HTTP API requests
    pub async fn start_http_server(self: Arc<Self>) -> std::io::Result<()> {
        let handler = self.clone();
        server_info!("HTTP Server listening on 8080");
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(handler.clone()))
                .service(post_transaction)
                .service(health_check)
                .service(create_wallet)
        })
        .bind("127.0.0.1:8080")?
        .run()
        .await
    }

    /// Handles incoming TCP connections, for P2P communication
    pub async fn start_tcp_server(self: Arc<Self>, addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr.clone()).await?;
        server_info!("TCP Server listening on {}", addr);

        while let Ok((stream, _)) = listener.accept().await {
            let handler_clone = self.clone();
            tokio::spawn(async move {
                handler_clone.handle_connection(stream).await;
            });
        }
        Ok(())
    }

    /// Handles new transactions submitted via HTTP
    pub(crate) async fn handle_new_transaction(&self, transaction: Transaction) -> bool {
        true
    }

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
