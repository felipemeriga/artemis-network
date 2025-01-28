use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::miner_info;
use std::sync::Arc;
use std::time::Instant;
use tokio::select;
use tokio::sync::{mpsc::Receiver, Mutex, RwLock};
use tokio::time::Duration;

pub async fn mine(
    blockchain: Arc<RwLock<Blockchain>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
    mut block_rx: Receiver<Option<Block>>,
) {
    loop {
        let data = format!("Block at {}", chrono::Utc::now());
        let mut mined_block: Option<Block> = None;

        // Prepare a new block for mining
        let (mut candidate_block, difficulty) = {
            let blockchain_read = blockchain.read().await;
            blockchain_read.prepare_block_for_mining(data.clone())
        };

        miner_info!("Starting mining with difficulty: {}", difficulty);
        let start_time = Instant::now();

        loop {
            // Incrementally mine
            candidate_block.mine_step();

            // Check if the block meets the difficulty
            if candidate_block.is_valid(difficulty) {
                mined_block = Some(candidate_block.clone());
                break;
            }

            select! {
                // If a new block is received from the network
                Some(new_block) = block_rx.recv() => {
                    miner_info!("New block received: {:?}. Restarting mining...", new_block);
                    break; // Exit the mining loop and restart
                }

                // Simulate mining time to let other tasks execute, and adding delay to the process
                // use this, when you need to test concurrent tasks against mining process, without having
                // this process to mine too many blocks
                // _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
        }

        // Commit the mined block if no new block was received
        if let Some(new_block) = mined_block {
            let mut blockchain_write = blockchain.write().await;

            // Ensure the chain hasn't been updated since mining began
            if blockchain_write.is_valid_new_block(&new_block) {
                blockchain_write.chain.push(new_block.clone());
                miner_info!(
                    "Mining complete! Block added to blockchain: {:?} (Elapsed: {:?})",
                    new_block,
                    start_time.elapsed()
                );
                broadcaster
                    .lock()
                    .await
                    .broadcast_new_block(&new_block)
                    .await;
            } else {
                miner_info!("Mining became invalid due to a chain update.");
                continue; // Restart the mining loop
            }
        }

        // Reset and restart on interruption or completion
        tokio::time::sleep(Duration::from_secs(1)).await;
        miner_info!("Restarting mining...");
    }
}
