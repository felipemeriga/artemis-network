use crate::constants::{NEW_BLOCK, TRANSACTION};
use crate::server::Request;
use crate::{broadcaster_error, broadcaster_info};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct Broadcaster {
    peers: Arc<Mutex<HashSet<String>>>,
    tcp_address: String,
}

pub enum BroadcastItem<T>
where
    T: Serialize + for<'de> Deserialize<'de>, // Simplified lifetime here
{
    NewBlock(T),
    Transaction(T),
}

impl Broadcaster {
    pub fn new(peers: Arc<Mutex<HashSet<String>>>, tcp_address: String) -> Self {
        Self { peers, tcp_address }
    }

    pub async fn broadcast_item<T>(&self, payload: BroadcastItem<T>)
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let (data, command, header) = match payload {
            BroadcastItem::NewBlock(block) => (block, NEW_BLOCK.to_string(), "block".to_string()),
            BroadcastItem::Transaction(tx) => {
                (tx, TRANSACTION.to_string(), "transaction".to_string())
            }
        };

        broadcaster_info!("broadcasting new {} to peers", header);
        let peers_list = { self.peers.lock().await.clone() };
        for peer_address in peers_list {
            if peer_address == self.tcp_address {
                continue;
            }
            if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                let block_string = match serde_json::to_string(&data) {
                    Ok(result) => result,
                    Err(e) => {
                        broadcaster_error!("Failed to serialize {}: {}", header, e);
                        break;
                    }
                };
                let request = Request {
                    command: command.clone(),
                    data: block_string,
                };

                let serialized_request = match serde_json::to_string(&request) {
                    Ok(result) => result,
                    Err(err) => {
                        broadcaster_error!("Failed to serialize request: {}", err);
                        break;
                    }
                };
                if let Err(e) = stream.write_all(serialized_request.as_bytes()).await {
                    broadcaster_error!("Failed to send {} to {}: {}", header, peer_address, e);
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
