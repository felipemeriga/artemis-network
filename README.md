# artemis-network

A straightforward way of learning how blockchain works under the hood.

---

## Motivations

Working through many different companies placed over web3 market, we can see among its developers,
that most of them usually have a proper knowledge of how the network they work with is supposed to work, although
we still see people that only interact with the blockchain network through
APIs and interfaces without understanding the underlying mechanisms.

Ideally, you don't need to know all the aspects behind a blockchain, but getting in touch with them
may put you over another superficial candidate.

## Introduction

Artemis-network is a blockchain fully written in Rust, which aims to be a lightweight codebase, 
containing all the necessary features that a production-ready blockchain has, but with less complexity,
for being a guide of how most of the cryptocurrency network works bellow the scenes.

Therefore, you can run our Nodes locally, or even through different servers, for:
- What are the main parts of a blockchain.
- How does the different concurrent/parallel process interact with each other.
- Networking details about p2p communication.
- Study the role of miners and how they validate transactions and create new blocks.
- Learn about different consensus algorithms used in blockchain networks and their pros and cons.
- Gain hands-on experience by building your own blockchain network from scratch.
- Explore the fundamentals of blockchain technology and its impact on various industries.
- How transactions are validated and processed.

Thus, while artemis isn't a production-ready blockchain, it's a deeper way of learning this concept.

---

## Current Features

We are still building this tool,
and we divided each blockchain's internal responsibilities inside a node, through different characters:
- Server:
  - Responsible for managing TCP and HTTP connections.
  - Works on discovering peers.
- Sync:
  - Keeps the blockchain state in sync between nodes.
- Miner:
  - Executes the real mining process.
- Broadcaster:
  - Broadcast transactions, blocks and the own blockchain to peers.

All these actors, run concurrently for achieving their tasks.
Sharing references safely about transaction pool,
blockchain, peers.

Here we have more in detail, what we currently have:

- Lightweight codebase.
- Guide for understanding blockchain networks.
- Basic block setup, and proof of work functions for mining it.
- Blockchain structure, containing blocks, and validations.
- Wallet generation, public/private key, and address.
- Transaction:
  - Creating, signing and validating.
- Transaction pool:
  - Prioritization based on gas.
  - Capability of handling transaction conflicts and resolving them.
- Networking:
  - TCP server for p2p communication.
  - Ability to broadcast transactions, blocks and blockchain.
  - HTTP server for client interactions, and future RPC.
- Sync:
  - Nodes periodically request the full copy of the blockchain to compare with the current one.
- Mining:
  - Current miners pop some transactions from the transaction pool and try to mine the block.

---

## Next Features to Implement

- Apply linting on the current code.
- Currently, peers are managed through command-line arguments.
  We need to add a mDNS mechanism, for dynamically
  discovering peers.
- Add RocksDB for managing and storing transactions and blocks.
- Validation rules, preventing double-spending.
- Configuration management of each node through a config file.
- Utils repo, for creating a wallet, signing transactions through a CLI tool.
- Handle forks on consensus algorithm
- Blockchain Explorer: Build a web interface to explore blockchain data.
- Scalability: Focus on improving performance and scalability.
- Testing and Optimization: Write tests, benchmark, and optimize the code.


## Current Command line arguments:
- `tcp-bind`: The hostname and port to run the TCP server (e.g., `127.0.0.1:5000`)
- `rpc-bind`: The hostname and port to run the HTTP server (e.g., `127.0.0.1:8080`)
- `peers`: List of peer nodes (comma-separated, e.g., `127.0.0.1:8333,192.168.1.1:8333`)

## How to run it locally

```shell
cargo run --color=always --package artemis-network --features qa --bin artemis-network -- --tcp-bind=127.0.0.1:5000 --rpc-bind=127.0.0.1:8080 --peers=127.0.0.1:5001
```
