

#[cfg(test)]
mod tests {
    use crate::blockchain;

    #[test]
    fn create_dummy_blockchain() {
        let mut blockchain = blockchain::Blockchain::new();

        // Add blocks to the blockchain with data
        let second_block = blockchain.mine_new_block("Second Block".to_string());
        blockchain.add_block(second_block);
        let third_block = blockchain.mine_new_block("Third Block".to_string());
        blockchain.add_block(third_block);

        // Print out the blockchain with hashes
        for block in blockchain.clone().chain {
            println!("Block #{}: Hash: {}", block.index, block.hash);
        }
        assert_eq!(blockchain.chain.len(), 3);

    }
}