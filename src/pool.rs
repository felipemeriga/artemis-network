use crate::transaction::Transaction;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub struct TransactionPool {
    pub heap: BinaryHeap<Transaction>,
    pub tx_map: HashMap<String, Transaction>,
    pub removed_set: HashSet<String>, // Store removed transaction hashes
}

impl TransactionPool {
    pub fn new() -> Self {
        TransactionPool {
            heap: BinaryHeap::new(),
            tx_map: HashMap::new(),
            removed_set: HashSet::new(),
        }
    }

    /// Add a transaction to both the heap and the map
    pub fn add_transaction(&mut self, transaction: Transaction) {
        let tx_hash = transaction.hash();

        // Avoid duplicate transactions
        if self.tx_map.contains_key(&tx_hash) {
            return;
        }

        self.tx_map.insert(tx_hash.clone(), transaction.clone());
        self.heap.push(transaction);
    }

    /// Get the next valid transaction, skipping removed ones
    pub fn get_next_transaction(&mut self) -> Option<Transaction> {
        while let Some(tx) = self.heap.pop() {
            let tx_hash = tx.hash();

            if self.removed_set.contains(&tx_hash) {
                // If it's marked as removed, discard it and continue
                self.removed_set.remove(&tx_hash);
                continue;
            }

            // Remove from tx_map and return the valid transaction
            self.tx_map.remove(&tx_hash);
            return Some(tx);
        }
        None
    }

    /// Remove a transaction by its hash safely
    pub fn remove_transaction(&mut self, tx_hash: &str) -> Option<Transaction> {
        if let Some(transaction) = self.tx_map.remove(tx_hash) {
            // Instead of trying to remove from heap (inefficient), mark it as removed
            self.removed_set.insert(tx_hash.to_string());
            Some(transaction)
        } else {
            None
        }
    }

    pub fn add_transactions_back(&mut self, transactions: Vec<Transaction>) {
        for tx in transactions {
            self.add_transaction(tx);
        }
    }

    /// Remove transactions that are present in the new block
    pub fn remove_confirmed_transactions(&mut self, confirmed_transactions: &[Transaction]) {
        for tx in confirmed_transactions {
            let tx_hash = tx.hash();
            self.remove_transaction(&tx_hash);
        }
    }
}
