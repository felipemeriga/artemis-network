use crate::block::Block;
use crate::error::DatabaseError;
use crate::transaction::Transaction;
use sled::Db;

pub struct Database {
    pub db: Db,
}

impl Database {
    pub fn new(node_id: String) -> Self {
        let db_path_for_node = format!("./database/blockchain-db-{}", node_id);

        // If running over dev feature, the DB will be recreated every time we run the program again
        #[cfg(feature = "dev")]
        {
            use std::fs;

            let path = db_path_for_node.clone();
            // Remove the old database directory if it exists
            if fs::metadata(path.clone()).is_ok() {
                fs::remove_dir_all(path).expect("Failed to delete existing DB folder");
            }
        }

        if let Ok(db) = sled::open(db_path_for_node) {
            Self { db }
        } else {
            panic!("Failed to open database");
        }
    }

    pub fn store_transaction(&self, tx: &Transaction, tx_hash: &str) -> Result<(), DatabaseError> {
        // Store transaction by hash
        self.db.insert(
            tx_hash,
            bincode::serialize(tx).map_err(|_| DatabaseError::BinCodeError)?,
        )?;

        // Index transaction by sender
        let sender_key = format!("addr_{}", tx.sender);
        let recipient_key = format!("addr_{}", tx.recipient);

        self.add_transaction_to_index(&sender_key, tx_hash)?;
        self.add_transaction_to_index(&recipient_key, tx_hash)?;
        Ok(())
    }

    pub fn add_transaction_to_index(&self, key: &str, tx_hash: &str) -> Result<(), DatabaseError> {
        let mut tx_list: Vec<String> = match self.db.get(key)? {
            Some(value) => bincode::deserialize(&value).map_err(|_| DatabaseError::BinCodeError)?,
            None => vec![],
        };

        if !tx_list.contains(&tx_hash.to_string()) {
            tx_list.push(tx_hash.to_string());
            self.db.insert(
                key,
                bincode::serialize(&tx_list).map_err(|_| DatabaseError::BinCodeError)?,
            )?;
        }

        Ok(())
    }

    pub fn get_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>, DatabaseError> {
        match self.db.get(tx_hash)? {
            Some(value) => Ok(Some(
                bincode::deserialize(&value).map_err(|_| DatabaseError::BinCodeError)?,
            )),
            None => Ok(None),
        }
    }

    pub fn get_transactions_by_wallet(
        &self,
        wallet: &str,
    ) -> Result<Vec<Transaction>, DatabaseError> {
        let key = format!("addr_{}", wallet);
        match self.db.get(key)? {
            Some(value) => {
                let tx_hashes: Vec<String> =
                    bincode::deserialize(&value).map_err(|_| DatabaseError::BinCodeError)?;
                let mut transactions = vec![];

                for tx_hash in tx_hashes {
                    if let Some(tx) = self.get_transaction(&tx_hash)? {
                        transactions.push(tx);
                    }
                }

                Ok(transactions)
            }
            None => Ok(vec![]),
        }
    }

    pub fn get_wallet_balance(&self, wallet_address: &str) -> Result<f64, DatabaseError> {
        let transactions = self.get_transactions_by_wallet(wallet_address)?;

        let mut balance: f64 = 0.0;

        transactions.iter().for_each(|tx| {
            if tx.recipient == wallet_address {
                balance += tx.amount.into_inner(); // Add received amount
            }
            if tx.sender == wallet_address {
                balance -= tx.amount.into_inner(); // Subtract sent amount
                balance -= tx.fee.into_inner(); // Subtract sent fee
            }
        });

        Ok(balance)
    }

    pub fn store_block(&self, block: &Block) -> Result<(), DatabaseError> {
        let key = format!("block:{}", block.hash);
        let value = serde_json::to_vec(block).unwrap();
        self.db.insert(key, value)?;
        Ok(())
    }

    pub fn get_block(&self, block_hash: &str) -> Option<Block> {
        let key = format!("block:{}", block_hash);
        if let Ok(Some(value)) = self.db.get(key) {
            let block: Block = serde_json::from_slice(&value).unwrap();
            return Some(block);
        }
        None
    }

    pub fn get_all_blocks(&self) -> Vec<Block> {
        let mut blocks: Vec<_> = self
            .db
            .scan_prefix("block:")
            .filter_map(|item| {
                item.ok()
                    .and_then(|(_, value)| serde_json::from_slice::<Block>(&value).ok())
            })
            .collect();

        // Sort by the block index field (assuming u64 or similar for sorting purposes)
        blocks.sort_by(|a, b| a.index.cmp(&b.index));
        blocks
    }

    // Store a list of blocks with all their internal transactions
    pub fn store_blocks_and_transactions(&self, blocks: Vec<Block>) -> Result<(), DatabaseError> {
        // Loop through each block
        for block in blocks {
            // Store the block itself
            self.store_block(&block)?;

            // Store all transactions in the block
            for tx in &block.transactions {
                let tx_hash = tx.hash();
                self.store_transaction(tx, &tx_hash)?; // Store each transaction and its hash
            }
        }
        Ok(())
    }
}
