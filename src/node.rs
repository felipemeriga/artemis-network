use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::miner::Miner;
use crate::pool::TransactionPool;
use crate::server::ServerHandler;
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

    pub async fn start(&self, tcp_address: String, http_address: String) {
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();

        let (block_tx, block_rx) = channel::<Option<Block>>(20);

        let tx = Arc::new(Mutex::new(block_tx));
        let server_tx = tx.clone();
        let broadcaster = Arc::new(Mutex::new(Broadcaster::new(peers)));
        let transaction_pool = Arc::new(Mutex::new(TransactionPool::new()));

        let server_broadcaster = broadcaster.clone();
        let server_tx_pool = transaction_pool.clone();

        let server = Arc::new(ServerHandler::new(
            blockchain,
            server_tx,
            server_broadcaster,
            server_tx_pool,
        ));

        // TCP Server will be used for p2p communication between nodes
        let tcp_server = server.clone();
        // HTTP Server will be used as RPC layer, for client communication with nodes
        let http_server = server.clone();

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let peers = self.peers.clone();
        let sync_tx = tx.clone();

        let mut sync = Sync::new(blockchain, peers, sync_tx);

        let blockchain = self.blockchain.clone();
        let miner_broadcaster = broadcaster.clone();
        let miner_tx_pool = transaction_pool.clone();
        let mut miner = Miner::new(
            blockchain,
            miner_broadcaster,
            block_rx,
            miner_tx_pool,
            true,
            1,
        );

        // Run everything concurrently
        let _ = tokio::join!(
            async {
                tcp_server.start_tcp_server(tcp_address).await.unwrap();
            },
            async {
                http_server.start_http_server(http_address).await.unwrap();
            },
            async {
                sync.sync_with_peers().await;
            },
            async {
                miner.mine().await;
            }
        );
    }
}
