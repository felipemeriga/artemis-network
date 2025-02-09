use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::broadcaster::Broadcaster;
use crate::miner_info;
use crate::pool::TransactionPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::select;
use tokio::sync::{mpsc::Receiver, Mutex, RwLock};
use tokio::time::Duration;

pub struct Miner {
    blockchain: Arc<RwLock<Blockchain>>,
    broadcaster: Arc<Mutex<Broadcaster>>,
    block_rx: Receiver<Option<Block>>,
    transaction_pool: Arc<Mutex<TransactionPool>>,
    mine_without_transactions: bool,
    transactions_per_block: i32,
}

impl Miner {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        broadcaster: Arc<Mutex<Broadcaster>>,
        block_rx: Receiver<Option<Block>>,
        transaction_pool: Arc<Mutex<TransactionPool>>,
        mine_without_transactions: bool,
        transactions_per_block: i32,
    ) -> Self {
        Self {
            blockchain,
            broadcaster,
            block_rx,
            transaction_pool,
            mine_without_transactions,
            transactions_per_block,
        }
    }

    pub async fn mine(&mut self) {
        loop {
            let data = {
                self.transaction_pool
                    .lock()
                    .await
                    .get_transactions_to_mine(self.transactions_per_block)
            };
            // If there are no transactions,
            // and this miner is configured to mine only when there are
            // transactions,
            // it won't start the process until a new transaction arrives
            if data.len() <= 0 && !self.mine_without_transactions {
                continue;
            }

            // For now, since our blockchain system is quite small, and used for learning
            // purposes, we will just include a single transaction in a block

            let mut mined_block: Option<Block> = None;

            // Prepare a new block for mining
            let (mut candidate_block, difficulty) = {
                let blockchain_read = self.blockchain.read().await;
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
                    Some(new_block) = self.block_rx.recv() => {
                        miner_info!("Received valid updated state during mining, aborting the current process");
                        if new_block.is_some() {
                            // check if the new incoming block,
                            // contains transactions that are present in this transaction pool
                            {
                             self.transaction_pool.lock().await.process_mined_transactions(false, &new_block.unwrap().transactions);
                            }

                        }
                        break; // Exit the mining loop and restart
                    }

                    // Simulate mining time to let other tasks execute, uncomment this for making the mining
                    // process slower
                    // _ = tokio::time::sleep(Duration::from_nanos(10)) => {}
                    _ = tokio::task::yield_now() => {}
                }
            }

            // Commit the mined block if no new block was received
            if let Some(new_block) = mined_block {
                let mut blockchain_write = self.blockchain.write().await;

                // Ensure the chain hasn't been updated since mining began
                if blockchain_write.is_valid_new_block(&new_block) {
                    blockchain_write.chain.push(new_block.clone());
                    miner_info!(
                        "Mining complete! Block added to blockchain: {:?} (Elapsed: {:?})",
                        new_block,
                        start_time.elapsed()
                    );
                    {
                        self.transaction_pool
                            .lock()
                            .await
                            .process_mined_transactions(true, &new_block.transactions);
                        self.broadcaster
                            .lock()
                            .await
                            .broadcast_new_block(&new_block)
                            .await;
                    }
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
}
