use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::server::Request;
use crate::sync_info;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock};

pub struct Sync {
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
}

impl Sync {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        peers: Arc<Mutex<Vec<String>>>,
        watch_tx: Arc<Mutex<Sender<Option<Block>>>>,
    ) -> Self {
        Self {
            blockchain,
            peers,
            block_tx: watch_tx,
        }
    }

    pub async fn sync_with_peers(&mut self) {
        loop {
            let peers = self.peers.lock().await.clone();
            let mut longest_chain = None;
            let mut max_length = self.blockchain.read().await.get_chain().len();

            for peer in peers {
                if let Ok(mut stream) = TcpStream::connect(&peer).await {
                    let request = Request {
                        command: "get_blockchain".to_string(),
                        data: "".to_string(),
                    };
                    let marshalled_request = serde_json::to_string(&request).unwrap();

                    if stream
                        .write_all(marshalled_request.as_bytes())
                        .await
                        .is_err()
                    {
                        continue;
                    }

                    let mut buffer = [0; 100000];
                    if let Ok(n) = stream.read(&mut buffer).await {
                        let data = String::from_utf8_lossy(&buffer[..n]);
                        if let Ok(peer_chain) = serde_json::from_str::<Vec<Block>>(&data) {
                            if peer_chain.len() > max_length
                                && Blockchain::is_valid_chain(&peer_chain)
                            {
                                max_length = peer_chain.len();
                                longest_chain = Some(peer_chain);
                            }
                        }
                    }
                }
            }

            if let Some(new_chain) = longest_chain {
                sync_info!("Replacing chain with longer chain from peer.");
                self.blockchain.write().await.replace_chain(new_chain);
                // notify miners that a new chain has been found
                self.block_tx
                    .lock()
                    .await
                    .send(Some(self.blockchain.read().await.get_last_block().clone()))
                    .await
                    .expect("could not send message");
            } else {
                sync_info!("Local chain is the longest.");
            }

            // Sleep for some time before the next sync
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}
