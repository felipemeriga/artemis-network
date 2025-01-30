use crate::transaction::Transaction;
use std::collections::BinaryHeap;

pub struct TransactionPool {
    pub heap: BinaryHeap<Transaction>,
}

impl TransactionPool {
    pub fn new() -> Self {
        TransactionPool {
            heap: BinaryHeap::new(),
        }
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.heap.push(transaction);
    }

    pub fn get_next_transaction(&mut self) -> Option<Transaction> {
        self.heap.pop() // Returns the highest-priority transaction
    }
}
