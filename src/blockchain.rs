use crate::block::Block;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,  // Difficulty for PoW
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            difficulty: 5, // Set a default difficulty
        };

        // Create the genesis block (first block in the chain)
        blockchain.create_genesis_block();

        blockchain
    }

    fn create_genesis_block(&mut self) {
        let genesis_block = Block::new(0, 1627926783, "Genesis Block".to_string(), "0".to_string());
        self.chain.push(genesis_block);
    }

    pub fn add_block(&mut self, data: String) {
        let previous_block = self.chain.last().unwrap();
        let new_block = Block::new(self.chain.len() as u64, 1627926784, data, previous_block.hash.clone());

        // Mine the block
        let mut block_to_add = new_block;
        block_to_add.mine(self.difficulty);

        self.chain.push(block_to_add);
    }
}
