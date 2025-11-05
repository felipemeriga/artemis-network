# Transaction Pool

This document explains the transaction pool implementation, including the priority queue, lazy deletion pattern, and pending transaction management.

## Table of Contents
- [What is the Transaction Pool?](#what-is-the-transaction-pool)
- [Data Structure](#data-structure)
- [Lazy Deletion Pattern](#lazy-deletion-pattern)
- [Priority Queue](#priority-queue)
- [Transaction Lifecycle in Pool](#transaction-lifecycle-in-pool)
- [Pending Transactions](#pending-transactions)
- [Conflict Resolution](#conflict-resolution)
- [Implementation Details](#implementation-details)

## What is the Transaction Pool?

The **transaction pool** (also called **mempool** in other blockchains) is a holding area for validated transactions waiting to be included in a block.

**Purpose**:
- Store pending transactions
- Prioritize transactions by fee
- Provide transactions to miners
- Prevent double-spending
- Handle mining interruptions

**Location**: `src/pool.rs:6-27`

```rust
pub struct TransactionPool {
    pub heap: BinaryHeap<Transaction>,           // Priority queue (max-heap)
    pub tx_map: HashMap<String, Transaction>,     // Fast lookup by hash
    pub removed_set: HashSet<String>,             // Lazy deletion
    pub pending_map: HashMap<String, Transaction>, // Currently mining
}
```

## Data Structure

The transaction pool uses **four data structures** working together to efficiently manage transactions.

### 1. Binary Heap (Priority Queue)

**Type**: `BinaryHeap<Transaction>`
**Purpose**: Order transactions by priority (fee)

**Characteristics**:
- **Max-heap**: Highest-priority transaction at top
- **O(log n)** insertion
- **O(log n)** removal (pop)
- **Cannot efficiently remove arbitrary elements**

**Why Max-Heap?**

Transactions implement `Ord` trait with fees (highest priority on top):

**Location**: `src/transaction.rs:58-64`

```rust
impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee
            .cmp(&other.fee) // Higher fee = higher priority
            .then_with(|| other.timestamp.cmp(&self.timestamp)) // Older = tiebreaker
    }
}
```

### 2. Transaction Map

**Type**: `HashMap<String, Transaction>`
**Purpose**: Fast lookup and duplicate prevention

**Key**: Transaction hash (SHA-256)
**Value**: Full transaction

**Operations**:
- **O(1)** lookup: Check if transaction exists
- **O(1)** insertion: Add transaction
- **O(1)** removal: Remove transaction

**Why Needed?**

BinaryHeap doesn't support:
- Fast lookup (would require O(n) traversal)
- Arbitrary removal (would require O(n) search + O(n) removal)

HashMap provides O(1) access for these operations.

### 3. Removed Set

**Type**: `HashSet<String>`
**Purpose**: Track removed transactions without traversing heap

**Why Needed?**

Removing from middle of BinaryHeap is expensive:
1. Search for element: O(n)
2. Remove element: O(n)
3. Re-heapify: O(log n)

**Lazy Deletion Solution**:
- Mark transaction as removed in HashSet: O(1)
- Skip it when popped from heap later
- Actually remove from heap only when encountered

See [Lazy Deletion Pattern](#lazy-deletion-pattern) for details.

### 4. Pending Map

**Type**: `HashMap<String, Transaction>`
**Purpose**: Track transactions currently being mined

**Why Needed?**

Transactions extracted for mining should:
- Not be returned to pool if successfully mined
- Be returned to pool if mining interrupted
- Not be duplicated if rebroadcast during mining

**States**:
- **In heap/tx_map**: Waiting to be mined
- **In pending_map**: Currently being mined
- **Not in pool**: Successfully mined or rejected

## Lazy Deletion Pattern

**Problem**: Removing arbitrary elements from BinaryHeap is expensive (O(n)).

**Solution**: **Lazy Deletion** - mark as removed, actually delete later.

### How It Works

**When removing a transaction**:

```rust
// Instead of expensive heap traversal...
// self.heap.remove(tx); // O(n) - NOT DONE

// Mark as removed in set
self.removed_set.insert(tx_hash); // O(1)
self.tx_map.remove(&tx_hash);     // O(1)
```

**When retrieving next transaction**:

**Location**: `src/pool.rs:66-81`

```rust
pub fn get_next_transaction(&mut self) -> Option<Transaction> {
    while let Some(tx) = self.heap.pop() {
        let tx_hash = tx.hash();

        // Check if marked as removed
        if self.removed_set.contains(&tx_hash) {
            // Discard and continue to next
            self.removed_set.remove(&tx_hash);
            continue;
        }

        // Valid transaction - remove from map and return
        self.tx_map.remove(&tx_hash);
        return Some(tx);
    }
    None
}
```

### Benefits

**Performance**:
- Removal: O(1) instead of O(n)
- No heap traversal needed
- No re-heapification on removal

**Correctness**:
- Eventually removed when popped
- tx_map immediately updated (no stale lookups)
- Removed transactions skipped efficiently

**Tradeoff**:
- Heap contains "ghost" entries
- Cleaned up lazily during pop operations
- Slightly more memory usage temporarily

### Example Flow

```
Initial State:
heap: [TX-A(fee:1.0), TX-B(fee:0.5), TX-C(fee:0.3)]
tx_map: {hash-A: TX-A, hash-B: TX-B, hash-C: TX-C}
removed_set: {}

Remove TX-B (fee: 0.5):
heap: [TX-A(fee:1.0), TX-B(fee:0.5), TX-C(fee:0.3)]  // Unchanged
tx_map: {hash-A: TX-A, hash-C: TX-C}                  // Removed
removed_set: {hash-B}                                 // Added

Pop next transaction:
1. heap.pop() → TX-A
2. Check removed_set: hash-A not in set
3. Return TX-A

Pop next transaction:
1. heap.pop() → TX-B
2. Check removed_set: hash-B IN set
3. Remove hash-B from removed_set
4. Continue loop
5. heap.pop() → TX-C
6. Check removed_set: hash-C not in set
7. Return TX-C
```

## Priority Queue

Transactions are ordered by **fee** (primary) and **timestamp** (tiebreaker).

### Priority Rules

**Location**: `src/transaction.rs:58-64`

```rust
impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee
            .cmp(&other.fee) // 1. Higher fee wins
            .then_with(|| other.timestamp.cmp(&self.timestamp)) // 2. Older wins
    }
}
```

**Priority Order**:
1. **Higher fee** → Higher priority (mined first)
2. **Older timestamp** → Higher priority (if fees equal)

### Example Ordering

```
Heap (Max-Heap):
         TX-A (fee: 1.0, time: 100)
        /                           \
TX-B (fee: 0.5, time: 200)    TX-C (fee: 0.5, time: 150)
    /
TX-D (fee: 0.1, time: 300)

Mining Order:
1. TX-A (fee: 1.0) - highest fee
2. TX-C (fee: 0.5, time: 150) - same fee as B, but older
3. TX-B (fee: 0.5, time: 200) - same fee as C, but newer
4. TX-D (fee: 0.1) - lowest fee
```

### Why This Ordering?

**Economic Incentive**:
- Miners earn more from high-fee transactions
- Users pay higher fees for faster confirmation
- Creates fee market

**Fairness**:
- Among equal fees, older transactions prioritized
- Prevents indefinite waiting (eventual processing)
- First-come-first-served within fee tier

## Transaction Lifecycle in Pool

### 1. Addition

**Location**: `src/pool.rs:48-58`

```rust
pub fn add_transaction(&mut self, transaction: Transaction) {
    let tx_hash = transaction.hash();

    // Check for duplicates
    if self.tx_map.contains_key(&tx_hash) || self.pending_map.contains_key(&tx_hash) {
        return; // Already in pool or being mined
    }

    // Add to both structures
    self.tx_map.insert(tx_hash.clone(), transaction.clone());
    self.heap.push(transaction);
}
```

**Checks**:
- Not already in active pool (tx_map)
- Not currently being mined (pending_map)

**Operations**:
- Insert into map: O(1)
- Push to heap: O(log n)

### 2. Extraction for Mining

**Location**: `src/pool.rs:86-99`

```rust
pub fn get_transactions_to_mine(&mut self, amount: i32) -> Vec<Transaction> {
    let mut transactions: Vec<Transaction> = vec![];

    for _ in 0..amount {
        match self.get_next_transaction() {
            Some(tx) => {
                // Move to pending map
                self.pending_map.insert(tx.hash(), tx.clone());
                transactions.push(tx);
            }
            None => break, // No more transactions
        }
    }

    transactions
}
```

**Process**:
1. Pop up to `amount` highest-priority transactions
2. Move each to `pending_map`
3. Remove from `tx_map`
4. Return vector of transactions

**State Change**:
```
Before:
tx_map: {hash-A, hash-B, hash-C}
pending_map: {}

After (get 2 transactions):
tx_map: {hash-C}
pending_map: {hash-A, hash-B}
```

### 3. Successful Mining

**Location**: `src/pool.rs:115-118`

```rust
if mined_by_self {
    self.pending_map.clear(); // All pending transactions were included
    return;
}
```

**Process**:
- Clear entire `pending_map`
- Transactions are now in blockchain
- No longer in pool

### 4. Mining Interrupted

**Location**: `src/pool.rs:140-147`

```rust
if !self.pending_map.is_empty() {
    let tx_to_add: Vec<_> = self.pending_map.values().cloned().collect();
    self.pending_map.clear();

    for tx in tx_to_add {
        self.add_transaction(tx); // Return to pool
    }
}
```

**Process**:
- Extract all pending transactions
- Clear pending map
- Re-add to pool (heap + tx_map)
- Transactions will be re-prioritized

## Pending Transactions

**Purpose**: Track transactions currently being mined to handle interruptions and prevent duplicates.

### States

| State | Location | Meaning |
|-------|----------|---------|
| **Active** | `tx_map` + `heap` | Waiting to be mined |
| **Pending** | `pending_map` | Currently being mined |
| **Removed** | `removed_set` | Marked for deletion |
| **None** | Not in pool | Mined, rejected, or never added |

### Pending Map Usage

**1. Duplicate Prevention** (`pool.rs:41-44`):
```rust
pub fn transaction_already_exists(&self, transaction: &Transaction) -> bool {
    self.tx_map.contains_key(&transaction.hash())
        || self.pending_map.contains_key(&transaction.hash())
}
```

Prevents re-adding transaction that's being mined.

**2. Mining Finalization** (`pool.rs:107-148`):
```rust
pub fn process_mined_transactions(
    &mut self,
    mined_by_self: bool,
    confirmed_transactions: &[Transaction],
) {
    if mined_by_self {
        // Our mining succeeded
        self.pending_map.clear();
        return;
    }

    // Another miner succeeded
    for tx in confirmed_transactions {
        let tx_hash = tx.hash();

        if self.pending_map.contains_key(&tx_hash) {
            // Was in pending - just remove
            self.pending_map.remove(&tx_hash);
        } else if self.tx_map.contains_key(&tx_hash) {
            // Was in active pool - lazy delete
            self.tx_map.remove(&tx_hash);
            self.removed_set.insert(tx_hash);
        }
    }

    // Return remaining pending to pool
    if !self.pending_map.is_empty() {
        let tx_to_add: Vec<_> = self.pending_map.values().cloned().collect();
        self.pending_map.clear();

        for tx in tx_to_add {
            self.add_transaction(tx);
        }
    }
}
```

**Scenarios**:

**A. Self Mined Block**:
```
pending_map: {TX-A, TX-B, TX-C}

Block mined successfully by us
→ Clear pending_map
```

**B. Other Miner's Block (with overlap)**:
```
Our pending_map: {TX-A, TX-B, TX-C}
Other's block: [TX-A, TX-B, TX-D]

Process:
1. Remove TX-A from pending_map (was mining, now mined)
2. Remove TX-B from pending_map (was mining, now mined)
3. TX-D not in our pool - ignore
4. TX-C still in pending_map - return to active pool
```

**C. Other Miner's Block (no overlap)**:
```
Our pending_map: {TX-A, TX-B}
Our tx_map: {TX-C, TX-D, TX-E}
Other's block: [TX-D, TX-E]

Process:
1. TX-D in tx_map - lazy delete (remove from tx_map, add to removed_set)
2. TX-E in tx_map - lazy delete
3. TX-A, TX-B still in pending_map - return to active pool
```

## Conflict Resolution

**Conflict**: Multiple transactions from same sender with different amounts/recipients.

### Current Implementation

**Location**: `src/pool.rs:48-58`

```rust
pub fn add_transaction(&mut self, transaction: Transaction) {
    let tx_hash = transaction.hash();

    // Duplicate prevention
    if self.tx_map.contains_key(&tx_hash) || self.pending_map.contains_key(&tx_hash) {
        return;
    }

    self.tx_map.insert(tx_hash.clone(), transaction.clone());
    self.heap.push(transaction);
}
```

**Current Behavior**:
- Only checks for **exact duplicates** (same hash)
- Does NOT check for conflicting transactions (same sender, different data)
- Allows multiple transactions from same sender

### Limitation

**Example Conflict**:
```
TX-A: Alice → Bob (10 coins, fee 0.1)
TX-B: Alice → Charlie (10 coins, fee 0.2)

Both accepted into pool, but Alice only has 10 coins.
One will fail when mined.
```

**Ideal Behavior** (not implemented):
1. Check sender's balance
2. If conflicting transactions exist, keep higher-fee transaction
3. Reject or replace lower-fee transaction

**Why Not Implemented?**

Complexity:
- Requires tracking sender's pending balance
- Requires efficient lookup by sender address
- Adds significant overhead

Educational focus:
- Demonstrates core concepts
- Simplified for learning
- Real blockchains (Bitcoin, Ethereum) have complex nonce/sequence systems

## Implementation Details

### Initialization

**Location**: `src/pool.rs:30-37`

```rust
pub fn new() -> Self {
    TransactionPool {
        heap: BinaryHeap::new(),
        tx_map: HashMap::new(),
        removed_set: HashSet::new(),
        pending_map: HashMap::new(),
    }
}
```

All structures initialized as empty.

### Complexity Analysis

| Operation | Time Complexity | Space Complexity |
|-----------|----------------|------------------|
| `add_transaction` | O(log n) | O(1) |
| `get_next_transaction` | O(log n) amortized | O(1) |
| `get_transactions_to_mine` | O(k log n) | O(k) |
| `transaction_already_exists` | O(1) | O(1) |
| `process_mined_transactions` | O(m log n) | O(m) |

**Variables**:
- n = number of transactions in pool
- k = number of transactions requested
- m = number of mined transactions

### Memory Usage

**Per Transaction**:
- Heap: 1 copy
- tx_map: 1 copy
- removed_set: hash only (if removed)
- pending_map: 1 copy (if pending)

**Typical**:
- Active transaction: 2 copies (heap + tx_map)
- Pending transaction: 1 copy (pending_map)
- Removed transaction: 1 copy (heap, lazily deleted) + 1 hash (removed_set)

**Memory Optimization**:

Could use `Rc<Transaction>` or indices to avoid copies, but:
- Simplicity preferred for educational code
- Transaction copies are relatively small
- Not a performance bottleneck in practice

## Summary

**Transaction Pool Design**:
- **BinaryHeap**: Priority queue (max-heap by fee)
- **HashMap** (tx_map): Fast lookup and duplicate prevention
- **HashSet** (removed_set): Lazy deletion
- **HashMap** (pending_map): Track transactions being mined

**Key Patterns**:
- **Lazy Deletion**: Mark as removed (O(1)), delete when popped
- **Dual Data Structures**: Heap for ordering, map for lookup
- **Pending State**: Separate tracking for mining transactions
- **Priority Ordering**: Fee-based with timestamp tiebreaker

**Operations**:
- Add: O(log n)
- Extract: O(log n) amortized
- Lookup: O(1)
- Remove: O(1) (lazy)

**Lifecycle**:
1. Add to pool (heap + tx_map)
2. Extract for mining (move to pending_map)
3. Mining succeeds → clear pending
4. Mining interrupted → return to pool

**Limitations**:
- No sender-based conflict resolution
- No balance tracking in pool
- Allows conflicting transactions from same sender
- Relies on validation at mining time

This design demonstrates efficient priority queue management with lazy deletion in a simplified, educational blockchain implementation.
