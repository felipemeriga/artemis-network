use crate::block::Block;
use crate::server::Request;
use crate::{broadcaster_error, broadcaster_info};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct Broadcaster {
    peers: Arc<Mutex<Vec<String>>>,
}

impl Broadcaster {
    pub fn new(peers: Arc<Mutex<Vec<String>>>) -> Self {
        Self { peers }
    }

    pub async fn broadcast_new_block(&self, block: &Block, excluded_peers: Option<Vec<String>>) {
        broadcaster_info!("broadcasting new block to peers");
        let peers_list = self.peers.lock().await.clone();
        let excluded_peers = excluded_peers.unwrap_or_default();
        for peer in peers_list {
            if excluded_peers.contains(&peer) {
                broadcaster_info!("Discarding peer {} from broadcasting", peer);
                continue;
            }

            if let Ok(mut stream) = TcpStream::connect(&peer).await {
                let request = Request {
                    command: "new_block".to_string(),
                    data: serde_json::to_string(&block).unwrap(),
                };

                let serialized_request = serde_json::to_string(&request).unwrap();
                if let Err(e) = stream.write_all(serialized_request.as_bytes()).await {
                    broadcaster_error!("Failed to send block to {}: {}", peer, e);
                }
            }
        }
    }
}
