use crate::block::Block;
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex, RwLock};
use tokio::time::{sleep, Duration};
use tokio::select;

pub async fn mine(blockchain: Arc<RwLock<Blockchain>>, peers: Arc<Mutex<Vec<String>>>, mut block_rx: Receiver<Option<Block>>) {
    loop {
        let data = format!("Block at {}", chrono::Utc::now());
        let mut mined_block: Option<Block> = None;

        // Concurrently mine and watch for new blocks
        select! {
            // Mining logic
            _ = async {

                        // Part 1: Extract necessary data while holding the read lock
                let (mut prepared_block, difficult);
                let mining_data = data.clone();
                {
                    let blockchain_read = blockchain.read().await;
                    (prepared_block, difficult) = blockchain_read.prepare_block_for_mining(mining_data); // Assume we create this method
                } // The read lock is released here


                println!("Mining a new block with data: {}", data);
                prepared_block.mine(difficult);
                mined_block = Some(prepared_block); // Mining complete
            } => {},

            // Listen for new blocks from the network
            Some(received_block) = block_rx.recv() => {
                if let Some(received_block) = received_block {
                    println!("New block received: {:?}. Halting mining and restarting...", received_block);
                    // Here we don't need to append the received block, since it's a blockchain behind an Arc<Mutex>>
                    // The block will be appended right after the server task receives the update

                    // Restart mining with the updated blockchain state
                    continue;
                }
            }
        }

        // Commit the mined block if no new block was received
        if let Some(new_block) = mined_block {
            let mut blockchain_write = blockchain.write().await;

            // Ensure the chain hasn't been updated since mining began
            if blockchain_write.is_valid_new_block(&new_block) {
                blockchain_write.chain.push(new_block.clone());
                println!("New block mined and added to the blockchain: {:?}", new_block);
            } else {
                println!("Mined block is invalid due to an update. Restarting...");
                continue; // Restart the mining loop
            }

            println!("broadcasting new block to peers");
            // Server::broadcast_new_block(&new_block, peers.clone()).await;
        }

        // Wait before starting to mine the next block
        sleep(Duration::from_secs(1)).await; // Adjustable based on network needs
    }
}
