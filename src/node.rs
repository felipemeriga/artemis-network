use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::discover::Discover;
use crate::miner::Miner;
use crate::pool::TransactionPool;
use crate::server::ServerHandler;
use crate::sync::Sync;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc::channel, Mutex, RwLock};
use uuid::Uuid;
use crate::db::Database;

pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            blockchain: Arc::new(RwLock::new(Blockchain::new())),
        }
    }

    pub async fn start(
        &self,
        tcp_address: String,
        http_address: String,
        bootstrap_address: String,
    ) {
        let miner_id = Uuid::new_v4().to_string(); // Unique ID for this miner
        let blockchain = self.blockchain.clone();
        let mut peers_set = HashSet::new();
        peers_set.insert(tcp_address.clone());
        let peers = Arc::new(Mutex::new(peers_set));
        let database = Arc::new(Mutex::new(Database::new()));

        let (block_tx, block_rx) = channel::<Option<Block>>(20);

        let tx = Arc::new(Mutex::new(block_tx));
        let server_tx = tx.clone();
        let broadcaster = Arc::new(Mutex::new(Broadcaster::new(
            peers.clone(),
            tcp_address.clone(),
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
        ));

        // TCP Server will be used for p2p communication between nodes
        let tcp_server = server.clone();
        // HTTP Server will be used as RPC layer, for client communication with nodes
        let http_server = server.clone();

        // Spawn a client task for syncing
        let blockchain = self.blockchain.clone();
        let sync_tx = tx.clone();

        let mut sync = Sync::new(blockchain, peers.clone(), sync_tx);

        let blockchain = self.blockchain.clone();
        let miner_broadcaster = broadcaster.clone();
        let miner_tx_pool = transaction_pool.clone();
        let mut miner = Miner::new(
            blockchain,
            miner_broadcaster,
            block_rx,
            miner_tx_pool,
            database,
            true,
            1,
        );

        let mut discover = None;

        if !bootstrap_address.is_empty() {
            let peers = peers.clone();
            discover = Some(Discover::new(peers));
        }

        // Run everything concurrently
        let _ = tokio::join!(
            async {
                if let Some(mut dsc) = discover {
                    dsc.find_peers(miner_id, tcp_address.clone(), bootstrap_address)
                        .await;
                }
            },
            async {
                tcp_server
                    .start_tcp_server(tcp_address.clone())
                    .await
                    .unwrap();
            },
            async {
                http_server.start_http_server(http_address).await.unwrap();
            },
            async {
                sync.sync_with_peers(tcp_address.clone()).await;
            },
            async {
                miner.mine().await;
            }
        );
    }
}

// fn validate_ip_with_port(s: &str) -> Result<SocketAddr, std::net::AddrParseError> {
//     s.parse()
// }
