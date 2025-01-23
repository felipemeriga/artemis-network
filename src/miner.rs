use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub async fn mine(blockchain: Arc<RwLock<Blockchain>>) {
    loop {
        // Generate dummy block data
        let data = format!("Block at {}", chrono::Utc::now());

        {
            // Acquire the blockchain lock and mine a new block
            let mut blockchain = blockchain.read().await;
            println!("Mining a new block with data: {}", data);
            blockchain.mine_new_block(data);
        }
        // TODO - After mining the block, create a new scope, and acquire a write lock for pushing the new block

        // Wait some time before mining the next block
        sleep(Duration::from_secs(30)).await; // Adjustable based on network needs
    }
}
