#[cfg(test)]
mod tests {
    use crate::blockchain;
    use crate::blockchain::create_genesis_block;
    use crate::config::load_config;
    use crate::db::Database;
    use crate::pool::TransactionPool;
    use crate::transaction::Transaction;
    use crate::wallet::Wallet;
    use ordered_float::OrderedFloat;
    use std::fs::write;
    use std::string::String;

    #[test]
    fn create_dummy_blockchain() {
        let mut blockchain = blockchain::Blockchain::new();

        // Add blocks to the blockchain with data
        let second_block = blockchain.mine_new_block(vec![]);
        blockchain.add_block(second_block);
        let third_block = blockchain.mine_new_block(vec![]);
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
            Wallet::from_hex_string(export_wallet.public_key, export_wallet.private_key).unwrap();
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
        assert_eq!(transaction.verify(), true);
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

        assert_eq!(transaction.verify(), false);
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

    fn initialize_database() -> crate::db::Database {
        Database::new(String::from("test"))
    }

    fn dump_database() {
        use std::fs;

        let test_database_path = format!("./database/blockchain-db-{}", String::from("test"));

        // Remove the old database directory if it exists
        if fs::metadata(test_database_path.clone()).is_ok() {
            fs::remove_dir_all(test_database_path).expect("Failed to delete existing DB folder");
        }
    }

    #[test]
    fn test_save_get_block() {
        let db = initialize_database();
        let block = create_genesis_block();
        db.store_block(&block).unwrap();
        let block_from_db = db.get_block(block.hash.as_str()).unwrap();
        assert_eq!(block, block_from_db);
        dump_database();
    }

    #[test]
    fn test_load_config_success() {
        let file_path = "test_config.yaml";
        let yaml_content = r#"
        tcpAddress: "127.0.0.1:8080"
        httpAddress: "127.0.0.1:3000"
        bootstrapAddress: "127.0.0.1:4000"
        nodeId: "node-123"
        minerWalletAddress: "30114c915aae70a7f5744f6263119c266b9a8dd9cb209385d4759fa76bf0741b"
        "#;

        // Write test data to a temporary file
        write(file_path, yaml_content).expect("Failed to write test configuration file.");

        // Call the load_config function
        let config = load_config(file_path).expect("Failed to load config.");

        // Validate the loaded config
        assert_eq!(config.tcp_address, "127.0.0.1:8080");
        assert_eq!(config.http_address, "127.0.0.1:3000");
        assert_eq!(config.bootstrap_address, Some("127.0.0.1:4000".to_string()));
        assert_eq!(config.node_id, "node-123");

        // Cleanup test file
        std::fs::remove_file(file_path).expect("Failed to remove test file.");
    }

    #[test]
    fn test_load_config_invalid_yaml() {
        let file_path = "invalid_config.yaml";
        let invalid_yaml_content = r#"
        tcpAddress: 127.0.0.1:8080
        httpAddress: [INVALID key]
        nodeId: "node-123"
        "#;

        // Write invalid test data to a temporary file
        write(file_path, invalid_yaml_content).expect("Failed to write test configuration file.");

        // Call the load_config function
        let result = load_config(file_path);

        // Check that the function returns an error
        assert!(result.is_err());

        // Cleanup test file
        std::fs::remove_file(file_path).expect("Failed to remove test file.");
    }
}
