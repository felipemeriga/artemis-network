# Development Guide

This guide covers how to build, run, test, and develop Artemis Network.

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Docker** (optional): For running multi-node network

## Building the Project

```bash
# Standard build
cargo build

# Release build (optimized)
cargo build --release
```

## Running Nodes

### Single Node

```bash
# Run with configuration file
cargo run -- --config=./config/config-1.yaml

# Run with dev feature (recreates database on startup)
cargo run --features dev -- --config=./config/config-1.yaml
```

### Multiple Nodes (Docker)

Run a 3-node network using Docker Compose:

```bash
cd docker
docker-compose up --build
```

**Node Endpoints**:
- Node 1: http://localhost:8080
- Node 2: http://localhost:8081
- Node 3: http://localhost:8082

### Multiple Nodes (Manual)

Run nodes in separate terminals:

```bash
# Terminal 1 - Genesis Node
cargo run -- --config=./config/config-1.yaml

# Terminal 2 - Node 2
cargo run -- --config=./config/config-2.yaml

# Terminal 3 - Node 3
cargo run -- --config=./config/config-3.yaml
```

## Configuration

Each node requires a YAML configuration file. See `config/` directory for examples.

### Configuration Fields

```yaml
tcpAddress: "127.0.0.1:5000"       # P2P communication port
httpAddress: "0.0.0.0:8080"        # HTTP API port
bootstrapAddress: null             # Bootstrap node address (null for genesis)
nodeId: "master"                   # Unique node identifier
minerWalletAddress: "address..."   # Wallet address for mining rewards
```

**Genesis Node**: Set `bootstrapAddress: null`

**Regular Nodes**: Set `bootstrapAddress` to the genesis node's TCP address (e.g., `"127.0.0.1:5000"`)

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Linting

```bash
# Run Clippy
cargo clippy

# Clippy with warnings as errors (same as CI)
cargo clippy -- -D warnings --verbose
```

## Formatting

```bash
# Check formatting
cargo fmt -- --check

# Format code
cargo fmt
```

## Using Cargo Make

The project includes a `Makefile.toml` for common tasks:

```bash
# Format code
cargo make format

# Clean build artifacts
cargo make clean

# Clean + build
cargo make build

# Run default node
cargo make run

# Clean + test
cargo make test

# Lint with Clippy
cargo make lint
```

## Development Mode

The `dev` feature flag enables development mode:

```bash
cargo run --features dev -- --config=./config/config-1.yaml
```

**What it does**:
- Deletes and recreates database on startup
- Provides a clean slate for testing
- Defined in `src/db.rs` with `#[cfg(feature = "dev")]`

## Database Location

Databases are stored per-node in the `database/` directory:

```
database/
├── blockchain-db-master/    (Node 1)
├── blockchain-db-node2/     (Node 2)
└── blockchain-db-node3/     (Node 3)
```

To reset a node's database, delete its directory:

```bash
rm -rf database/blockchain-db-master/
```

## HTTP API Usage

### Create Wallet

```bash
curl -X POST http://localhost:8080/create-wallet
```

**Response**:
```json
{
  "privateKey": "...",
  "publicKey": "...",
  "address": "..."
}
```

### Submit Transaction

```bash
curl -X POST http://localhost:8080/transaction/submit \
  -H "Content-Type: application/json" \
  -d '{
    "sender": "sender_address",
    "recipient": "recipient_address",
    "amount": 10.0,
    "fee": 0.1,
    "timestamp": 1699876543,
    "signature": "signature_hex"
  }'
```

### Sign and Submit (Dev Only)

⚠️ **Warning**: Sends private key over network. For development/learning only!

```bash
curl -X POST http://localhost:8080/transaction/sign-and-submit \
  -H "Content-Type: application/json" \
  -d '{
    "transaction": {
      "sender": "sender_address",
      "recipient": "recipient_address",
      "amount": 10.0,
      "fee": 0.1,
      "timestamp": 1699876543
    },
    "publicKeyHex": "...",
    "privateKeyHex": "..."
  }'
```

### Get Wallet Balance

```bash
curl http://localhost:8080/wallet/{address}/balance
```

### Get All Blocks

```bash
curl http://localhost:8080/blocks
```

### Get Block by Hash

```bash
curl http://localhost:8080/block/{hash}
```

### Get Transaction by Hash

```bash
curl http://localhost:8080/transaction/{hash}
```

### Get Wallet Transactions

```bash
curl http://localhost:8080/wallet/{address}/transactions
```

### Health Check

```bash
curl http://localhost:8080/health
```

## Logging

Logs are written to stdout with structured prefixes per component:

