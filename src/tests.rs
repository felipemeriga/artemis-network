#[cfg(test)]
mod tests {
    use crate::blockchain;
    use crate::pool::TransactionPool;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;
    use ordered_float::OrderedFloat;

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
    fn export_wallet() {
        let wallet = crate::wallet::Wallet::new();
        let export_wallet = wallet.export_wallet();
        assert_eq!(export_wallet.private_key.len(), 64);
        assert_eq!(export_wallet.public_key.len(), 66);

        let wallet_from_binary =
            Wallet::from_binary(export_wallet.public_key, export_wallet.private_key).unwrap();
        assert_eq!(wallet_from_binary.address(), wallet.address());
        assert_eq!(wallet_from_binary.public_key, wallet.public_key);
        assert_eq!(wallet_from_binary.private_key, wallet.private_key);
    }

    #[test]
    fn verify_transaction_signature() {
        let sender_wallet = Wallet::new();
        let recipient_wallet = Wallet::new();
        let amount = 1.0;
        let fee = 0.1;

        let mut transaction = Transaction::new(
            sender_wallet.address(),
            recipient_wallet.address(),
            amount,
            fee,
            chrono::Utc::now().timestamp(),
        );
        transaction.sign(&sender_wallet);
        assert_eq!(transaction.verify(&sender_wallet.public_key), true);
    }

    #[test]
    fn verify_transaction_signature_invalid() {
        let sender_wallet = Wallet::new();
        let recipient_wallet = Wallet::new();
        let amount = 1.0;
        let fee = 0.1;

        let mut transaction = Transaction::new(
            sender_wallet.address(),
            recipient_wallet.address(),
            amount,
            fee,
            chrono::Utc::now().timestamp(),
        );
        transaction.sign(&sender_wallet);

        // Modifying the value of the amount, therefore the signature will be invalid
        // since the signing hash, will be different
        transaction.amount = OrderedFloat::from(10.0);

        assert_eq!(transaction.verify(&sender_wallet.public_key), false);
    }

    #[test]
    fn transaction_pool_get_next_transaction_priority_order_correct() {
        let mut pool = TransactionPool::new();

        let tx1 = Transaction::new("Alice".into(), "Bob".into(), 10.0, 1.0, 100);
        let tx2 = Transaction::new("Charlie".into(), "Dave".into(), 5.0, 2.0, 101);

        pool.add_transaction(tx1);
        pool.add_transaction(tx2);
        // Make sure the transaction with the higher fee will be popped first
        assert_eq!(
            pool.get_next_transaction().unwrap().fee,
            OrderedFloat::from(2.0)
        )
    }

    #[test]
    fn transaction_pool_get_next_transaction_same_fee_order_correct() {
        // When comparing transactions with the same fee, the oldest one will be prioritized
        let mut pool = TransactionPool::new();

        let tx1 = Transaction::new("Alice".into(), "Bob".into(), 10.0, 1.0, 100);
        let tx2 = Transaction::new("Charlie".into(), "Dave".into(), 5.0, 1.0, 101);
        pool.add_transaction(tx1);
        pool.add_transaction(tx2);

        let tx = pool.get_next_transaction().unwrap();
        assert_eq!(tx.amount, OrderedFloat::from(10.0));
        assert_eq!(tx.timestamp, 100);
    }
}
