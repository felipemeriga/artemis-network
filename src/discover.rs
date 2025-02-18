use crate::discover_info;
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
        boostrap_address: String,
        first_discover_done: Arc<Mutex<bool>>
    ) {
        loop {
            discover_info!("Looking for peers on bootstrap node");
            if let Ok(mut stream) = TcpStream::connect(&boostrap_address).await {
                let this_peer = Peer {
                    id: node_id.clone(),
                    address: tcp_address.clone(),
                };

                // Send request to register itself in the bootstrap node
                let request = Request {
                    command: "register".to_string(),
                    data: serde_json::to_string(&this_peer).unwrap(),
                };

                let marshalled_request = serde_json::to_string(&request).unwrap();

                // TODO - Add fatal error for connecting to invalid bootstrap node
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
            }

            
            // Using a mutex for letting other tasks aware that this process
            // executed at least once
            {
                if !*first_discover_done.lock().await {
                    *first_discover_done.lock().await = true;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
        }
    }
}
