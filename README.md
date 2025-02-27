# artemis-network

A simple and effective way to learn how blockchain works under the hood.

---

## Motivation

While working with various companies in the Web3 space, we’ve noticed a common trend among developers: many have a solid understanding of how the blockchain networks they work with are supposed to function. However, some primarily interact with blockchain networks through APIs and interfaces without fully grasping the underlying mechanisms.

Although you don’t need to understand every technical aspect of blockchain to work with it, gaining deeper insights can give you a significant edge over others.

## Introduction

Artemis-network is a blockchain implementation written entirely in Rust. It aims to provide a lightweight yet fully functional codebase, showcasing essential blockchain features found in production-ready networks, but with reduced complexity. This makes it an excellent learning tool for understanding how cryptocurrency networks operate beneath the surface.

You can run our nodes locally or across different servers to explore topics such as:
- The core components of a blockchain.
- How concurrent and parallel processes interact within a blockchain network.
- Networking details, including peer-to-peer (P2P) communication.
- The role of miners, transaction validation, and block creation.
- Various consensus algorithms used in blockchain networks, along with their advantages and trade-offs.
- Hands-on experience in building your own blockchain network from scratch.
- Fundamentals of blockchain technology and its impact on different industries.
- The transaction validation and processing lifecycle.

While Artemis-network is not a production-ready blockchain, it provides a deep, hands-on approach to learning these concepts.

---

## Current Features

We are actively developing this project and have structured the blockchain’s internal responsibilities within a node using different functional components:

- **Server**:
  - Manages TCP and HTTP connections.
- **Sync**:
  - Keeps the blockchain state synchronized across nodes.
- **Miner**:
  - Executes the mining process.
- **Broadcaster**:
  - Broadcasts transactions, blocks, and the blockchain itself to peers.
- **Discover**:
  - Responsible for discovering another peers

These components run concurrently, safely sharing references to the transaction pool, blockchain, and peer data.

### Key Features Implemented:
- Lightweight and easy-to-read codebase.
- A guide for understanding blockchain networks.
- Basic block setup with proof-of-work mining functions.
- Blockchain structure, including blocks and validation mechanisms.
- Wallet generation with public/private key cryptography and address management.
- **Transactions**:
  - Creation, signing, and validation.
- **Transaction Pool**:
  - Prioritization based on gas fees.
  - Handling of transaction conflicts and resolution.
- **Networking**:
  - TCP server for P2P communication.
  - Broadcasting transactions, blocks, and the blockchain state.
  - HTTP server for client interactions and future RPC support.
- **Sync Mechanism**:
  - Nodes periodically request full blockchain copies to compare and update their state.
- **Mining Process**:
  - Miners extract transactions from the transaction pool and attempt to mine new blocks.

---

## Upcoming Features

- Implement dynamic peer discovery using mDNS (instead of manual command-line configurations).
- Strengthen validation rules to prevent double-spending.
- Develop a CLI tool for wallet creation and transaction signing.
- Improve consensus handling to manage forks efficiently.
- **Blockchain Explorer**: Build a web-based interface to visualize blockchain data.
- **Scalability Enhancements**: Improve performance and scalability.
- **Testing & Optimization**: Write tests, benchmark, and optimize the code.

---

## Command-Line Arguments

In order to run a node, you need to provide the following arguments:

- `config` → The path to the config file, containing the node basic config.

---

## Config file

The config file is responsible for setting the node configuration, it needs to be a yaml file, containing the
following fields:

- `tcpAddress`: The hostname and port for the TCP server (e.g., `127.0.0.1:5000`).
- `httpAddress`: The hostname and port for the HTTP server (e.g., `127.0.0.1:8080`).
- `bootstrapAddress`: The hostname and the port of the bootstrap node, which represents another node that already exists 
in the network, which will be the first point of contact of this current node, for discovering another nodes. If launching 
genesis node, you don't need to provide this attribute.
- `nodeId`: A unique ID for your node.

---

## Running the Project Locally

To run Artemis-network locally, use the following command:

```shell
cargo run --color=always --package artemis-network --features dev --bin artemis-network -- --config=./config/node-1.yaml
```