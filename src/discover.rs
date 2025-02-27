use crate::{discover_error, discover_info};
use crate::server::Request;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Peer {
    pub(crate) id: String,
    pub(crate) address: String,
}

pub struct Discover {
    peers: Arc<Mutex<HashSet<String>>>,
}

impl Discover {
    pub fn new(peers: Arc<Mutex<HashSet<String>>>) -> Self {
        Self { peers }
    }

    pub async fn find_peers(
        &mut self,
        node_id: String,
        tcp_address: String,
        first_discover_done: Arc<Mutex<bool>>,
    ) {
        // First 3-seconds sleep
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        loop {
            discover_info!("Looking for discovering new peers");
            let peers = { self.peers.lock().await.clone() };

            let mut receive_one_response = false;
            for peer_address in peers {
                if peer_address == tcp_address {
                    continue;
                }
                if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                    let this_peer = Peer {
                        id: node_id.clone(),
                        address: tcp_address.clone(),
                    };
                    
                    let data = match serde_json::to_string(&this_peer){
                        Ok(result) => result,
                        Err(err) => {
                            discover_error!("failed to serialize peer data: {}", err);
                            continue;
                        }
                    };

                    // Send request to register itself in the bootstrap node
                    let request = Request {
                        command: "register".to_string(),
                        data,
                    };

                    let marshalled_request = match serde_json::to_string(&request){
                        Ok(result) => result,
                        Err(err) => {
                            discover_error!("failed to serialize request: {}", err);
                            continue;
                        }
                    };

                    if stream
                        .write_all(marshalled_request.as_bytes())
                        .await
                        .is_err()
                    {
                        continue;
                    }

                    let mut buffer = [0; 1024];
                    if let Ok(n) = stream.read(&mut buffer).await {
                        let data = String::from_utf8_lossy(&buffer[..n]);
                        if let Ok(remote_peers) = serde_json::from_str::<HashSet<String>>(&data) {
                            receive_one_response = true;
                            for address in remote_peers {
                                if address != tcp_address {
                                    {
                                        let mut peers = self.peers.lock().await;
                                        if !peers.contains(&address.clone()) {
                                            discover_info!(
                                                "New peer discovered on address: {}",
                                                address
                                            );
                                            peers.insert(address);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    discover_error!("Failed to connect to peer: {}", peer_address);
                    {
                        // In the case the node can't connect to that peer, it will remove from the list
                        self.peers.lock().await.remove(&peer_address);
                    }
                }
                if receive_one_response {
                    break;
                }
            }

            // Using a mutex for letting other tasks aware that this process
            // executed at least once
            {
                if !*first_discover_done.lock().await {
                    *first_discover_done.lock().await = true;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
