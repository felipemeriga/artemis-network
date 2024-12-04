mod block;
mod blockchain;
mod network;
mod wallet;
mod consensus;

fn main() {
    let mut blockchain = blockchain::Blockchain::new();

    // Add blocks to the blockchain with data
    blockchain.add_block("Second Block".to_string());
    blockchain.add_block("Third Block".to_string());

    // Print out the blockchain with hashes
    for block in blockchain.chain {
        println!("Block #{}: Hash: {}", block.index, block.hash);
    }
}
