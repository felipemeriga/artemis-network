use crate::block::Block;

pub struct Consensus;

impl Consensus {
    // Proof of Work algorithm
    pub fn proof_of_work(block: &mut Block, difficulty: usize) {
        let target = "0".repeat(difficulty);

        while &block.hash[..difficulty] != target {
            block.nonce += 1;
            block.hash = block.calculate_hash();
        }
        println!("Block mined with nonce: {} and hash: {}", block.nonce, block.hash);
    }
}
