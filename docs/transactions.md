# Transactions

This document explains how transactions work in Artemis Network, from creation to inclusion in a block.

## Table of Contents
- [What is a Transaction?](#what-is-a-transaction)
- [Transaction Structure](#transaction-structure)
- [Transaction Types](#transaction-types)
- [Transaction Lifecycle](#transaction-lifecycle)
- [Transaction Prioritization](#transaction-prioritization)
- [Signing and Verification](#signing-and-verification)
- [Transaction Validation](#transaction-validation)
- [Double-Spend Prevention](#double-spend-prevention)

## What is a Transaction?

A **transaction** represents a transfer of value from one wallet to another. Each transaction:
- Transfers a specified amount from sender to recipient
- Includes a fee to incentivize miners
- Is cryptographically signed to prove ownership
- Is immutable once included in a block

## Transaction Structure

**Location**: `src/transaction.rs:23-34`

```rust
pub struct Transaction {
    pub sender: String,         // Sender's wallet address
    pub recipient: String,      // Recipient's wallet address
    pub amount: OrderedFloat<f64>,  // Amount to transfer
    pub fee: OrderedFloat<f64>,     // Transaction fee
    pub timestamp: i64,             // Unix timestamp
    pub signature: Option<String>,  // ECDSA signature (hex-encoded)
}
```

### Fields Explained

| Field | Type | Description |
|-------|------|-------------|
| `sender` | String | Wallet address of sender (SHA-256 hash of public key) |
| `recipient` | String | Wallet address of recipient |
| `amount` | OrderedFloat<f64> | Amount of coins to transfer |
| `fee` | OrderedFloat<f64> | Fee paid to miner for processing |
| `timestamp` | i64 | Unix timestamp when transaction was created |
| `signature` | Option<String> | ECDSA signature proving sender owns the private key |

### Why OrderedFloat?

**Location**: `src/transaction.rs:2`

```rust
use ordered_float::OrderedFloat;
```

Rust's `f64` type doesn't implement `Ord` (total ordering) because of special values like `NaN`. Since transactions need to be sorted by fee in a priority queue, we use `OrderedFloat<f64>` which provides deterministic ordering.

**Custom Serde Implementation** (`src/transaction.rs:36-54`): Serializes/deserializes as regular `f64` in JSON while maintaining ordering in Rust.

## Transaction Types

### 1. Regular Transactions

**Characteristics**:
- Sender: Wallet address
- Requires valid ECDSA signature
- Deducted from sender's balance
- Added to recipient's balance

**Example**:
```json
{
  "sender": "8f4a7b2c...",
  "recipient": "9e5d6f3a...",
  "amount": 10.5,
  "fee": 0.1,
  "timestamp": 1699876543,
  "signature": "3045022100..."
}
```

### 2. COINBASE Transactions

**Characteristics**:
- Sender: `"COINBASE"` (special identifier)
- Recipient: Miner's wallet address
- Amount: Block reward + transaction fees
- Fee: 0.0
- Signature: `None` (no signature required)

**Purpose**: Create new coins as mining rewards

**Location**: Created in `src/blockchain.rs:36-49`

```rust
pub fn get_miner_transaction(&self, miner_address: String, fees: f64) -> Option<Transaction> {
    if self.total_supply <= MAX_SUPPLY {
        return Some(Transaction::new(
            "COINBASE".to_string(),
            miner_address.clone(),
            REWARD as f64 + fees,  // 5 + fees
            0.0,
            new_timestamp as i64,
        ));
    }
    None
}
```

**Special Handling** (`src/transaction.rs:126-128`):
```rust
if self.sender == "COINBASE" {
    return true; // No signature verification needed
}
```

## Transaction Lifecycle

### 1. Creation

**Options**:

**A. Create Unsigned Transaction** (`src/transaction.rs:81-90`):
```rust
let tx = Transaction::new(
    sender_address,
    recipient_address,
    amount,
    fee,
    timestamp
);
```

**B. Create via HTTP API**:
```bash
POST /transaction/sign-and-submit
{
  "transaction": {
    "sender": "...",
    "recipient": "...",
    "amount": 10.0,
    "fee": 0.1,
    "timestamp": 1699876543
  },
  "publicKeyHex": "...",
  "privateKeyHex": "..."
}
```

⚠️ **Warning**: Sending private keys over HTTP is **insecure** and for **learning purposes only**!

### 2. Signing

**Location**: `src/transaction.rs:93-121`

```rust
pub fn sign(&mut self, wallet: &Wallet) {
    let secp = Secp256k1::new();

    // 1. Create message data
    let message_data = format!(
        "{}:{}:{}:{}:{}",
        self.sender, self.recipient, self.amount, self.fee, self.timestamp
    );

    // 2. Hash the message
    let message_hash = Sha256::digest(message_data.as_bytes());
    let message = Message::from_digest(<[u8; 32]>::from(message_hash));

    // 3. Sign with ECDSA
    let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &wallet.private_key);

    // 4. Serialize signature with recovery ID
    let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();
    let mut sig_with_recovery = sig_bytes.to_vec();
    sig_with_recovery.push(recovery_id as u8);

    // 5. Store as hex string
    self.signature = Some(hex::encode(sig_with_recovery));
}
```

**Signing Process**:
1. Concatenate transaction fields (sender, recipient, amount, fee, timestamp)
2. Hash with SHA-256
3. Create ECDSA signature using sender's private key (secp256k1 curve)
4. Append recovery ID to signature (allows public key recovery)
5. Encode as hexadecimal string

**Why Recoverable Signature?**
- Allows verifier to recover the public key from signature
- No need to include public key in transaction
- Saves space and simplifies structure

### 3. Submission

**HTTP Endpoint**: `POST /transaction/submit` (`src/handler.rs:11-51`)

```rust
pub async fn submit_transaction(
    handler: web::Data<Arc<ServerHandler>>,
    transaction_request: web::Json<Transaction>,
) -> impl Responder {
    let tx = transaction_request.into_inner();

    // 1. Verify signature
    if tx.verify() {
        // 2. Check balance
        if let Ok(balance) = server_handler.database.lock().await
            .calculate_balance(&tx.sender)
        {
            if balance >= (tx.amount.into_inner() + tx.fee.into_inner()) {
                // 3. Add to transaction pool
                server_handler.transaction_pool.lock().await
                    .add_transaction(tx.clone());

                // 4. Broadcast to peers
                server_handler.broadcaster.lock().await
                    .broadcast_item(BroadcastItem::Transaction(tx.clone()))
                    .await;

                return HttpResponse::Ok().body("Transaction added to pool");
            } else {
                return HttpResponse::BadRequest()
                    .body("Insufficient balance");
            }
        }
    }

    HttpResponse::BadRequest().body("Invalid transaction signature")
}
```

**Validation Steps**:
1. **Signature verification**: Ensure transaction was signed by sender
2. **Balance check**: Ensure sender has sufficient funds (amount + fee)
3. **Pool addition**: Add to transaction pool
4. **Broadcast**: Send to all peers in network

### 4. Transaction Pool

Once validated, transactions enter the **transaction pool** where they wait to be mined.

See [Transaction Pool Documentation](transaction-pool.md) for details on:
- Priority queue implementation
- Transaction ordering
- Conflict resolution
- Double-spend prevention

### 5. Mining

Miners extract transactions from the pool based on priority (highest fees first).

**Location**: `src/miner.rs:57-62`

```rust
let data = {
    self.transaction_pool
        .lock()
        .await
        .get_transactions_to_mine(self.transactions_per_block)
};
```

See [Mining Documentation](mining.md) for details on:
- Transaction extraction
- Block creation
- Proof-of-work mining

### 6. Inclusion in Block

When a block is successfully mined, transactions are:
1. Included in the block's `transactions` field
2. Removed from the transaction pool
3. Persisted to the database
4. Broadcast to all peers

**Location**: `src/miner.rs:142-146`

```rust
self.transaction_pool
    .lock()
    .await
    .process_mined_transactions(true, &new_block.transactions);
```

### 7. Confirmation

Once in a block, the transaction is considered **confirmed**. As more blocks are added on top, the transaction becomes increasingly secure.

**Finality**: In Artemis Network, transactions in blocks are considered final (no reorganization logic beyond longest-chain replacement).

## Transaction Prioritization

Transactions are prioritized in the transaction pool using a **max-heap** (binary heap).

**Location**: `src/transaction.rs:58-76`

### Custom Ordering Implementation

```rust
impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee
            .cmp(&other.fee)  // Primary: Higher fee = higher priority
            .then_with(|| other.timestamp.cmp(&self.timestamp))  // Tiebreaker: Older = higher priority
    }
}
```

### Prioritization Rules

**Priority Order**:
1. **Higher fee** → Higher priority
2. **Older timestamp** → Higher priority (if fees are equal)

**Examples**:

| Transaction | Fee | Timestamp | Priority |
|-------------|-----|-----------|----------|
| TX-A | 1.0 | 1699876543 | **1st** (highest fee) |
| TX-B | 0.5 | 1699876500 | 2nd |
| TX-C | 0.5 | 1699876600 | 3rd (same fee as B, but newer) |
| TX-D | 0.1 | 1699876400 | 4th (lowest fee) |

**Why This Order?**
- **Miners earn more** from high-fee transactions
- **Incentivizes users** to pay higher fees for faster processing
- **Fair tiebreaker**: Older transactions get priority when fees are equal
- **Prevents starvation**: Low-fee transactions eventually get mined

## Signing and Verification

### Signing Process

**Algorithm**: ECDSA (Elliptic Curve Digital Signature Algorithm)
**Curve**: secp256k1 (same as Bitcoin and Ethereum)

**Steps** (`src/transaction.rs:93-121`):

1. **Message Construction**:
   ```rust
   let message_data = format!(
       "{}:{}:{}:{}:{}",
       sender, recipient, amount, fee, timestamp
   );
   ```

2. **Hashing**:
   ```rust
   let message_hash = Sha256::digest(message_data.as_bytes());
   ```

3. **Signing**:
   ```rust
   let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &wallet.private_key);
   ```

4. **Serialization**:
   ```rust
   let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();
   let mut sig_with_recovery = sig_bytes.to_vec();
   sig_with_recovery.push(recovery_id as u8);  // 64 bytes + 1 byte
   ```

5. **Encoding**:
   ```rust
   self.signature = Some(hex::encode(sig_with_recovery));  // 130 hex chars
   ```

**Signature Format**:
- 64 bytes: ECDSA signature (r, s values)
- 1 byte: Recovery ID (0-3, typically 0 or 1)
- Total: 65 bytes → 130 hexadecimal characters

### Verification Process

**Location**: `src/transaction.rs:124-177`

```rust
pub fn verify(&self) -> bool {
    // 1. Skip verification for COINBASE
    if self.sender == "COINBASE" {
        return true;
    }

    let secp = Secp256k1::new();

    if let Some(signature_hex) = &self.signature {
        // 2. Decode signature from hex
        let sig_bytes = hex::decode(signature_hex)?;

        // 3. Extract recovery ID (last byte)
        let recovery_id_byte = sig_bytes.last().cloned().unwrap_or(0);
        let recovery_id = RecoveryId::try_from(recovery_id_byte as i32)?;

        // 4. Deserialize signature (first 64 bytes)
        let recoverable_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id)?;

        // 5. Hash message (same as signing)
        let message_data = format!(
            "{}:{}:{}:{}:{}",
            self.sender, self.recipient, self.amount, self.fee, self.timestamp
        );
        let message_hash = Sha256::digest(message_data.as_bytes());
        let message = Message::from_digest(<[u8; 32]>::from(message_hash));

        // 6. Recover public key from signature
        let recovered_key = secp.recover_ecdsa(&message, &recoverable_sig)?;

        // 7. Hash public key to get address
        let recovered_pub_key_hash = hash_public_key(&recovered_key);

        // 8. Verify address matches sender
        return recovered_pub_key_hash == self.sender;
    }

    false
}
```

**Verification Steps**:
1. Skip for COINBASE transactions
2. Decode signature from hex
3. Extract recovery ID
4. Deserialize ECDSA signature
5. Reconstruct message hash
6. **Recover public key** from signature + message
7. Hash public key to derive address
8. **Compare derived address to sender's address**

**Key Insight**: We don't store the public key in the transaction. Instead, we **recover** it from the signature and verify it produces the sender's address.

**Why This Works**:
- Wallet address = `SHA256(public_key)`
- Signature is created with private key
- Public key can be recovered from signature
- Hashing recovered public key should match sender's address

## Transaction Validation

Before accepting a transaction, multiple validation checks are performed.

### 1. Signature Validation

**Location**: `src/handler.rs:17` and `src/transaction.rs:124-177`

```rust
if tx.verify() {
    // Transaction is valid
}
```

**Checks**:
- Signature exists (except for COINBASE)
- Signature decodes properly
- Recovered public key matches sender address

### 2. Balance Validation

**Location**: `src/handler.rs:20-38`

```rust
if let Ok(balance) = server_handler
    .database
    .lock()
    .await
    .calculate_balance(&tx.sender)
{
    if balance >= (tx.amount.into_inner() + tx.fee.into_inner()) {
        // Sufficient balance
    }
}
```

**Checks**:
- Sender's balance ≥ (amount + fee)
- Balance calculated from entire blockchain history

### 3. Duplicate Prevention

**Location**: `src/server.rs:108-113`

```rust
if !self
    .transaction_pool
    .lock()
    .await
    .transaction_already_exists(&tx)
{
    // Not a duplicate
}
```

**Checks**:
- Transaction doesn't already exist in pool
- Prevents processing same transaction multiple times from different peers

### 4. Block Validation

When validating blocks, all transactions are verified:

**Location**: `src/blockchain.rs:77-80`

```rust
for tx in &block.transactions {
    if tx.sender != "COINBASE" && !tx.verify() {
        return false;
    }
}
```

## Double-Spend Prevention

**Double-spending** is when someone tries to spend the same coins twice.

### Detection Mechanisms

#### 1. Balance Checking

Before accepting a transaction, the node checks if the sender has sufficient balance.

**Location**: `src/handler.rs:20-38`

```rust
let balance = database.calculate_balance(&tx.sender)?;
if balance >= (tx.amount + tx.fee) {
    // Accept
}
```

**Balance Calculation** (`src/db.rs`): Scans entire blockchain and transaction pool to calculate available balance.

#### 2. Conflict Resolution in Pool

The transaction pool detects conflicts (multiple transactions from same sender).

**Location**: `src/pool.rs`

```rust
pub fn resolve_conflict(&mut self, new_tx: &Transaction, existing_tx: &Transaction)
```

See [Transaction Pool Documentation](transaction-pool.md) for detailed conflict resolution logic.

**Rules**:
- If sender has insufficient balance for both transactions, keep higher-fee transaction
- If new transaction has higher fee, replace existing
- If existing has higher fee, reject new

#### 3. First-Mined Wins

Once a transaction is included in a mined block:
1. It's removed from all transaction pools
2. Sender's balance is updated
3. Conflicting transactions are rejected

**Location**: `src/miner.rs:142-146`

```rust
self.transaction_pool
    .lock()
    .await
    .process_mined_transactions(true, &new_block.transactions);
```

This clears the successfully mined transactions from the pool.

### Attack Scenarios

#### Scenario 1: Send Same Transaction Twice

**Attack**: User submits identical transaction twice

**Prevention**:
- `transaction_already_exists()` check prevents duplicates
- Same hash → detected as duplicate

#### Scenario 2: Send Two Different Transactions with Same Funds

**Attack**: User creates two transactions spending the same balance

**Prevention**:
- Balance check: First transaction accepted, second rejected for insufficient balance
- Pool conflict resolution: Higher-fee transaction prioritized
- Mining: First transaction to be mined invalidates the second

#### Scenario 3: Submit to Different Nodes Simultaneously

**Attack**: Submit conflicting transactions to different nodes

**Prevention**:
- Broadcasting propagates transactions network-wide
- Nodes receive both transactions
- Conflict resolution in pool keeps higher-fee transaction
- Miners include higher-fee transaction in block
- After block is mined, losing transaction is invalidated

## Transaction Hash

Each transaction has a unique hash identifier.

**Location**: `src/transaction.rs:179-187`

```rust
pub fn hash(&self) -> String {
    let tx_data = format!(
        "{}:{}:{}:{}:{}",
        self.sender, self.recipient, self.amount, self.fee, self.timestamp
    );

    let tx_hash = Sha256::digest(tx_data.as_bytes());
    hex::encode(tx_hash)
}
```

**Uses**:
- Unique identifier for querying: `GET /transaction/{hash}`
- Used in block hash calculation
- Database indexing

**Note**: Signature is **not included** in the hash, allowing the hash to be calculated before signing.

## Summary

**Transaction Flow**:
1. Create unsigned transaction
2. Sign with sender's private key (ECDSA secp256k1)
3. Submit via HTTP or P2P
4. Validate signature and balance
5. Add to transaction pool (priority queue by fee)
6. Wait for miner to extract
7. Include in block
8. Mine block with PoW
9. Broadcast mined block
10. Remove from pool
11. Persist to database
12. Transaction confirmed

**Key Concepts**:
- **ECDSA signing**: Proves ownership without revealing private key
- **Recoverable signatures**: Allows public key recovery from signature
- **Fee-based prioritization**: Higher fees get mined faster
- **Double-spend prevention**: Balance checking + pool conflict resolution
- **COINBASE transactions**: Special transactions creating new coins
- **Signature verification**: Recovers public key and compares to sender address

This design demonstrates the core principles of cryptocurrency transactions in a simplified, educational implementation.
