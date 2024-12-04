use sha2::{Sha256, Digest}; // Import the necessary traits and types
use hex; // We will use hex encoding

pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64, // New field for PoW
}

impl Block {
    pub fn new(index: u64, timestamp: u64, data: String, previous_hash: String) -> Self {
        let mut block = Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(), // Initially empty
            nonce: 0,            // Initially zero
        };

        block.hash = block.calculate_hash(); // Calculate hash after creating the block
        block
    }

    pub(crate) fn calculate_hash(&self) -> String {
        let input = format!("{}{}{}{}{}", self.index, self.timestamp, self.data, self.previous_hash, self.nonce);

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
        println!("Block mined! Nonce: {}, Hash: {}", self.nonce, self.hash);
    }
}
