use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::config::Config;
use crate::db::Database;
use crate::discover::Discover;
use crate::miner::Miner;
use crate::pool::TransactionPool;
use crate::server::ServerHandler;
use crate::sync::Sync;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc::channel, Mutex, RwLock};
use uuid::Uuid;

pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            blockchain: Arc::new(RwLock::new(Blockchain::new())),
        }
    }

    pub async fn start(&self, config: Config) {
        let miner_id = Uuid::new_v4().to_string(); // Unique ID for this miner
        let blockchain = self.blockchain.clone();
        let mut peers_set = HashSet::new();
        peers_set.insert(config.tcp_address.clone());
        let peers = Arc::new(Mutex::new(peers_set));
        let database = Arc::new(Mutex::new(Database::new()));

        let (block_tx, block_rx) = channel::<Option<Block>>(20);

        let tx = Arc::new(Mutex::new(block_tx));
        let server_tx = tx.clone();
        let broadcaster = Arc::new(Mutex::new(Broadcaster::new(
            peers.clone(),
            config.tcp_address.clone(),
        )));
        let transaction_pool = Arc::new(Mutex::new(TransactionPool::new()));

        let server_broadcaster = broadcaster.clone();
        let server_tx_pool = transaction_pool.clone();

        let server = Arc::new(ServerHandler::new(
            blockchain,
            server_tx,
            server_broadcaster,
            server_tx_pool,
            peers.clone(),
            database.clone(),
        ));

        // TCP Server will be used for p2p communication between nodes
        let tcp_server = server.clone();
        // HTTP Server will be used as RPC layer, for client communication with nodes
        let http_server = server.clone();

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let sync_tx = tx.clone();

        // These will be responsible for controlling whether to start a new task
        // When a node has been started, we need to make sure it was capable to find peers,
        // and that he could get the latest state of the Blockchain, only after that we can start
        // mining
        let first_discover_done = Arc::new(Mutex::new(false));
        let first_sync_done = Arc::new(Mutex::new(false));

        let mut sync = Sync::new(blockchain, peers.clone(), sync_tx);

        let blockchain = self.blockchain.clone();
        let miner_broadcaster = broadcaster.clone();
        let miner_tx_pool = transaction_pool.clone();
        let mut miner = Miner::new(
            blockchain,
            miner_broadcaster,
            block_rx,
            miner_tx_pool,
            database.clone(),
            true,
            1,
        );

        let mut discover = None;

        if config.bootstrap_address.is_some() {
            let peers = peers.clone();
            discover = Some(Discover::new(peers));
        } else {
            {
                *first_discover_done.lock().await = true;
            }
        }

        // Run everything concurrently
        let _ = tokio::join!(
            async {
                tcp_server
                    .start_tcp_server(config.tcp_address.clone())
                    .await
                    .unwrap();
            },
            async {
                http_server
                    .start_http_server(config.http_address)
                    .await
                    .unwrap();
            },
            async {
                if let Some(mut dsc) = discover {
                    dsc.find_peers(
                        miner_id,
                        config.tcp_address.clone(),
                        config.bootstrap_address.unwrap(),
                        first_discover_done.clone(),
                    )
                    .await;
                }
            },
            async {
                sync.sync_with_peers(
                    config.tcp_address.clone(),
                    first_discover_done.clone(),
                    first_sync_done.clone(),
                )
                .await;
            },
            async {
                miner.mine(first_sync_done.clone()).await;
            }
        );
    }
}

// fn validate_ip_with_port(s: &str) -> Result<SocketAddr, std::net::AddrParseError> {
//     s.parse()
// }
