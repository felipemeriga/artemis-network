# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Artemis Network is an educational blockchain implementation written in Rust, designed to help developers understand how cryptocurrency networks operate. This is **not production-ready** and serves as a learning tool with reduced complexity compared to real-world blockchains.

## High-Level Architecture

The system uses a **concurrent, component-based architecture** where different responsibilities run in parallel using Tokio's async runtime. All components share state through thread-safe wrappers and coordinate via channels.

### Core Components (Running Concurrently)

1. **Server** (`src/server.rs`)
   - TCP Server: P2P communication with other nodes
   - HTTP Server: REST API using Actix-Web for client interactions
   - Routes: TRANSACTION, NEW_BLOCK, GET_BLOCKCHAIN, REGISTER

2. **Miner** (`src/miner.rs`)
   - Executes proof-of-work mining with difficulty checking
   - Extracts transactions from pool based on priority (fees)
   - Handles mining interruption when new blocks arrive
   - Creates coinbase transactions for rewards

3. **Sync** (`src/sync.rs`)
   - Periodically requests full blockchain from peers (every 120s)
   - Implements longest-chain consensus rule
   - Validates and replaces local chain if longer valid chain found

4. **Broadcaster** (`src/broadcaster.rs`)
   - Broadcasts transactions and blocks to all known peers
   - Generic implementation supporting multiple data types
   - Handles peer connection failures and cleanup

5. **Discover** (`src/discover.rs`)
   - Registers with bootstrap node
   - Periodically refreshes peer list (every 60s)
   - Implements basic peer discovery protocol

### Concurrency & State Management

**Critical Pattern**: The Node (`src/node.rs`) orchestrates all components using:
- `Arc<RwLock<Blockchain>>`: Shared blockchain state (multiple readers, single writer)
- `Arc<Mutex<TransactionPool>>`, `Arc<Mutex<Peers>>`, etc.: Protected mutable state
- `mpsc channels`: Block notifications between components (e.g., Server → Miner for interruption)

**Coordination Flags**:
- `first_discover_done`: Ensures peer discovery completes before sync
- `first_sync_done`: Ensures sync completes before mining starts

### Key Data Structures

1. **TransactionPool** (`src/pool.rs`)
   - Binary max-heap for priority-based ordering (higher fees first)
   - HashMap for O(1) transaction lookup
   - "Removed set" for lazy deletion (avoids heap traversal)
   - Pending map for transactions currently being mined
   - Sophisticated conflict resolution

2. **Blockchain** (`src/blockchain.rs`)
   - Vector of blocks with difficulty setting (default: 5 leading zeros)
   - Total supply tracking (MAX_SUPPLY: 21,000,000)
   - Chain validation and replacement logic

3. **Transaction** (`src/transaction.rs`)
   - ECDSA signature verification using secp256k1
   - OrderedFloat for deterministic ordering
   - Priority based on fees and timestamp

4. **Database** (`src/db.rs`)
   - Sled embedded database (per-node instances)
   - Indexes transactions by sender/recipient addresses
   - Wallet balance calculation

### Startup Sequence

1. Load configuration from YAML
2. Initialize blockchain with genesis block
3. Store genesis block in database
4. Create channel for block notifications
5. Initialize all components with shared state
6. Add bootstrap peer if configured
7. Launch concurrent tasks: TCP Server, HTTP Server, Discover, Sync, Miner

## Development Commands

### Using Cargo

```bash
# Build
cargo build
cargo build --release

# Run single node
cargo run -- --config=./config/config-1.yaml

# Run with dev feature (recreates DB on startup)
cargo run --features dev -- --config=./config/config-1.yaml

# Run tests
cargo test

# Lint (CI uses -D warnings)
cargo clippy -- -D warnings --verbose

# Format
cargo fmt -- --emit=files
```

### Using Cargo Make (Makefile.toml)

```bash
cargo make format    # Format code
cargo make clean     # Clean build artifacts
cargo make build     # Clean + build
cargo make run       # Run default node
cargo make test      # Clean + test
cargo make lint      # Clippy
```

### Using Docker (3-Node Network)

```bash
cd docker
docker-compose up --build

# Node ports:
# - Node 1: localhost:8080
# - Node 2: localhost:8081
# - Node 3: localhost:8082
```

## Configuration Structure

Each node requires a YAML config file (see `config/config-*.yaml`):

```yaml
tcpAddress: "127.0.0.1:5000"       # P2P communication port
httpAddress: "0.0.0.0:8080"        # HTTP API port
bootstrapAddress: null             # Bootstrap node address (null for genesis)
nodeId: "master"                   # Unique node identifier
minerWalletAddress: "address..."   # Wallet address for mining rewards
```

## HTTP API Endpoints

- `POST /transaction/submit` - Submit signed transaction
- `POST /transaction/sign-and-submit` - Sign and submit (dev only, not secure)
- `POST /transaction/sign` - Sign transaction
- `GET /transaction/{hash}` - Get transaction by hash
- `GET /wallet/{address}/transactions` - Get wallet transactions
- `GET /wallet/{address}/balance` - Get wallet balance
- `GET /block/{hash}` - Get block by hash
- `GET /blocks` - Get all blocks
- `POST /create-wallet` - Create new wallet
- `GET /health` - Health check

## Important Patterns & Conventions

### Structured Logging

Custom macros per component with timestamps and prefixes:
- `server_info!()`, `server_error!()`
- `miner_info!()`, `miner_error!()`
- `sync_info!()`, `sync_error!()`
- `discover_info!()`, `discover_error!()`
- `broadcaster_info!()`, `broadcaster_error!()`

### Error Handling

Uses `thiserror` for custom error types with proper propagation. See `src/error.rs` for WalletError, DatabaseError, etc.

### Feature Flags

- `dev` feature: Recreates database on startup (for development/testing)

### Transaction Priority

Transactions are ordered by:
1. Fee amount (higher = higher priority)
2. Timestamp (older = higher priority as tiebreaker)

### Security Notes

- **COINBASE transactions** are validated to prevent abuse (only miner can create)
- Private keys should NOT be transmitted over network
- `/transaction/sign-and-submit` endpoint is for development/learning only
- Signature verification uses secp256k1 curve
- Balance checking before transaction acceptance

## Key Dependencies

- **tokio**: Async runtime
- **actix-web**: HTTP server framework
- **secp256k1**: Elliptic curve cryptography
- **sha2**: SHA-256 hashing
- **serde/serde_json/serde_yaml**: Serialization
- **sled**: Embedded database
- **clap**: CLI argument parsing

## CI/CD

GitHub Actions runs on all branches:
- Lint: `cargo clippy` with warnings as errors
- Build and test: `cargo build && cargo test`
