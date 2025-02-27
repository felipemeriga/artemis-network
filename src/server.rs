use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::{BroadcastItem, Broadcaster};
use crate::constants::{GET_BLOCKCHAIN, NEW_BLOCK, REGISTER, TRANSACTION};
use crate::db::Database;
use crate::discover::Peer;
use crate::handler::{
    create_wallet, get_all_blocks, get_block_by_hash, get_transaction_by_hash,
    get_transactions_by_wallet, get_wallet_balance, health_check, sign_and_submit_transaction,
    sign_transaction, submit_transaction,
};
use crate::pool::TransactionPool;
use crate::transaction::Transaction;
use crate::{server_error, server_info, server_warn};
use actix_web::{web, App, HttpServer};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::collections::HashSet;
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
    pub broadcaster: Arc<Mutex<Broadcaster>>,
    pub transaction_pool: Arc<Mutex<TransactionPool>>,
    pub peers: Arc<Mutex<HashSet<String>>>,
    pub database: Arc<Mutex<Database>>,
}

impl ServerHandler {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        block_tx: Arc<Mutex<Sender<Option<Block>>>>,
        broadcaster: Arc<Mutex<Broadcaster>>,
        transaction_pool: Arc<Mutex<TransactionPool>>,
        peers: Arc<Mutex<HashSet<String>>>,
        database: Arc<Mutex<Database>>,
    ) -> Self {
        Self {
            blockchain,
            block_tx,
            broadcaster,
            transaction_pool,
            peers,
            database,
        }
    }

    /// Starts the Actix Web server for handling HTTP API requests
    pub async fn start_http_server(self: Arc<Self>, http_addr: String) -> std::io::Result<()> {
        let handler = self.clone();
        server_info!("HTTP Server listening on {}", http_addr);
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(handler.clone()))
                .service(submit_transaction)
                .service(health_check)
                .service(create_wallet)
                .service(sign_and_submit_transaction)
                .service(sign_transaction)
                .service(get_transaction_by_hash)
                .service(get_transactions_by_wallet)
                .service(get_block_by_hash)
                .service(get_all_blocks)
                .service(get_wallet_balance)
        })
        .bind(http_addr)?
        .run()
        .await
    }

    /// Handles incoming TCP connections, for P2P communication
    pub async fn start_tcp_server(self: Arc<Self>, tcp_addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(tcp_addr.clone()).await?;
        server_info!("TCP Server listening on {}", tcp_addr);

        while let Ok((stream, _)) = listener.accept().await {
            let handler_clone = self.clone();
            tokio::spawn(async move {
                handler_clone.handle_connection(stream).await;
            });
        }
        Ok(())
    }

    pub async fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer).await {
            let request: Result<Request, _> = serde_json::from_slice(&buffer[..n]);
            if let Ok(req) = request {
                match req.command.as_str() {
                    TRANSACTION => {
                        if let Ok(tx) = serde_json::from_str::<Transaction>(&req.data) {
                            // Inside the function,
                            // there is already a validation,
                            // for avoiding duplicate transactions
                            if !self
                                .transaction_pool
                                .lock()
                                .await
                                .transaction_already_exists(&tx)
                            {
                                self.broadcaster
                                    .lock()
                                    .await
                                    .broadcast_item(BroadcastItem::Transaction(tx.clone()))
                                    .await;
                            };
                            self.transaction_pool.lock().await.add_transaction(tx);
                        } else {
                            server_warn!("Invalid transaction received")
                        }
                    }
                    NEW_BLOCK => {
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
                    GET_BLOCKCHAIN => {
                        let chain = { self.blockchain.read().await.get_chain() };

                        for block in chain {
                            let block_json_string = match to_string(&block) {
                                Ok(result) => result,
                                Err(e) => {
                                    server_error!("Failed to serialize block: {}", e);
                                    break;
                                }
                            };
                            let block_chunk = format!("{}{}\n", block_json_string, "<END_BLOCK>"); // Append delimiter

                            if let Err(e) = stream.write_all(block_chunk.as_bytes()).await {
                                server_error!("Failed to send block: {}", e);
                                break;
                            }

                            if let Err(e) = stream.flush().await {
                                server_error!("Failed to flush stream: {}", e);
                                break;
                            }
                        }
                        // Send a final message indicating completion
                        let _ = stream.write_all(b"<END_CHAIN>\n").await;
                        let _ = stream.flush().await;
                    }
                    REGISTER => {
                        if let Ok(peer) = serde_json::from_str::<Peer>(&req.data) {
                            let peers = {
                                let mut peers_lock = self.peers.lock().await;
                                if !peers_lock.contains(&peer.address) {
                                    server_info!(
                                        "received a new peer in address: {}",
                                        peer.address
                                    );
                                    peers_lock.insert(peer.address);
                                }
                                peers_lock.clone()
                            };
                            let response = match serde_json::to_string(&peers) {
                                Ok(result) => result,
                                Err(e) => {
                                    server_error!("Failed to serialize peers: {}", e);
                                    return;
                                }
                            };

                            let _ = stream.write_all(response.as_bytes()).await;
                        } else {
                            server_warn!("Invalid new peer received")
                        }
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
            self.block_tx
                .lock()
                .await
                .send(Some(block.clone()))
                .await
                .expect("TODO: panic message");
            self.broadcaster
                .lock()
                .await
                .broadcast_item(BroadcastItem::NewBlock(block.clone()))
                .await;
        }
    }
}
