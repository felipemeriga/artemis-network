# Artemis Network

An educational blockchain implementation in Rust designed to help developers understand how cryptocurrency networks work under the hood.

## About

Artemis Network is a lightweight, fully functional blockchain that demonstrates the core concepts found in production cryptocurrency networks, but with reduced complexity. This makes it an excellent learning tool for understanding:

- **Proof-of-Work Mining**: How SHA-256 hashing and difficulty targets secure the network
- **Transaction Processing**: ECDSA cryptographic signing and verification
- **Consensus Mechanisms**: How nodes agree on blockchain state using the longest-chain rule
- **P2P Networking**: Peer discovery and message broadcasting across the network
- **Distributed Systems**: Concurrent components working together with shared state
- **Data Persistence**: Embedded database with indexing strategies

⚠️ **Important**: This is an educational project, not production-ready. It prioritizes code clarity and learning value over production optimizations.

## Getting Started

### Quick Start

```bash
# Build and run a node
cargo run -- --config=./config/config-1.yaml

# Run with dev mode (fresh database on startup)
cargo run --features dev -- --config=./config/config-1.yaml
```

For detailed instructions on building, running, testing, and using the API, see **[DEVELOPMENT.md](DEVELOPMENT.md)**.

## Documentation

📚 **[MODULES.md](MODULES.md)** - Complete module overview and architecture documentation

📖 **[DEVELOPMENT.md](DEVELOPMENT.md)** - Build, run, test, and API usage guide

### Detailed Concept Guides

The `docs/` directory contains in-depth explanations of blockchain concepts:

- [Mining & Proof of Work](docs/mining.md) - How mining works with difficulty and nonce
- [Transactions](docs/transactions.md) - Transaction lifecycle, signing, and prioritization
- [Consensus & Synchronization](docs/consensus.md) - How nodes stay in sync
- [Networking & Peer Discovery](docs/networking.md) - P2P communication protocols
- [Transaction Pool](docs/transaction-pool.md) - Priority queue implementation
- [Wallet & Cryptography](docs/wallet-cryptography.md) - secp256k1 and key management
- [Database & Storage](docs/database.md) - Persistent storage with Sled

## Architecture

Artemis Network uses a **concurrent, component-based architecture** with five main components running in parallel:

- **Server**: Handles P2P (TCP) and client (HTTP) communication
- **Miner**: Executes proof-of-work block mining
- **Sync**: Synchronizes blockchain with peers
- **Broadcaster**: Propagates transactions and blocks across network
- **Discover**: Discovers and registers peers

Each component runs as an independent Tokio task, safely sharing state through Arc and Mutex wrappers. See [MODULES.md](MODULES.md) for complete architectural details.

## Contributing

Contributions are welcome! This is an educational project, so clarity and learning value are prioritized over production optimizations.