```
[2025-11-05 10:30:15] [SERVER] TCP Server listening on 127.0.0.1:5000
[2025-11-05 10:30:16] [DISCOVER] New peer discovered on address: 127.0.0.1:5001
[2025-11-05 10:30:20] [MINER] Starting mining with difficulty: 5
[2025-11-05 10:30:45] [MINER] Mining complete! Block added to blockchain
[2025-11-05 10:30:46] [BROADCASTER] Broadcasting new block to peers
```

**Log Levels**: Set via `RUST_LOG` environment variable:

```bash
# All logs
RUST_LOG=debug cargo run -- --config=./config/config-1.yaml

# Info only
RUST_LOG=info cargo run -- --config=./config/config-1.yaml

# Specific module
RUST_LOG=artemis_network::miner=debug cargo run -- --config=./config/config-1.yaml
```

## Troubleshooting

### Port Already in Use

If you see "Address already in use":

```bash
# Find process using port
lsof -i :5000
lsof -i :8080

# Kill process
kill -9 <PID>
```

Or change ports in your config file.

### Database Errors

If you encounter database corruption:

```bash
# Delete database and restart
rm -rf database/blockchain-db-{node_id}/
cargo run --features dev -- --config=./config/config-1.yaml
```

### Peer Connection Issues

- Ensure bootstrap node is running first
- Check `bootstrapAddress` in config matches bootstrap node's `tcpAddress`
- Verify firewall isn't blocking ports

### Mining Not Starting

Mining requires:
1. Peer discovery to complete (3 seconds + first discovery)
2. Blockchain sync to complete (first 120-second cycle)

Check logs for:
```
[DISCOVER] New peer discovered...
[SYNC] Local chain is the longest.
[MINER] Starting mining with difficulty: 5
```

## CI/CD

GitHub Actions runs on all branches:

**Lint Job**:
```bash
cargo clippy -- -D warnings --verbose
```

**Build and Test Job**:
```bash
cargo build
cargo test
```

## Project Structure

```
artemis-network/
├── config/              # Node configuration files
├── docker/              # Docker setup
├── docs/                # Detailed documentation
├── src/                 # Source code
│   ├── main.rs         # Entry point
│   ├── node.rs         # Component orchestrator
│   ├── blockchain.rs   # Blockchain logic
│   ├── block.rs        # Block structure
│   ├── transaction.rs  # Transaction logic
│   ├── pool.rs         # Transaction pool
│   ├── miner.rs        # Mining process
│   ├── server.rs       # TCP/HTTP servers
│   ├── handler.rs      # HTTP handlers
│   ├── sync.rs         # Blockchain sync
│   ├── discover.rs     # Peer discovery
│   ├── broadcaster.rs  # P2P broadcasting
│   ├── wallet.rs       # Wallet management
│   ├── db.rs           # Database operations
│   └── ...
├── Cargo.toml          # Dependencies
├── Makefile.toml       # Cargo make tasks
└── README.md           # Project overview
```

## Performance Notes

### Mining Difficulty

Default difficulty is 5 (five leading zeros). This requires ~1 million hash attempts on average.

To change difficulty, modify `src/blockchain.rs`:

```rust
Blockchain {
    chain: vec![genesis_block],
    difficulty: 5, // Change this value
    total_supply: 0,
}
```

**Warning**: Higher difficulty = slower mining. Lower difficulty = faster blocks but less realistic.

### Transaction Pool Size

Maximum transactions per block: 10 (configurable in node initialization)

### Sync Interval

Blockchain sync occurs every 120 seconds. To change, modify `src/sync.rs`:

```rust
tokio::time::sleep(Duration::from_secs(120)).await; // Change interval
```

### Peer Discovery Interval

Peer discovery runs every 60 seconds. To change, modify `src/discover.rs`:

```rust
tokio::time::sleep(Duration::from_secs(60)).await; // Change interval
```

## Development Tips

### Hot Reload

For faster development, use `cargo-watch`:

```bash
cargo install cargo-watch
cargo watch -x 'run -- --config=./config/config-1.yaml'
```

### Better Logging

For colored logs, use `env_logger`:

```bash
RUST_LOG=info cargo run -- --config=./config/config-1.yaml
```

### Debugging

Use `dbg!()` macro or attach debugger:

```bash
# GDB (Linux)
rust-gdb target/debug/artemis-network

# LLDB (macOS)
rust-lldb target/debug/artemis-network
```

### Clean Slate Testing

Always use dev mode for testing:

```bash
cargo run --features dev -- --config=./config/config-1.yaml
```

This ensures a fresh database on each run.

## Contributing

When contributing:

1. Run formatter: `cargo fmt`
2. Run linter: `cargo clippy -- -D warnings`
3. Run tests: `cargo test`
4. Test manually with dev mode
5. Update documentation if needed

## Additional Resources

- [MODULES.md](MODULES.md) - Complete architecture overview
- [docs/](docs/) - Detailed concept documentation
- [Rust Book](https://doc.rust-lang.org/book/) - Learn Rust
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Async Rust
