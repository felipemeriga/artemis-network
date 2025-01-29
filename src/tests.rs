#[cfg(test)]
mod tests {
    use crate::blockchain;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;

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

    #[test]
    fn create_wallet() {
        let wallet = crate::wallet::Wallet::new();
        let address = wallet.address();
        // The sha256_hash produces, a 32 bytes result,
        // but as we are doing a hex string encoding
        // where each byte is represented by 2 hex characters
        // therefore it's still 32 bytes, but 64 characters long
        assert_eq!(address.len(), 64);
    }

    #[test]
    fn verify_transaction_signature() {
        let sender_wallet = Wallet::new();
        let recipient_wallet = Wallet::new();
        let amount = 1.0;

        let mut transaction =
            Transaction::new(sender_wallet.address(), recipient_wallet.address(), amount);
        transaction.sign(&sender_wallet);
        assert_eq!(transaction.verify(&sender_wallet.public_key), true);
    }

    #[test]
    fn verify_transaction_signature_invalid() {
        let sender_wallet = Wallet::new();
        let recipient_wallet = Wallet::new();
        let amount = 1.0;

        let mut transaction =
            Transaction::new(sender_wallet.address(), recipient_wallet.address(), amount);
        transaction.sign(&sender_wallet);

        // Modifying the value of the amount, therefore the signature will be invalid
        // since the signing hash, will be different
        transaction.amount = 10.0;

        assert_eq!(transaction.verify(&sender_wallet.public_key), false);
    }
}
