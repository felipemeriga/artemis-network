use sled::{Db};
use crate::error::DatabaseError;
use crate::transaction::Transaction;

pub struct Database {
    pub db: Db
}


impl Database {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("blockchain_db");
        let db = sled::open(temp_dir).unwrap();
        Self { db }
    }

    pub fn store_transaction(&self, tx: &Transaction, tx_hash: &str) -> Result<(), DatabaseError> {
        // Store transaction by hash
        self.db.insert(tx_hash, bincode::serialize(tx).map_err(|_| DatabaseError::BinCodeError)?)?;

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
            self.db.insert(key, bincode::serialize(&tx_list).map_err(|_| DatabaseError::BinCodeError)?)?;
        }

        Ok(())
    }

    // fn get_transaction(db: &Db, tx_hash: &str) -> Result<Option<Transaction>, Box<dyn Error>> {
    //     match db.get(tx_hash)? {
    //         Some(value) => Ok(Some(bincode::deserialize(&value)?)),
    //         None => Ok(None),
    //     }
    // }
}