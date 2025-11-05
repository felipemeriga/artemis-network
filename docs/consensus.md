# Consensus & Synchronization

This document explains how Artemis Network achieves consensus across nodes and keeps blockchains synchronized.

## Table of Contents
- [What is Consensus?](#what-is-consensus)
- [Longest-Chain Rule](#longest-chain-rule)
- [Synchronization Process](#synchronization-process)
- [Chain Validation](#chain-validation)
- [Chain Replacement](#chain-replacement)
- [Coordination with Mining](#coordination-with-mining)
- [Fork Resolution](#fork-resolution)

## What is Consensus?

**Consensus** is the mechanism by which distributed nodes agree on the current state of the blockchain. In a decentralized network:
- Multiple nodes maintain copies of the blockchain
- Nodes may mine blocks simultaneously
- Network latency causes different nodes to have different views
- A consensus rule determines which chain is "correct"

**Artemis Network's Consensus**: **Longest Valid Chain Rule** (also used by Bitcoin)

## Longest-Chain Rule

The **longest-chain rule** states that the valid chain with the most blocks (greatest cumulative work) is considered the canonical chain.

### Core Principle

**Location**: `src/sync.rs:80-83`

```rust
if peer_chain.len() > max_length && Blockchain::is_valid_chain(&peer_chain) {
    max_length = peer_chain.len();
    longest_chain = Some(peer_chain);
}
```

**Rules**:
1. **Length**: Chain with more blocks wins
2. **Validity**: Chain must be valid (all blocks properly linked and mined)
3. **Replacement**: Shorter local chain is replaced with longer valid chain

### Why Longest Chain?

**Security**:
- An attacker needs to outpace the entire network's mining power
- The longer the chain, the more computational work invested
- Makes it exponentially harder to rewrite history

**Simplicity**:
- Clear, deterministic rule
- No need for voting or complex coordination
- Works well with proof-of-work

**Decentralization**:
- No central authority needed
- Any node can independently verify which chain is longest
- Nodes naturally converge to same chain

### Chain Length vs. Chain Work

**Note**: Artemis Network uses **chain length** (number of blocks) rather than **total cumulative work** (sum of difficulty).

This simplification works because:
- Difficulty is constant (5 leading zeros)
- All blocks have equal "weight"
- Longer chain = more work

In real blockchains like Bitcoin:
- Difficulty adjusts over time
- Chain with most **cumulative difficulty** wins
- Not necessarily the chain with most blocks

## Synchronization Process

The `Sync` component periodically synchronizes the local blockchain with peers.

**Location**: `src/sync.rs:14-34`

```rust
pub struct Sync {
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<HashSet<String>>>,
    block_tx: Arc<Mutex<Sender<Option<Block>>>>,
    database: Arc<Mutex<Database>>,
}
```

### Sync Loop

**Location**: `src/sync.rs:36-128`

```rust
pub async fn sync_with_peers(&mut self, ...) {
    loop {
        // 1. Wait for peer discovery
        if !*first_discover_done.lock().await {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // 2. Request blockchain from all peers
        let peers = { self.peers.lock().await.clone() };
        let mut longest_chain = None;
        let mut max_length = self.blockchain.read().await.get_chain().len();

        for peer_address in peers {
            if peer_address == tcp_address {
                continue; // Skip self
            }

            // 3. Connect to peer and request blockchain
            if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                let request = Request {
                    command: "get_blockchain".to_string(),
                    data: "".to_string(),
                };

                stream.write_all(marshalled_request.as_bytes()).await?;

                // 4. Receive and validate peer's chain
                let peer_chain = Self::receive_blockchain(stream).await;
                if peer_chain.len() > max_length && Blockchain::is_valid_chain(&peer_chain) {
                    max_length = peer_chain.len();
                    longest_chain = Some(peer_chain);
                }
            } else {
                // Remove dead peer
                self.peers.lock().await.remove(&peer_address);
            }
        }

        // 5. Replace chain if longer valid chain found
        if let Some(new_chain) = longest_chain {
            sync_info!("Replacing chain with longer chain from peer.");
            self.blockchain.write().await.replace_chain(new_chain.clone());

            // 6. Notify miner to stop current mining
            self.block_tx
                .lock()
                .await
                .send(Some(self.blockchain.read().await.get_last_block().clone()))
                .await?;

            // 7. Persist new chain to database
            self.database
                .lock()
                .await
                .store_blocks_and_transactions(new_chain.clone())?;
        } else {
            sync_info!("Local chain is the longest.");
        }

        // 8. Mark first sync as done
        if !*first_sync_done.lock().await {
            *first_sync_done.lock().await = true;
        }

        // 9. Sleep before next sync
        tokio::time::sleep(Duration::from_secs(120)).await;
    }
}
```

### Synchronization Steps

**1. Wait for Peer Discovery** (`sync.rs:43-48`):
```rust
if !*first_discover_done.lock().await {
    tokio::time::sleep(Duration::from_secs(1)).await;
    continue;
}
```

Ensures the node has discovered peers before attempting sync.

**2. Request Blockchain from Each Peer** (`sync.rs:54-90`):
```rust
for peer_address in peers {
    if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
        let request = Request {
            command: "get_blockchain".to_string(),
            data: "".to_string(),
        };
        stream.write_all(marshalled_request.as_bytes()).await?;
        let peer_chain = Self::receive_blockchain(stream).await;
        // ...
    }
}
```

Connects to each peer and sends `GET_BLOCKCHAIN` request.

**3. Receive and Validate** (`sync.rs:80-83`):
```rust
if peer_chain.len() > max_length && Blockchain::is_valid_chain(&peer_chain) {
    max_length = peer_chain.len();
    longest_chain = Some(peer_chain);
}
```

Keeps track of the longest valid chain encountered.

**4. Replace Local Chain** (`sync.rs:92-97`):
```rust
if let Some(new_chain) = longest_chain {
    self.blockchain.write().await.replace_chain(new_chain.clone());
}
```

Replaces local chain if a longer valid chain was found.

**5. Notify Miner** (`sync.rs:99-104`):
```rust
self.block_tx
    .lock()
    .await
    .send(Some(self.blockchain.read().await.get_last_block().clone()))
    .await?;
```

Sends notification to miner to interrupt current mining and restart on updated chain.

**6. Persist to Database** (`sync.rs:105-116`):
```rust
self.database
    .lock()
    .await
    .store_blocks_and_transactions(new_chain.clone())?;
```

Saves the new blockchain to persistent storage.

**7. Periodic Sync** (`sync.rs:126`):
```rust
tokio::time::sleep(Duration::from_secs(120)).await;
```

Waits 120 seconds (2 minutes) before next sync cycle.

### Receiving Blockchain Data

**Location**: `src/sync.rs:130-164`

```rust
pub async fn receive_blockchain(mut stream: TcpStream) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut buffer = String::new();
    let mut temp = [0u8; 1024]; // Read in chunks

    while let Ok(n) = stream.read(&mut temp).await {
        if n == 0 {
            break; // Connection closed
        }

        buffer.push_str(&String::from_utf8_lossy(&temp[..n]));

        // Process complete blocks (delimited by <END_BLOCK>)
        while let Some(pos) = buffer.find("<END_BLOCK>\n") {
            let extracted_block = buffer[..pos].trim().to_string();
            buffer.drain(..pos + "<END_BLOCK>\n".len());

            if extracted_block == "<END_CHAIN>" {
                return blocks; // End of chain
            }

            // Deserialize block
            match from_str::<Block>(&extracted_block) {
                Ok(block) => blocks.push(block),
                Err(e) => eprintln!("Failed to deserialize block: {}", e),
            }
        }
    }

    blocks
}
```

**Protocol**:
- Blocks are sent as JSON strings
- Each block is delimited by `<END_BLOCK>\n`
- Chain ends with `<END_CHAIN>` marker
- Blocks are streamed (not sent all at once)

### Sending Blockchain Data

**Location**: `src/server.rs:140-161`

```rust
GET_BLOCKCHAIN => {
    let chain = { self.blockchain.read().await.get_chain() };

    for block in chain {
        let block_json_string = to_string(&block)?;
        let block_chunk = format!("{}{}\n", block_json_string, "<END_BLOCK>");

        stream.write_all(block_chunk.as_bytes()).await?;
    }

    // Send end marker
    stream.write_all(b"<END_CHAIN><END_BLOCK>\n").await?;
}
```

**Streaming Benefits**:
- Doesn't require loading entire chain into memory
- Can start processing blocks before entire chain is received
- Handles large blockchains efficiently

## Chain Validation

Before accepting a new chain, it must be validated to ensure integrity.

**Location**: `src/blockchain.rs:23-32`

```rust
pub fn is_valid_chain(chain: &[Block]) -> bool {
    for i in 1..chain.len() {
        if chain[i].previous_hash != chain[i - 1].hash
            || chain[i].hash != chain[i].calculate_hash()
        {
            return false;
        }
    }
    true
}
```

### Validation Checks

**For Each Block** (starting from index 1):

**1. Previous Hash Linkage**:
```rust
chain[i].previous_hash != chain[i - 1].hash
```

Each block's `previous_hash` must match the previous block's `hash`.

**2. Hash Integrity**:
```rust
chain[i].hash != chain[i].calculate_hash()
```

Recalculate the hash and verify it matches the stored hash.

**What's NOT Checked** (in `is_valid_chain`):
- Proof-of-work difficulty (leading zeros)
- Transaction signatures
- Block timestamps

**Why?**
- `is_valid_chain` is a quick validation for sync
- Full block validation happens when blocks are added individually
- Trusts that honest peers have already validated these

### Individual Block Validation

When receiving a new block (not via sync), full validation is performed.

**Location**: `src/blockchain.rs:61-86`

```rust
pub fn is_valid_new_block(&self, block: &Block) -> bool {
    if let Some(last_block) = self.chain.last() {
        // 1. Validate previous hash
        if block.previous_hash != last_block.hash {
            return false;
        }

        // 2. Validate hash integrity and PoW
        let calculated_hash = block.calculate_hash();
        if block.hash != calculated_hash
            || !block.hash.starts_with(&"0".repeat(self.difficulty))
        {
            return false;
        }

        // 3. Validate all transactions
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

**Full Validation Includes**:
1. Previous hash linkage
2. Hash integrity
3. **Proof-of-work** (hash starts with required zeros)
4. **Transaction signatures** (all non-COINBASE transactions)

## Chain Replacement

When a longer valid chain is found, the local chain is replaced.

**Location**: `src/blockchain.rs:94-96`

```rust
pub fn replace_chain(&mut self, new_chain: Vec<Block>) {
    self.chain = new_chain;
}
```

### Replacement Process

**In Sync** (`src/sync.rs:92-116`):

```rust
if let Some(new_chain) = longest_chain {
    // 1. Replace blockchain
    self.blockchain.write().await.replace_chain(new_chain.clone());

    // 2. Notify miner (interrupt current mining)
    self.block_tx
        .lock()
        .await
        .send(Some(self.blockchain.read().await.get_last_block().clone()))
        .await?;

    // 3. Persist to database
    self.database
        .lock()
        .await
        .store_blocks_and_transactions(new_chain.clone())?;
}
```

### Impact on Other Components

**Mining**:
- Miner receives notification via `block_tx` channel
- Current mining is interrupted
- Transactions are returned to pool
- Mining restarts on new chain tip

See [Mining Documentation](mining.md#mining-interruption) for details.

**Transaction Pool**:
- Currently, the pool is NOT automatically cleared on chain replacement
- This is a limitation in the current implementation
- Ideally, transactions in new chain should be removed from pool

**Database**:
- New chain is persisted, overwriting old chain
- Ensures database matches in-memory blockchain state

## Coordination with Mining

Synchronization and mining must be carefully coordinated to prevent issues.

### Startup Coordination

**Location**: `src/node.rs`

```rust
let first_discover_done = Arc::new(Mutex::new(false));
let first_sync_done = Arc::new(Mutex::new(false));
```

**Coordination Flags**:
1. `first_discover_done`: Set by Discover component when initial peer discovery completes
2. `first_sync_done`: Set by Sync component when initial sync completes

**Startup Sequence**:
```
1. Start Discover
2. Discover finds peers → set first_discover_done
3. Start Sync (waits for first_discover_done)
4. Sync completes → set first_sync_done
5. Start Miner (waits for first_sync_done)
```

**Why This Order?**
- **Discover before Sync**: Need peers to sync with
- **Sync before Mining**: Prevents mining on outdated chain
- **Mining last**: Ensures node has latest state before mining

### Runtime Coordination

**Sync → Miner Communication**:

**Channel**: `mpsc::channel<Option<Block>>`

**Sender**: Sync component (when chain is replaced)
**Receiver**: Miner component

**Location** (`src/sync.rs:99-104`):
```rust
self.block_tx
    .lock()
    .await
    .send(Some(self.blockchain.read().await.get_last_block().clone()))
    .await?;
```

**Miner Response** (`src/miner.rs:107-122`):
```rust
select! {
    Some(received_option) = self.block_rx.recv() => {
        if let Some(new_block) = received_option {
            // Interrupt mining
            self.transaction_pool
                .lock()
                .await
                .process_mined_transactions(false, &new_block.transactions);
            break; // Restart mining
        }
    }
}
```

**Flow**:
1. Sync finds longer chain
2. Sync replaces local chain
3. Sync sends notification to Miner
4. Miner stops current mining
5. Miner returns transactions to pool
6. Miner starts mining on new chain tip

### Preventing Race Conditions

**Double-Check Before Committing**:

**Location** (`src/miner.rs:135-169`):
```rust
if let Some(new_block) = mined_block {
    let mut blockchain_write = self.blockchain.write().await;

    // Validate again in case chain was updated during mining
    if blockchain_write.is_valid_new_block(&new_block) {
        blockchain_write.add_block(new_block.clone());
        // ... broadcast, save, etc.
    } else {
        miner_info!("Mining became invalid due to a chain update.");
        continue; // Restart mining
    }
}
```

**Why?**
- Chain might have been replaced during mining
- `is_valid_new_block` checks if block's `previous_hash` matches current chain tip
- If chain changed, `previous_hash` won't match → block rejected
- Prevents adding block to wrong chain

## Fork Resolution

**Forks** occur when two miners find valid blocks simultaneously.

### How Forks Happen

```
Initial State:
... → Block 10

Miner A finds Block 11a
Miner B finds Block 11b

Network Split:
... → Block 10 → Block 11a (Some nodes)
... → Block 10 → Block 11b (Other nodes)
```

### Fork Resolution

**Longest Chain Wins**:

```
Time 1:
... → Block 10 → Block 11a (length: 12)
... → Block 10 → Block 11b (length: 12)

Time 2 (Miner on chain A finds Block 12a):
... → Block 10 → Block 11a → Block 12a (length: 13) ← WINNER
... → Block 10 → Block 11b (length: 12) ← ORPHANED

All nodes sync to longer chain:
... → Block 10 → Block 11a → Block 12a
```

### Orphaned Blocks

**Orphaned Block**: A valid block that is not part of the longest chain

**What Happens to Orphaned Blocks**:
- **Block**: Discarded
- **Transactions**: Returned to pool (if not in winning chain)
- **Miner Reward**: Lost (not in canonical chain)

**Current Limitation**: Artemis Network doesn't automatically return transactions from orphaned blocks to the pool.

### Fork Probability

**Factors Affecting Forks**:
- **Network latency**: Higher latency → more forks
- **Mining rate**: Faster block time → more forks
- **Peer count**: More peers → faster convergence

**Artemis Network**:
- 2-second delay after mining
- 120-second sync interval
- Small network size
- Relatively low fork probability

## Summary

**Consensus Mechanism**: Longest Valid Chain Rule

**Key Components**:
- **Sync**: Periodically requests blockchains from peers
- **Validation**: Ensures chain integrity (hash linkage, PoW, signatures)
- **Replacement**: Replaces shorter chain with longer valid chain
- **Coordination**: Flags ensure proper startup order
- **Mining Integration**: Miner interrupted when new chain found

**Synchronization Flow**:
1. Wait for peer discovery
2. Request blockchain from all peers
3. Validate each peer's chain
4. Keep track of longest valid chain
5. Replace local chain if longer found
6. Notify miner to interrupt
7. Persist new chain to database
8. Sleep 120 seconds
9. Repeat

**Security**:
- Attackers must outpace honest network
- Longer chain = more cumulative work
- Reorganizing deep blocks exponentially harder

**Limitations**:
- Uses chain length instead of cumulative difficulty
- No automatic transaction pool cleanup on reorg
- No deep reorganization protection (accepts any longer valid chain)
- Trusts peer validation for sync (doesn't re-verify PoW)

This design demonstrates the core principles of blockchain consensus in a simplified, educational context.
