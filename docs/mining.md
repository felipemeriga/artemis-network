# Mining & Proof of Work

This document explains how mining and proof-of-work (PoW) work in Artemis Network.

## Table of Contents
- [What is Proof of Work?](#what-is-proof-of-work)
- [Mining Difficulty](#mining-difficulty)
- [The Mining Process](#the-mining-process)
- [Mining Interruption](#mining-interruption)
- [Miner Rewards](#miner-rewards)
- [Block Validation](#block-validation)
- [Implementation Details](#implementation-details)

## What is Proof of Work?

**Proof of Work (PoW)** is a consensus mechanism that requires miners to perform computational work to add new blocks to the blockchain. The "work" involves finding a special number (called a **nonce**) that, when combined with the block's data and hashed, produces a hash with specific characteristics.

### The Goal

Find a **nonce** value such that when the block is hashed, the resulting hash starts with a certain number of zeros. This requires trying many different nonce values until one produces a valid hash.

### Why PoW?

- **Security**: Makes it computationally expensive to attack the network
- **Fairness**: Gives all miners a chance based on computational power
- **Rate Limiting**: Controls how fast new blocks are added
- **Decentralization**: Anyone with computing power can participate

## Mining Difficulty

**Difficulty** determines how hard it is to find a valid block hash.

### Configuration

**Location**: `src/blockchain.rs:18`
```rust
pub difficulty: usize, // Default: 5
```

**Difficulty Value**: `5` (requires 5 leading zeros in the block hash)

### What Difficulty Means

Difficulty `5` means the block hash must start with `00000` (five zeros).

**Examples**:
- ✅ **Valid hash** (difficulty 5): `000004f7a8b2...` (starts with 5 zeros)
- ❌ **Invalid hash** (difficulty 5): `00003f7a8b2...` (only 4 zeros)
- ❌ **Invalid hash** (difficulty 5): `1000000000...` (no leading zeros)

### Target Hash Pattern

```rust
// From block.rs:60
let target = "0".repeat(difficulty); // "00000" for difficulty 5
```

The miner keeps incrementing the nonce and recalculating the hash until it finds one that starts with the target pattern.

### Probability & Effort

Each hash has a `1/16^n` probability of having `n` leading hexadecimal zeros:
- Difficulty 1: ~1 in 16 attempts
- Difficulty 2: ~1 in 256 attempts
- Difficulty 3: ~1 in 4,096 attempts
- Difficulty 4: ~1 in 65,536 attempts
- **Difficulty 5**: ~1 in 1,048,576 attempts

This makes it computationally expensive but not impossible.

## The Mining Process

### High-Level Flow

```
1. Wait for sync to complete (first_sync_done flag)
2. Extract transactions from pool (up to transactions_per_block)
3. Prepare candidate block with transactions
4. Calculate total fees from transactions
5. Create COINBASE transaction (reward + fees)
6. Add COINBASE transaction to block
7. Start mining loop:
   a. Increment nonce
   b. Calculate new hash
   c. Check if hash meets difficulty
   d. Listen for new blocks from network
   e. If valid hash found, add to chain and broadcast
   f. If new block received, abort and restart
8. Return transactions to pool if interrupted
9. Clear pending transactions if successful
10. Restart mining
```

### Step-by-Step Implementation

#### Step 1: Wait for Synchronization

**Location**: `src/miner.rs:48-55`
```rust
if !*first_sync_done.lock().await {
    tokio::time::sleep(Duration::from_secs(1)).await;
    continue;
}
```

Mining only starts after the node has synchronized with the network to avoid mining on outdated chains.

#### Step 2: Extract Transactions

**Location**: `src/miner.rs:57-69`
```rust
let data = {
    self.transaction_pool
        .lock()
        .await
        .get_transactions_to_mine(self.transactions_per_block)
};

if data.is_empty() && !self.mine_without_transactions {
    continue;
}
```

Gets up to `transactions_per_block` (typically 10) highest-priority transactions from the pool.

**Configuration**:
- `transactions_per_block`: Maximum transactions per block
- `mine_without_transactions`: If `false`, waits for transactions before mining

#### Step 3: Prepare Candidate Block

**Location**: `src/miner.rs:77-84` and `src/blockchain.rs:106-117`
```rust
let (mut candidate_block, difficulty, miner_reward_tx) = {
    let blockchain_read = self.blockchain.read().await;
    let (candidate_block, fees, difficulty) =
        blockchain_read.prepare_block_for_mining(data.clone());
    let miner_reward_tx =
        blockchain_read.get_miner_transaction(self.wallet_address.clone(), fees);
    (candidate_block, difficulty, miner_reward_tx)
};
```

**Block Preparation** (`blockchain.rs:106-117`):
1. Calculate total fees from all transactions
2. Get last block's hash
3. Create new block with:
   - Index: last block index + 1
   - Timestamp: current UTC time
   - Transactions: from pool
   - Previous hash: last block's hash
   - Hash: initially calculated (not yet valid)
   - Nonce: starts at 0

#### Step 4: Add Miner Reward

**Location**: `src/miner.rs:88-90`
```rust
if let Some(tx) = miner_reward_tx {
    candidate_block.transactions.push(tx);
}
```

Adds the COINBASE transaction (miner reward) to the block.

#### Step 5: Mining Loop

**Location**: `src/miner.rs:95-128`

```rust
loop {
    // Increment nonce and recalculate hash
    candidate_block.mine_step();

    // Check if valid
    if candidate_block.is_valid(difficulty) {
        mined_block = Some(candidate_block.clone());
        break;
    }

    select! {
        // Listen for new blocks from network
        Some(received_option) = self.block_rx.recv() => {
            if let Some(new_block) = received_option {
                // Interrupt mining
                self.transaction_pool
                    .lock()
                    .await
                    .process_mined_transactions(false, &new_block.transactions);
                break;
            }
        }
        _ = tokio::task::yield_now() => {}
    }
}
```

**Mining Step** (`block.rs:72-75`):
```rust
pub fn mine_step(&mut self) {
    self.nonce += 1;
    self.hash = self.calculate_hash();
}
```

**Validation Check** (`block.rs:68-70`):
```rust
pub fn is_valid(&self, difficulty: usize) -> bool {
    self.hash.starts_with(&"0".repeat(difficulty))
}
```

### Hash Calculation

**Location**: `src/block.rs:37-55`

```rust
pub fn calculate_hash(&self) -> String {
    let transactions_data: String = self
        .transactions
        .iter()
        .map(|tx| tx.hash())
        .collect::<Vec<_>>()
        .join("");

    let input = format!(
        "{}{}{}{}{}",
        self.index,
        self.timestamp,
        transactions_data,
        self.previous_hash,
        self.nonce
    );

    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    hex::encode(result)
}
```

**Hash Input Components**:
1. Block index
2. Timestamp
3. Concatenated transaction hashes
4. Previous block hash
5. **Nonce** (the variable we increment)

The nonce is the only value that changes during mining, causing the hash to change with each iteration.

#### Step 6: Commit Mined Block

**Location**: `src/miner.rs:131-170`

```rust
if let Some(new_block) = mined_block {
    let mut blockchain_write = self.blockchain.write().await;

    // Ensure chain hasn't been updated
    if blockchain_write.is_valid_new_block(&new_block) {
        blockchain_write.add_block(new_block.clone());

        // Update transaction pool
        self.transaction_pool
            .lock()
            .await
            .process_mined_transactions(true, &new_block.transactions);

        // Broadcast to network
        self.broadcaster
            .lock()
            .await
            .broadcast_item(BroadcastItem::NewBlock(new_block.clone()))
            .await;

        // Save to database
        tokio::spawn(async move {
            Self::save_mine_result(database, persist_block).await;
        });

        // Fair mining delay
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
```

**Important**: Validates the block again before committing to ensure the chain hasn't been updated during mining.

## Mining Interruption

One of the most important features of the mining implementation is **interruption handling**.

### Why Interruption is Needed

If another miner finds a valid block first and broadcasts it to the network, continuing to mine on the old chain would be wasteful. The miner needs to:
1. Stop current mining
2. Accept the new block
3. Return transactions to pool
4. Start mining on the updated chain

### How Interruption Works

**Channel Communication**: `src/node.rs` creates a channel:
```rust
let (block_notify_tx, block_notify_rx) = mpsc::channel::<Option<Block>>(100);
```

**Sending Notification**: When Server receives a new block (`src/server.rs`):
```rust
// Notify miner to stop current mining
let _ = block_tx.send(Some(new_block.clone())).await;
```

**Receiving in Miner**: `src/miner.rs:105-122`
```rust
select! {
    Some(received_option) = self.block_rx.recv() => {
        if let Some(new_block) = received_option {
            miner_info!("Received valid updated state during mining, aborting...");

            // Return transactions to pool (mark as not mined)
            self.transaction_pool
                .lock()
                .await
                .process_mined_transactions(false, &new_block.transactions);

            // Save new block to database
            tokio::spawn(async move {
                Self::save_mine_result(database, persist_block).await;
            });

            break; // Exit mining loop and restart
        }
    }
    _ = tokio::task::yield_now() => {}
}
```

### Transaction Pool Handling

When mining is interrupted:
- **Successful mining** (`true`): `process_mined_transactions(true, ...)` - clears pending transactions
- **Interrupted mining** (`false`): `process_mined_transactions(false, ...)` - returns transactions to pool

This prevents transaction loss when mining is interrupted.

## Miner Rewards

### COINBASE Transaction

A **COINBASE transaction** is a special transaction that creates new coins as a reward for the miner.

**Characteristics**:
- Sender: `"COINBASE"` (special identifier)
- Recipient: Miner's wallet address
- Amount: Fixed reward + transaction fees
- Fee: 0.0 (no fee for coinbase)
- No signature required (sender is not a real wallet)

### Reward Calculation

**Location**: `src/blockchain.rs:36-49`

```rust
pub fn get_miner_transaction(&self, miner_address: String, fees: f64) -> Option<Transaction> {
    if self.total_supply <= MAX_SUPPLY {
        let new_timestamp = chrono::Utc::now().timestamp() as u64;
        return Some(Transaction::new(
            "COINBASE".to_string(),
            miner_address.clone(),
            REWARD as f64 + fees,  // Fixed reward + fees
            0.0,
            new_timestamp as i64,
        ));
    }
    None
}
```

**Reward Components**:
1. **Fixed Reward**: `REWARD` constant = 5 coins (`src/constants.rs:2`)
2. **Transaction Fees**: Sum of all fees from transactions in the block

**Total Reward** = `REWARD + Σ(transaction fees)`

### Supply Limit

**Maximum Supply**: 21,000,000 coins (`src/constants.rs:1`)

When `total_supply` reaches `MAX_SUPPLY`, no more COINBASE transactions are created. Miners would only earn from transaction fees.

**Note**: In the current implementation, `total_supply` tracking is defined but not actively enforced across all operations.

## Block Validation

Before accepting a mined block, it undergoes strict validation.

### Validation Criteria

**Location**: `src/blockchain.rs:61-86`

```rust
pub fn is_valid_new_block(&self, block: &Block) -> bool {
    if let Some(last_block) = self.chain.last() {
        // 1. Validate previous hash
        if block.previous_hash != last_block.hash {
            return false;
        }

        // 2. Validate block hash and PoW
        let calculated_hash = block.calculate_hash();
        if block.hash != calculated_hash
            || !block.hash.starts_with(&"0".repeat(self.difficulty))
        {
            return false;
        }

        // 3. Validate all transactions in the block
        for tx in &block.transactions {
            if tx.sender != "COINBASE" && !tx.verify() {
                return false;
            }
        }

        return true;
    }
    false
}
```

**Checks**:
1. **Previous Hash**: Block's `previous_hash` must match last block's `hash`
2. **Hash Integrity**: Recalculated hash must match block's `hash`
3. **Proof of Work**: Hash must start with required number of zeros (difficulty)
4. **Transaction Signatures**: All non-COINBASE transactions must have valid signatures

If any check fails, the block is rejected.

## Implementation Details

### Incremental Mining

**Why Incremental?**

The original `mine()` function (`block.rs:59-66`) mines in a single blocking loop:
```rust
pub fn mine(&mut self, difficulty: usize) {
    let target = "0".repeat(difficulty);
    while self.hash[..difficulty] != target {
        self.nonce += 1;
        self.hash = self.calculate_hash();
    }
}
```

This blocks the async runtime and prevents handling interruptions.

**Solution**: `mine_step()` (`block.rs:72-75`)
```rust
pub fn mine_step(&mut self) {
    self.nonce += 1;
    self.hash = self.calculate_hash();
}
```

Called in a loop with `tokio::select!` to allow concurrent listening for new blocks.

### Concurrency with tokio::select!

**Location**: `src/miner.rs:105-128`

```rust
select! {
    Some(received_option) = self.block_rx.recv() => {
        // Handle new block from network
    }
    _ = tokio::task::yield_now() => {
        // Yield to allow other tasks to execute
    }
}
```

This allows the miner to:
1. Continue mining (incrementing nonce)
2. Listen for new blocks from the network
3. Yield to other async tasks periodically

### Fair Mining Delay

**Location**: `src/miner.rs:165`

```rust
tokio::time::sleep(Duration::from_secs(2)).await;
```

After successfully mining a block, the miner waits 2 seconds before starting the next block. This:
- Gives other nodes time to receive and process the new block
- Prevents a single fast miner from dominating
- Makes the network more fair for learning purposes

**Note**: Real blockchains have more sophisticated mechanisms (like Bitcoin's difficulty adjustment).

### Database Persistence

**Location**: `src/miner.rs:178-207`

Block and transaction persistence happens asynchronously:
```rust
tokio::spawn(async move {
    Self::save_mine_result(database, persist_block).await;
});
```

This prevents mining from blocking on I/O operations.

### Mining Without Transactions

**Configuration**: `mine_without_transactions` flag

- **`true`**: Mine empty blocks (only COINBASE transaction)
- **`false`**: Wait for transactions before mining

**Use Case**: For testing, you might want to mine without waiting for transactions.

## Performance Considerations

### SHA-256 Performance

Mining is intentionally CPU-intensive. Each iteration:
1. Formats a string with block data
2. Computes SHA-256 hash
3. Checks if hash meets difficulty

With difficulty 5, this typically requires ~1 million iterations.

### Yielding to Runtime

```rust
_ = tokio::task::yield_now() => {}
```

Ensures the miner doesn't monopolize the CPU and allows other tasks to execute.

### Optional Slowdown (Commented Out)

```rust
// _ = tokio::time::sleep(Duration::from_nanos(10)) => {}
```

You can uncomment this to slow down mining for testing or demonstration purposes.

## Summary

**Proof of Work in Artemis Network**:
- Uses SHA-256 hashing
- Requires 5 leading zeros in block hash (difficulty 5)
- Incremental mining with interruption support
- Miner rewards: 5 coins + transaction fees
- COINBASE transactions create new coins
- Maximum supply: 21,000,000 coins
- Fair mining with 2-second delay between blocks
- Full validation before accepting blocks

This implementation demonstrates the core concepts of proof-of-work mining in a simplified, educational context.
