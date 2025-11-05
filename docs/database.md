# Database & Storage

This document explains the persistent storage implementation in Artemis Network using Sled embedded database.

## Table of Contents
- [Database Overview](#database-overview)
- [Sled Embedded Database](#sled-embedded-database)
- [Storage Schema](#storage-schema)
- [Block Storage](#block-storage)
- [Transaction Storage](#transaction-storage)
- [Indexing Strategy](#indexing-strategy)
- [Balance Calculation](#balance-calculation)
- [Development Mode](#development-mode)

## Database Overview

Artemis Network uses **Sled**, an embedded key-value database, for persistent storage.

**Purpose**:
- Persist blockchain data across restarts
- Store blocks and transactions
- Index transactions by wallet address
- Calculate wallet balances efficiently

**Location**: `src/db.rs:6-8`

```rust
pub struct Database {
    pub db: Db,  // Sled database instance
}
```

## Sled Embedded Database

### What is Sled?

**Sled** is a modern embedded database for Rust applications.

**Characteristics**:
- **Embedded**: Runs in-process (no separate server)
- **Key-Value Store**: Simple get/put interface
- **ACID**: Atomicity, Consistency, Isolation, Durability
- **Thread-Safe**: Lock-free concurrency
- **Crash-Safe**: Uses log-structured storage

**Documentation**: https://docs.rs/sled

### Why Sled?

**Embedded**:
- No external database server required
- Simple deployment
- Low overhead

**Performance**:
- Fast reads and writes
- Lock-free data structures
- Efficient for append-heavy workloads (blockchain)

**Reliability**:
- Crash-safe by design
- Automatic recovery
- ACID transactions

**Rust Native**:
- Type-safe API
- Zero-cost abstractions
- Memory safe

### Database Initialization

**Location**: `src/db.rs:11-31`

```rust
pub fn new(node_id: String) -> Self {
    let db_path_for_node = format!("./database/blockchain-db-{}", node_id);

    // Dev mode: recreate DB on startup
    #[cfg(feature = "dev")]
    {
        use std::fs;
        if fs::metadata(db_path_for_node.clone()).is_ok() {
            fs::remove_dir_all(db_path_for_node.clone())?;
        }
    }

    // Open database
    let db = sled::open(db_path_for_node)?;
    Self { db }
}
```

### Per-Node Databases

**Path Format**: `./database/blockchain-db-{node_id}`

**Examples**:
- Node 1: `./database/blockchain-db-master`
- Node 2: `./database/blockchain-db-node2`
- Node 3: `./database/blockchain-db-node3`

**Why Separate Databases?**
- Multiple nodes can run on same machine
- Each node has independent storage
- No database conflicts
- Easier testing and development

## Storage Schema

Sled is a key-value store, so we design a schema using key prefixes.

### Key Prefixes

| Prefix | Purpose | Example Key |
|--------|---------|-------------|
| `block:` | Block by hash | `block:00000abc...` |
| `{tx_hash}` | Transaction by hash | `a1b2c3d4...` (no prefix) |
| `addr_{address}` | Transaction index by address | `addr_9f86d081...` |

### Data Serialization

**Blocks**: JSON (serde_json)
**Transactions**: Binary (bincode)
**Indices**: Binary (bincode)

**Why Different Formats?**

**JSON for Blocks**:
- Human-readable
- Debugging friendly
- Larger size acceptable (blocks are infrequent)

**Bincode for Transactions**:
- Compact binary format
- Faster serialization/deserialization
- More efficient for frequent operations

## Block Storage

### Storing Blocks

**Location**: `src/db.rs:116-121`

```rust
pub fn store_block(&self, block: &Block) -> Result<(), DatabaseError> {
    let key = format!("block:{}", block.hash);
    let value = serde_json::to_vec(block)?;
    self.db.insert(key, value)?;
    Ok(())
}
```

**Key Format**: `block:{block_hash}`

**Example**:
```
Key: block:00000abc123def456...
Value: {"index":10,"timestamp":1699876543,"transactions":[...],...}
```

**Operations**:
1. Create key with `block:` prefix + block hash
2. Serialize block to JSON bytes
3. Insert into database

### Retrieving Blocks

**By Hash** (`src/db.rs:123-135`):
```rust
pub fn get_block(&self, block_hash: &str) -> Option<Block> {
    let key = format!("block:{}", block_hash);
    if let Ok(Some(value)) = self.db.get(key) {
        let block: Block = serde_json::from_slice(&value)?;
        return Some(block);
    }
    None
}
```

**All Blocks** (`src/db.rs:137-150`):
```rust
pub fn get_all_blocks(&self) -> Vec<Block> {
    let mut blocks: Vec<_> = self
        .db
        .scan_prefix("block:") // Get all keys starting with "block:"
        .filter_map(|item| {
            item.ok()
                .and_then(|(_, value)| serde_json::from_slice::<Block>(&value).ok())
        })
        .collect();

    // Sort by index
    blocks.sort_by(|a, b| a.index.cmp(&b.index));
    blocks
}
```

**Prefix Scan**:
- `scan_prefix("block:")` returns iterator over all blocks
- Efficiently retrieves all entries with matching prefix
- Avoids scanning entire database

### Bulk Block Storage

**Location**: `src/db.rs:153-166`

```rust
pub fn store_blocks_and_transactions(&self, blocks: Vec<Block>) -> Result<(), DatabaseError> {
    for block in blocks {
        // Store block
        self.store_block(&block)?;

        // Store all transactions in block
        for tx in &block.transactions {
            let tx_hash = tx.hash();
            self.store_transaction(tx, &tx_hash)?;
        }
    }
    Ok(())
}
```

**Use Case**: When receiving full blockchain from peer during sync.

**Process**:
1. Store each block
2. Store all transactions in each block
3. Index transactions by sender/recipient

## Transaction Storage

### Storing Transactions

**Location**: `src/db.rs:33-47`

```rust
pub fn store_transaction(&self, tx: &Transaction, tx_hash: &str) -> Result<(), DatabaseError> {
    // 1. Store transaction by hash
    self.db.insert(
        tx_hash,
        bincode::serialize(tx)?,
    )?;

    // 2. Index by sender
    let sender_key = format!("addr_{}", tx.sender);
    self.add_transaction_to_index(&sender_key, tx_hash)?;

    // 3. Index by recipient
    let recipient_key = format!("addr_{}", tx.recipient);
    self.add_transaction_to_index(&recipient_key, tx_hash)?;

    Ok(())
}
```

**Three Operations**:
1. **Direct Storage**: Store full transaction by hash
2. **Sender Index**: Add to sender's transaction list
3. **Recipient Index**: Add to recipient's transaction list

### Transaction Key Format

**Direct Lookup**:
```
Key: a1b2c3d4e5f6... (transaction hash)
Value: [binary encoded transaction]
```

**No prefix** - transaction hash is the key directly.

### Retrieving Transactions

**By Hash** (`src/db.rs:66-73`):
```rust
pub fn get_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>, DatabaseError> {
    match self.db.get(tx_hash)? {
        Some(value) => Ok(Some(bincode::deserialize(&value)?)),
        None => Ok(None),
    }
}
```

**By Wallet** (`src/db.rs:75-96`):
```rust
pub fn get_transactions_by_wallet(&self, wallet: &str) -> Result<Vec<Transaction>, DatabaseError> {
    let key = format!("addr_{}", wallet);

    match self.db.get(key)? {
        Some(value) => {
            // Get list of transaction hashes
            let tx_hashes: Vec<String> = bincode::deserialize(&value)?;

            // Fetch each transaction
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
```

**Process**:
1. Look up index: `addr_{wallet_address}` → list of transaction hashes
2. For each hash, fetch full transaction
3. Return vector of transactions

## Indexing Strategy

### Why Indexing?

**Problem**: Finding all transactions for a wallet requires scanning entire blockchain.

**Solution**: Maintain index mapping wallet addresses to transaction hashes.

### Index Structure

**Key Format**: `addr_{wallet_address}`

**Value**: Vector of transaction hashes (binary encoded)

**Example**:
```
Key: addr_9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
Value: ["a1b2c3d4...", "e5f6g7h8...", "i9j0k1l2..."]
```

### Adding to Index

**Location**: `src/db.rs:49-64`

```rust
pub fn add_transaction_to_index(&self, key: &str, tx_hash: &str) -> Result<(), DatabaseError> {
    // 1. Get existing index
    let mut tx_list: Vec<String> = match self.db.get(key)? {
        Some(value) => bincode::deserialize(&value)?,
        None => vec![],
    };

    // 2. Add transaction hash if not already present
    if !tx_list.contains(&tx_hash.to_string()) {
        tx_list.push(tx_hash.to_string());

        // 3. Save updated index
        self.db.insert(
            key,
            bincode::serialize(&tx_list)?,
        )?;
    }

    Ok(())
}
```

**Process**:
1. Retrieve existing list (or empty if new address)
2. Check for duplicates
3. Append transaction hash
4. Store updated list

**Duplicate Prevention**: Ensures each transaction only indexed once per address.

### Dual Indexing

Each transaction is indexed **twice**:

**Sender Index**:
```
addr_{sender_address} → [tx1, tx2, tx3, ...]
```

**Recipient Index**:
```
addr_{recipient_address} → [tx1, tx4, tx5, ...]
```

**Why Both?**
- Calculate balance: need both incoming and outgoing
- Transaction history: user wants to see all activity
- Efficiency: O(1) lookup instead of O(n) scan

### Example

**Transaction**:
```json
{
  "sender": "alice_address",
  "recipient": "bob_address",
  "amount": 10.0,
  "fee": 0.1
}
```

**Stored As**:
```
Key: tx_hash_123
Value: [binary encoded transaction]

Key: addr_alice_address
Value: ["...", "tx_hash_123", "..."]

Key: addr_bob_address
Value: ["...", "tx_hash_123", "..."]
```

## Balance Calculation

**Location**: `src/db.rs:98-114`

```rust
pub fn get_wallet_balance(&self, wallet_address: &str) -> Result<f64, DatabaseError> {
    // 1. Get all transactions for wallet
    let transactions = self.get_transactions_by_wallet(wallet_address)?;

    let mut balance: f64 = 0.0;

    // 2. Sum incoming and outgoing amounts
    transactions.iter().for_each(|tx| {
        if tx.recipient == wallet_address {
            balance += tx.amount.into_inner(); // Received
        }
        if tx.sender == wallet_address {
            balance -= tx.amount.into_inner(); // Sent
            balance -= tx.fee.into_inner();     // Fee paid
        }
    });

    Ok(balance)
}
```

### Calculation Logic

**Starting Balance**: `0.0`

**For Each Transaction**:
- **If recipient**: `balance += amount` (coins received)
- **If sender**: `balance -= amount` (coins sent)
- **If sender**: `balance -= fee` (transaction fee paid)

**Final Balance**: Sum of all incoming minus all outgoing

### Example Calculation

**Transactions**:
```
TX1: COINBASE → Alice (50 coins) [mining reward]
TX2: Alice → Bob (10 coins, fee 0.1)
TX3: Charlie → Alice (5 coins, fee 0.1)
TX4: Alice → Dave (20 coins, fee 0.2)
```

**Alice's Balance**:
```
Start: 0.0
TX1 (received): +50.0 = 50.0
TX2 (sent): -10.0 - 0.1 = 39.9
TX3 (received): +5.0 = 44.9
TX4 (sent): -20.0 - 0.2 = 24.7

Final Balance: 24.7 coins
```

### Performance Consideration

**Complexity**: O(n) where n = number of transactions involving address

**Trade-off**:
- Simple implementation
- No separate balance storage to maintain
- Always accurate (calculated from source of truth)
- Acceptable performance for educational blockchain

**Production Optimization**:
- Cache balances
- Update incrementally
- Use UTXO model (like Bitcoin)
- Maintain balance in account state (like Ethereum)

## Development Mode

### Dev Feature Flag

**Location**: `src/db.rs:14-24`

```rust
#[cfg(feature = "dev")]
{
    use std::fs;

    let path = db_path_for_node.clone();
    // Remove old database directory if it exists
    if fs::metadata(path.clone()).is_ok() {
        fs::remove_dir_all(path)?;
    }
}
```

**Compilation**:
```bash
# Dev mode (recreate DB on startup)
cargo run --features dev -- --config=config-1.yaml

# Normal mode (persist DB)
cargo run -- --config=config-1.yaml
```

**Behavior**:
- **Dev mode**: Deletes database folder on startup, starts fresh
- **Normal mode**: Reuses existing database if present

**Use Cases**:
- **Development**: Clean slate for testing
- **Testing**: Consistent initial state
- **Production**: Maintain historical data across restarts

### Database Location

**Directory Structure**:
```
artemis-network/
├─ database/
│  ├─ blockchain-db-master/     (Node 1)
│  ├─ blockchain-db-node2/      (Node 2)
│  └─ blockchain-db-node3/      (Node 3)
├─ config/
├─ src/
└─ ...
```

**Files in Database Directory**:
- `conf`: Sled configuration
- `db`: Main database file
- `snap.*`: Snapshot files (for crash recovery)

## Storage Schema Summary

### Complete Key-Value Schema

| Key Pattern | Value Type | Description | Example |
|-------------|------------|-------------|---------|
| `block:{hash}` | Block (JSON) | Block by hash | `block:00000abc...` |
| `{tx_hash}` | Transaction (bincode) | Transaction by hash | `a1b2c3d4...` |
| `addr_{address}` | Vec<String> (bincode) | Transaction hashes for address | `addr_9f86d081...` |

### Data Flow

**Storing a New Block**:
```
1. store_block(block)
   → db["block:{hash}"] = JSON(block)

2. For each transaction in block:
   a. store_transaction(tx, hash)
      → db["{tx_hash}"] = bincode(tx)
      → db["addr_{sender}"] += [tx_hash]
      → db["addr_{recipient}"] += [tx_hash]
```

**Querying Wallet Balance**:
```
1. get_transactions_by_wallet(address)
   → tx_hashes = db["addr_{address}"]
   → For each hash: transactions.push(db[hash])

2. get_wallet_balance(address)
   → transactions = get_transactions_by_wallet(address)
   → balance = sum(received) - sum(sent) - sum(fees)
```

## Limitations & Future Improvements

### Current Limitations

**No UTXO Model**:
- Uses account-based balance calculation
- O(n) balance queries (scan all transactions)
- No unspent transaction output tracking

**No State Pruning**:
- All transactions stored forever
- Database grows indefinitely
- No archival nodes vs. full nodes distinction

**No Merkle Trees**:
- Cannot prove transaction inclusion without full block
- SPV (Simplified Payment Verification) not supported

**No Database Migrations**:
- Schema changes require fresh database
- No versioning or upgrade path

### Potential Improvements

**Caching**:
- Cache recent balances
- Cache frequently accessed blocks/transactions
- Reduce database reads

**Batch Operations**:
- Transaction support for atomic writes
- Bulk inserts for better performance

**Compression**:
- Compress old blocks
- Archive historical data

**Separate Stores**:
- Hot storage (recent blocks)
- Cold storage (archived blocks)
- Balance cache (current balances)

**UTXO Model**:
- Track unspent outputs
- O(1) balance queries
- Better concurrency

## Summary

**Database**: Sled embedded key-value store

**Storage Schema**:
- Blocks: `block:{hash}` → JSON
- Transactions: `{hash}` → bincode
- Indices: `addr_{address}` → Vec<tx_hash>

**Key Operations**:
- Store block: O(1)
- Store transaction: O(1) + 2 × index updates
- Get block: O(1)
- Get transaction: O(1)
- Get wallet transactions: O(k) where k = transaction count
- Calculate balance: O(k) where k = transaction count

**Features**:
- Per-node databases (isolated storage)
- Dual indexing (sender + recipient)
- Balance calculation from transaction history
- Dev mode (fresh database on startup)
- Crash-safe persistent storage

**Trade-offs**:
- Simplicity over optimization
- Easy to understand and maintain
- Suitable for educational purposes
- Not production-optimized

This storage design demonstrates the fundamentals of blockchain persistence in a simplified, educational implementation using an embedded database.
