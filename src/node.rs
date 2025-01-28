use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::miner::mine;
use crate::node_info;
use crate::server::run_server;
use crate::sync::Sync;
use std::sync::Arc;
use tokio::sync::{mpsc::channel, Mutex, RwLock};

pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>, // List of peers (IP:PORT)
}

impl Node {
    pub fn new(peers: Vec<String>) -> Self {
        Node {
            blockchain: Arc::new(RwLock::new(Blockchain::new())),
            peers: Arc::new(Mutex::new(peers)),
        }
    }

    pub async fn start(&self, address: String) {
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();

        let (block_tx, block_rx) = channel::<Option<Block>>(20);

        let tx = Arc::new(Mutex::new(block_tx));
        let server_tx = tx.clone();
        let server_address = address.clone();
        let broadcaster = Arc::new(Mutex::new(Broadcaster::new(peers)));

        let server_broadcaster = broadcaster.clone();
        // Spawn server task
        let server_handle = tokio::spawn(async move {
            run_server(blockchain, server_address, server_tx, server_broadcaster).await;
        });

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();
        let sync_tx = tx.clone();

        let mut sync = Sync::new(blockchain, peers, sync_tx);
        let sync_handle = tokio::spawn(async move {
            sync.sync_with_peers().await;
        });

        let blockchain = self.blockchain.clone();
        let miner_broadcaster = broadcaster.clone();
        // Spawn a task for mining new blocks
        let miner_handle = tokio::spawn(async move {
            mine(blockchain, miner_broadcaster, block_rx).await;
        });

        node_info!("started at {}", address);

        // Wait for both client and server to finish
        let _ = tokio::try_join!(server_handle, miner_handle, sync_handle);
    }
}
