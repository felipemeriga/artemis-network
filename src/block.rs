use crate::transaction::Transaction;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// Import the necessary traits and types
// We will use hex encoding

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>, // Store the transactions
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64, // New field for PoW
}

impl Block {
    pub fn new(
        index: u64,
        timestamp: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
    ) -> Self {
        let mut block = Block {
            index,
            timestamp,
            transactions,
            previous_hash,
            hash: String::new(), // Initially empty
            nonce: 0,            // Initially zero
        };

        block.hash = block.calculate_hash(); // Calculate hash after creating the block
        block
    }

    pub fn calculate_hash(&self) -> String {
        let transactions_data: String = self
            .transactions
            .iter()
            .map(|tx| tx.hash()) // Get hash of each transaction
            .collect::<Vec<_>>()
            .join(""); // Concatenate all transaction hashes

        let input = format!(
            "{}{}{}{}{}",
            self.index, self.timestamp, transactions_data, self.previous_hash, self.nonce
        );

        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();

        hex::encode(result)
    }

    // Mine the block (PoW)
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty); // Target hash difficulty (e.g., "0000...")

        while &self.hash[..difficulty] != target {
            self.nonce += 1;
            self.hash = self.calculate_hash();
        }
    }

    pub fn is_valid(&self, difficulty: usize) -> bool {
        self.hash.starts_with(&"0".repeat(difficulty))
    }

    pub fn mine_step(&mut self) {
        self.nonce += 1;
        self.hash = self.calculate_hash();
    }
}
