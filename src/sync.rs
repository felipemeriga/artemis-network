use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::db::Database;
use crate::server::Request;
use crate::sync_info;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock};

pub struct Sync {
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<HashSet<String>>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
    database: Arc<Mutex<Database>>,
}

impl Sync {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        peers: Arc<Mutex<HashSet<String>>>,
        watch_tx: Arc<Mutex<Sender<Option<Block>>>>,
        database: Arc<Mutex<Database>>,
    ) -> Self {
        Self {
            blockchain,
            peers,
            block_tx: watch_tx,
            database,
        }
    }

    pub async fn sync_with_peers(
        &mut self,
        tcp_address: String,
        first_discover_done: Arc<Mutex<bool>>,
        first_sync_done: Arc<Mutex<bool>>,
    ) {
        loop {
            {
                if !*first_discover_done.lock().await {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            }

            let peers = { self.peers.lock().await.clone() };
            let mut longest_chain = None;
            let mut max_length = self.blockchain.read().await.get_chain().len();

            for peer_address in peers {
                if peer_address == tcp_address {
                    continue;
                }
                if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
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

                    let mut buffer = [0; 8184];
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
                self.blockchain
                    .write()
                    .await
                    .replace_chain(new_chain.clone());
                // notify miners that a new chain has been found
                self.block_tx
                    .lock()
                    .await
                    .send(Some(self.blockchain.read().await.get_last_block().clone()))
                    .await
                    .expect("could not send message");
                sync_info!("Saving the copy of the blockchain from peer, into the DB");
                {
                    if self
                        .database
                        .lock()
                        .await
                        .store_blocks_and_transactions(new_chain.clone())
                        .is_err()
                    {
                        panic!("Unable to store the copy of the blockchain from peer, into the DB")
                    }
                }
            } else {
                sync_info!("Local chain is the longest.");
            }
            {
                if !*first_sync_done.lock().await {
                    *first_sync_done.lock().await = true;
                }
            }
            // Sleep for some time before the next sync
            tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
        }
    }
}
