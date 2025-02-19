use crate::block::Block;
use crate::server::Request;
use crate::transaction::Transaction;
use crate::{broadcaster_error, broadcaster_info};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct Broadcaster {
    peers: Arc<Mutex<HashSet<String>>>,
    tcp_address: String,
}

impl Broadcaster {
    pub fn new(peers: Arc<Mutex<HashSet<String>>>, tcp_address: String) -> Self {
        Self { peers, tcp_address }
    }

    pub async fn broadcast_new_block(&self, block: &Block) {
        broadcaster_info!("broadcasting new block to peers");
        let peers_list = { self.peers.lock().await.clone() };
        for peer_address in peers_list {
            if peer_address == self.tcp_address {
                continue;
            }
            if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                let request = Request {
                    command: "new_block".to_string(),
                    data: serde_json::to_string(&block).unwrap(),
                };

                let serialized_request = serde_json::to_string(&request).unwrap();
                if let Err(e) = stream.write_all(serialized_request.as_bytes()).await {
                    broadcaster_error!("Failed to send block to {}: {}", peer_address, e);
                }
            } else {
                {
                    // In the case the node can't connect to that peer, it will remove from the list
                    self.peers.lock().await.remove(&peer_address);
                }
            }
        }
    }

    pub async fn broadcast_transaction(&self, transaction: Transaction) {
        broadcaster_info!("broadcasting new transaction to peers");
        let peers_list = { self.peers.lock().await.clone() };
        for peer_address in peers_list {
            if peer_address == self.tcp_address {
                continue;
            }
            if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                let request = Request {
                    command: "transaction".to_string(),
                    data: serde_json::to_string(&transaction).unwrap(),
                };

                let serialized_request = serde_json::to_string(&request).unwrap();
                if let Err(e) = stream.write_all(serialized_request.as_bytes()).await {
                    broadcaster_error!("Failed to send transaction to {}: {}", peer_address, e);
                }
            } else {
                {
                    // In the case the node can't connect to that peer, it will remove from the list
                    self.peers.lock().await.remove(&peer_address);
                }
            }
        }
    }
}
