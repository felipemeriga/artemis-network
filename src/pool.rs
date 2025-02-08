use crate::transaction::Transaction;
use std::collections::{BinaryHeap, HashMap, HashSet};

// Here is where all the incoming transactions will be added
// for being processed
pub struct TransactionPool {
    // Here is a Binary heap, basically a max heap, where transactions with
    // higher fees will be prioritized (higher nodes)
    pub heap: BinaryHeap<Transaction>,
    // Since it's quite challenging, and not effective to keep traversing
    // a binary heap, we use a hash map to register all the transactions that are
    // currently inside the heap.
    pub tx_map: HashMap<String, Transaction>,
    // Removed set, represents the transactions that have been removed during the process;
    // therefore, we don't need to traverse the heap removing them,
    // instead, when we pop the next transaction from the heap, if this transaction
    // is inside the removed set, we basically ignore it, and pop the next one.
    pub removed_set: HashSet<String>, // Store removed transaction hashes
    // Pending map will store transactions that are under the process of mining,
    // having a pending map,
    // is important to avoid issues like,
    // preventing duplicates;
    // Transactions being mined aren't re-added if rebroadcast.
    // Transactions return to the pool if mining is interrupted,
    // but duplicates are avoided.
    pub pending_map: HashMap<String, Transaction>,
}

impl TransactionPool {
    pub fn new() -> Self {
        TransactionPool {
            heap: BinaryHeap::new(),
            tx_map: HashMap::new(),
            removed_set: HashSet::new(),
            pending_map: Default::default(),
        }
    }

    pub fn transaction_already_exists(&self, transaction: &Transaction) -> bool {
        self.tx_map.contains_key(&transaction.hash())
            || self.pending_map.contains_key(&transaction.hash())
    }

    /// Add a transaction to both the heap and the map
    pub fn add_transaction(&mut self, transaction: Transaction) {
        let tx_hash = transaction.hash();

        // Avoid duplicate transactions
        if self.tx_map.contains_key(&tx_hash) || self.pending_map.contains_key(&tx_hash) {
            return;
        }

        self.tx_map.insert(tx_hash.clone(), transaction.clone());
        self.heap.push(transaction);
    }

    pub fn get_transactions_to_mine(&mut self, amount: i32) -> Vec<Transaction> {
        let mut transactions: Vec<Transaction> = vec![];
        for _ in 0..amount {
            match self.get_next_transaction() {
                Some(tx) => {
                    self.pending_map.insert(tx.hash(), tx.clone());
                    transactions.push(tx);
                }
                None => break,
            }
        }

        transactions
    }

    pub fn process_mined_transactions(&mut self, confirmed_transactions: &[Transaction]) {
        for tx in confirmed_transactions {
            self.tx_map.remove(&tx.hash());
            self.removed_set.insert(tx.hash());
            self.pending_map.remove(&tx.hash());
        }

        if self.pending_map.len() > 0 {
            let tx_to_add: Vec<_> = self.pending_map.values().cloned().collect();
            self.pending_map.clear();

            for tx in tx_to_add {
                self.add_transaction(tx);
            }
        }
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
}
