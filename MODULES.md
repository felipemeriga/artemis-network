# Artemis Network - Module Documentation

This document provides an overview of the Artemis Network blockchain implementation and links to detailed documentation for each component and concept.

## About Artemis Network

Artemis Network is an **educational blockchain implementation** written in Rust, designed to help developers understand how cryptocurrency networks operate. This is **not production-ready** and serves as a learning tool with reduced complexity compared to real-world blockchains.

## Architecture Overview

The system uses a **concurrent, component-based architecture** where different responsibilities run in parallel using Tokio's async runtime. Five main components coordinate through shared state and message passing:

1. **Server** - Handles P2P (TCP) and client (HTTP) communication
2. **Miner** - Executes proof-of-work block mining
3. **Sync** - Synchronizes blockchain with peers
4. **Broadcaster** - Broadcasts transactions and blocks to network
5. **Discover** - Discovers and registers peers

## Core Modules

### Entry Points
- **`main.rs`** - Application entry point; parses CLI args, loads config, initializes logger
- **`node.rs`** - Central orchestrator; spawns all concurrent components with shared state

### Data Structures
- **`blockchain.rs`** - Blockchain state, validation, and chain replacement logic
- **`block.rs`** - Block structure and proof-of-work mining
- **`transaction.rs`** - Transaction structure, signing, and verification
- **`pool.rs`** - Transaction pool with priority queue ([detailed docs](docs/transaction-pool.md))
- **`wallet.rs`** - Cryptographic wallet functionality ([detailed docs](docs/wallet-cryptography.md))
- **`db.rs`** - Persistent storage using Sled ([detailed docs](docs/database.md))

### Networking Components
- **`server.rs`** - TCP (P2P) and HTTP (RPC) servers
- **`handler.rs`** - HTTP endpoint handlers for RPC API
- **`broadcaster.rs`** - Generic broadcast utility for P2P messages
- **`discover.rs`** - Peer discovery protocol
- **`sync.rs`** - Blockchain synchronization

### Mining
- **`miner.rs`** - Block mining with proof-of-work ([detailed docs](docs/mining.md))

### Supporting Modules
- **`config.rs`** - Configuration loading from YAML
- **`constants.rs`** - Global constants
- **`error.rs`** - Custom error types using `thiserror`
- **`logger.rs`** - Structured logging macros per component
- **`utils.rs`** - Shared utility functions
- **`tests.rs`** - Unit tests

## Detailed Documentation

### Core Concepts

📖 **[Mining & Proof of Work](docs/mining.md)**
- How proof-of-work mining works
- Mining difficulty and target hash
- Mining interruption mechanism
- Miner rewards and coinbase transactions

📖 **[Transactions](docs/transactions.md)**
- Transaction structure and lifecycle
- ECDSA signing and verification
- Transaction prioritization by fees
- Special transaction types (COINBASE)

📖 **[Consensus & Synchronization](docs/consensus.md)**
- Longest-chain consensus rule
- Blockchain validation
- Peer synchronization process
- Chain replacement logic

📖 **[Networking & Peer Discovery](docs/networking.md)**
- P2P communication protocol (TCP)
- Peer discovery mechanism
- Message types and routing
- Broadcasting to network

### Advanced Topics

📖 **[Transaction Pool](docs/transaction-pool.md)**
- Priority queue implementation (BinaryHeap)
- Lazy deletion pattern
- Conflict resolution and double-spend prevention
- Pending transaction management

📖 **[Wallet & Cryptography](docs/wallet-cryptography.md)**
- secp256k1 key pair generation
- Address derivation from public keys
- ECDSA signature creation and verification
- Key import/export

📖 **[Database & Storage](docs/database.md)**
- Sled embedded database schema
- Block and transaction storage
- Address-based indexing
- Balance calculation from transaction history

## Module Dependencies

```
main.rs
  └─> node.rs (orchestrator)
       ├─> server.rs ──> handler.rs
       ├─> miner.rs ──────────────────┐
       ├─> sync.rs                    │
       ├─> discover.rs                │
       └─> broadcaster.rs             │
                                      │
       All components share:          │
       ├─> blockchain.rs <────────────┘
       ├─> block.rs
       ├─> transaction.rs
       ├─> pool.rs
       ├─> wallet.rs
       ├─> db.rs
       ├─> config.rs
       ├─> constants.rs
       ├─> error.rs
       ├─> logger.rs
       └─> utils.rs
```

## Concurrency Model

All major components run as independent Tokio tasks:
- **Shared State**: `Arc<RwLock<Blockchain>>` for blockchain, `Arc<Mutex<>>` for pools/peers
- **Message Passing**: `mpsc` channels for block notifications (Server → Miner)
- **Coordination Flags**: `first_discover_done`, `first_sync_done` ensure proper startup order

## Quick Reference

### Key Data Structures

| Structure | Purpose | Location |
|-----------|---------|----------|
| `Blockchain` | Chain state and validation | `blockchain.rs` |
| `Block` | Individual block with PoW | `block.rs` |
| `Transaction` | Signed value transfer | `transaction.rs` |
| `TransactionPool` | Priority queue for pending transactions | `pool.rs` |
| `Wallet` | Key management and signing | `wallet.rs` |
| `Node` | Component orchestrator | `node.rs` |

### Key Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `DIFFICULTY` | 5 | Number of leading zeros in block hash |
| `MAX_SUPPLY` | 21,000,000 | Total coin supply limit |
| Sync Interval | 120s | Blockchain synchronization period |
| Discovery Interval | 60s | Peer discovery period |
| Max Transactions/Block | 10 | Maximum transactions per block |

### HTTP API Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/transaction/submit` | Submit signed transaction |
| POST | `/transaction/sign-and-submit` | Sign and submit (dev only) |
| GET | `/transaction/{hash}` | Get transaction by hash |
| GET | `/wallet/{address}/balance` | Get wallet balance |
| GET | `/wallet/{address}/transactions` | Get wallet transactions |
| GET | `/block/{hash}` | Get block by hash |
| GET | `/blocks` | Get all blocks |
| POST | `/create-wallet` | Create new wallet |
| GET | `/health` | Health check |

