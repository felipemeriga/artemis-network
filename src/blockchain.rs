use crate::block::Block;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis_block = create_genesis_block();
        // let genesis_block = Block::new(0, 0, vec![], "0".to_string());
        let blockchain = Blockchain {
            chain: vec![genesis_block],
            difficulty: 5, // Set the PoW difficulty (e.g., 4 leading zeros)
        };
        // By default, the block after genesis, will contain no transactions
        blockchain.prepare_block_for_mining(vec![]);

        blockchain
    }

    pub fn is_valid_chain(chain: &[Block]) -> bool {
        for i in 1..chain.len() {
            if chain[i].previous_hash != chain[i - 1].hash
                || chain[i].hash != chain[i].calculate_hash()
            {
                return false;
            }
        }
        true
    }

    pub fn add_block(&mut self, new_block: Block) -> bool {
        // Ensure the block's previous_hash is valid
        let last_block = self.chain.last().unwrap();
        if last_block.hash == new_block.previous_hash {
            self.chain.push(new_block);
            return true;
        }
        false
    }

    pub fn is_valid_new_block(&self, block: &Block) -> bool {
        if let Some(last_block) = self.chain.last() {
            // Validate previous hash
            if block.previous_hash != last_block.hash {
                return false;
            }

            let calculated_hash = block.calculate_hash();
            // Validate block's hash and PoW
            if block.hash != calculated_hash
                || !block.hash.starts_with(&"0".repeat(self.difficulty))
            {
                return false;
            }
            return true;
        }
        false
    }

    // pub fn validate_block(&self, block: &Block) -> bool {
    //     // Check if the block's hash matches the difficulty
    //     let target = "0".repeat(self.difficulty);
    //     block.hash.starts_with(&target)
    // }

    pub fn replace_chain(&mut self, new_chain: Vec<Block>) {
        self.chain = new_chain;
    }

    #[allow(dead_code)]
    pub fn mine_new_block(&self, data: Vec<Transaction>) -> Block {
        let (mut mined_block, difficult) = self.prepare_block_for_mining(data);
        mined_block.mine(difficult);

        mined_block
    }

    pub fn prepare_block_for_mining(&self, data: Vec<Transaction>) -> (Block, usize) {
        let last_block = self.chain.last().unwrap();
        let new_index = last_block.index + 1;
        let new_timestamp = chrono::Utc::now().timestamp() as u64;
        let new_block = Block::new(new_index, new_timestamp, data, last_block.hash.clone());
        let mined_block = new_block;
        (mined_block, self.difficulty)
    }

    pub fn get_last_block(&self) -> &Block {
        self.chain.last().unwrap()
    }

    pub fn get_chain(&self) -> Vec<Block> {
        self.chain.clone()
    }
}

fn create_genesis_block() -> Block {
    Block {
        index: 0,                                               // First block has index 0
        timestamp: 0,         // Placeholder for the timestamp (e.g., Unix epoch time 0)
        transactions: vec![], // No transactions in the genesis block
        previous_hash: String::from("0"), // Special value to denote no parent block
        hash: String::from("00000000000000000000000000000000"), // Predefined hash for genesis
        nonce: 0,             // PoW value starts at 0
    }
}
