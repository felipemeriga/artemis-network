# Networking & Peer Discovery

This document explains how Artemis Network nodes communicate and discover each other in a peer-to-peer network.

## Table of Contents
- [Network Architecture](#network-architecture)
- [Communication Protocols](#communication-protocols)
- [Peer Discovery](#peer-discovery)
- [Broadcasting](#broadcasting)
- [Message Types](#message-types)
- [Server Components](#server-components)
- [Error Handling](#error-handling)

## Network Architecture

Artemis Network uses a **hybrid peer-to-peer architecture** with an optional bootstrap node for initial discovery.

### Network Topology

```
┌─────────────────┐
│ Bootstrap Node  │ (Genesis node, optional)
│   (Node 1)      │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
┌───▼───┐ ┌──▼────┐
│Node 2 │ │Node 3 │
└───┬───┘ └──┬────┘
    │         │
    └────┬────┘
         │
    ┌────▼────┐
    │ Node 4  │
    └─────────┘
```

**Bootstrap Node**:
- Acts as initial contact point for new nodes
- Maintains list of all registered peers
- Shares peer list with joining nodes
- Not required (nodes can connect directly if addresses known)

**Regular Nodes**:
- Connect to bootstrap node on startup
- Receive peer list from bootstrap
- Establish P2P connections with other peers
- Broadcast transactions and blocks to all peers

### Two-Server Architecture

Each node runs **two servers** simultaneously:

1. **TCP Server** (P2P Communication):
   - Binary protocol (JSON over TCP)
   - Handles peer-to-peer messages
   - Port configured in `tcpAddress` (e.g., 5000-5002)

2. **HTTP Server** (RPC API):
   - REST API for clients
   - JSON responses
   - Port configured in `httpAddress` (e.g., 8080-8082)

## Communication Protocols

### P2P Protocol (TCP)

**Protocol**: JSON over TCP
**Encoding**: UTF-8 text

**Message Format**:
```json
{
  "command": "command_name",
  "data": "serialized_payload"
}
```

**Location**: `src/server.rs:26-29`

```rust
pub struct Request {
    command: String,
    data: String,
}
```

**Commands**:
- `transaction`: Broadcast new transaction
- `new_block`: Broadcast newly mined block
- `get_blockchain`: Request full blockchain
- `register`: Register as peer

### HTTP Protocol (RPC)

**Protocol**: HTTP/1.1 REST API
**Format**: JSON request/response

**Example Request**:
```http
POST /transaction/submit HTTP/1.1
Content-Type: application/json

{
  "sender": "abc123...",
  "recipient": "def456...",
  "amount": 10.0,
  "fee": 0.1,
  "timestamp": 1699876543,
  "signature": "3045..."
}
```

**Example Response**:
```http
HTTP/1.1 200 OK
Content-Type: text/plain

Transaction added to pool
```

See [MODULES.md](../MODULES.md) for full API endpoint list.

## Peer Discovery

The `Discover` component implements peer discovery protocol.

**Location**: `src/discover.rs:16-23`

```rust
pub struct Discover {
    peers: Arc<Mutex<HashSet<String>>>,
}
```

### Discovery Process

**Location**: `src/discover.rs:25-121`

```rust
pub async fn find_peers(&mut self, node_id: String, tcp_address: String, ...) {
    // Initial 3-second delay
    tokio::time::sleep(Duration::from_secs(3)).await;

    loop {
        let peers = { self.peers.lock().await.clone() };

        for peer_address in peers {
            if peer_address == tcp_address {
                continue; // Skip self
            }

            if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
                // 1. Create peer registration message
                let this_peer = Peer {
                    id: node_id.clone(),
                    address: tcp_address.clone(),
                };

                // 2. Send REGISTER request
                let request = Request {
                    command: "register".to_string(),
                    data: serde_json::to_string(&this_peer)?,
                };

                stream.write_all(marshalled_request.as_bytes()).await?;

                // 3. Receive peer list from bootstrap
                let mut buffer = [0; 1024];
                if let Ok(n) = stream.read(&mut buffer).await {
                    let data = String::from_utf8_lossy(&buffer[..n]);
                    if let Ok(remote_peers) = serde_json::from_str::<HashSet<String>>(&data) {
                        // 4. Add discovered peers to local list
                        for address in remote_peers {
                            if address != tcp_address {
                                self.peers.lock().await.insert(address);
                            }
                        }
                    }
                }

                break; // Exit after successful registration
            } else {
                // Remove dead peer
                self.peers.lock().await.remove(&peer_address);
            }
        }

        // Mark first discovery as done
        if !*first_discover_done.lock().await {
            *first_discover_done.lock().await = true;
        }

        // Periodic refresh every 60 seconds
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

### Discovery Steps

**1. Initial Delay** (`discover.rs:32`):
```rust
tokio::time::sleep(Duration::from_secs(3)).await;
```

Waits 3 seconds on startup to ensure servers are running.

**2. Connect to Bootstrap** (`discover.rs:42`):
```rust
if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
    // ...
}
```

Attempts TCP connection to each known peer (typically bootstrap node).

**3. Register Self** (`discover.rs:43-76`):
```rust
let this_peer = Peer {
    id: node_id.clone(),
    address: tcp_address.clone(),
};

let request = Request {
    command: "register".to_string(),
    data: serde_json::to_string(&this_peer)?,
};

stream.write_all(marshalled_request.as_bytes()).await?;
```

Sends `REGISTER` command with node's ID and address.

**4. Receive Peer List** (`discover.rs:78-98`):
```rust
if let Ok(remote_peers) = serde_json::from_str::<HashSet<String>>(&data) {
    for address in remote_peers {
        if address != tcp_address {
            self.peers.lock().await.insert(address);
        }
    }
}
```

Receives `HashSet<String>` of peer addresses from bootstrap node.

**5. Update Local Peers** (`discover.rs:86-94`):
```rust
let mut peers = self.peers.lock().await;
if !peers.contains(&address.clone()) {
    discover_info!("New peer discovered on address: {}", address);
    peers.insert(address);
}
```

Adds new peers to local peer list (avoiding duplicates).

**6. Set Discovery Flag** (`discover.rs:114-116`):
```rust
if !*first_discover_done.lock().await {
    *first_discover_done.lock().await = true;
}
```

Signals that initial discovery is complete, allowing other components (Sync, Miner) to start.

**7. Periodic Refresh** (`discover.rs:118`):
```rust
tokio::time::sleep(Duration::from_secs(60)).await;
```

Repeats discovery every 60 seconds to find new peers.

### Bootstrap Node Registration Handling

When bootstrap node receives `REGISTER` command:

**Location**: `src/server.rs:163-175`

```rust
REGISTER => {
    if let Ok(peer) = serde_json::from_str::<Peer>(&req.data) {
        server_info!("Peer registered: {} at {}", peer.id, peer.address);

        // Add peer to list
        self.peers.lock().await.insert(peer.address.clone());

        // Send back full peer list
        let peers = { self.peers.lock().await.clone() };
        let peers_json = serde_json::to_string(&peers)?;

        stream.write_all(peers_json.as_bytes()).await?;
    }
}
```

**Response**: Bootstrap sends back `HashSet<String>` of all known peers.

### Peer Structure

**Location**: `src/discover.rs:10-14`

```rust
pub struct Peer {
    pub id: String,       // Node identifier (from config)
    pub address: String,  // TCP address (e.g., "127.0.0.1:5000")
}
```

## Broadcasting

The `Broadcaster` component propagates transactions and blocks across the network.

**Location**: `src/broadcaster.rs:11-14`

```rust
pub struct Broadcaster {
    peers: Arc<Mutex<HashSet<String>>>,
    tcp_address: String,
}
```

### Broadcast Types

**Location**: `src/broadcaster.rs:16-22`

```rust
pub enum BroadcastItem<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    NewBlock(T),
    Transaction(T),
}
```

**Generic Implementation**: Can broadcast any serializable type.

### Broadcast Process

**Location**: `src/broadcaster.rs:29-76`

```rust
pub async fn broadcast_item<T>(&self, payload: BroadcastItem<T>)
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    // 1. Determine command type
    let (data, command, header) = match payload {
        BroadcastItem::NewBlock(block) => (block, NEW_BLOCK.to_string(), "block"),
        BroadcastItem::Transaction(tx) => (tx, TRANSACTION.to_string(), "transaction"),
    };

    broadcaster_info!("broadcasting new {} to peers", header);

    // 2. Get peer list
    let peers_list = { self.peers.lock().await.clone() };

    // 3. Send to each peer
    for peer_address in peers_list {
        if peer_address == self.tcp_address {
            continue; // Skip self
        }

        if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
            // 4. Serialize data
            let data_string = serde_json::to_string(&data)?;

            // 5. Create request
            let request = Request {
                command: command.clone(),
                data: data_string,
            };

            // 6. Send to peer
            stream.write_all(serde_json::to_string(&request)?.as_bytes()).await?;
        } else {
            // 7. Remove dead peer
            self.peers.lock().await.remove(&peer_address);
        }
    }
}
```

### Broadcast Flow

**1. Type Determination**:
```rust
match payload {
    BroadcastItem::NewBlock(block) => (block, "new_block", "block"),
    BroadcastItem::Transaction(tx) => (tx, "transaction", "transaction"),
}
```

Sets appropriate command string based on broadcast type.

**2. Peer Iteration**:
```rust
for peer_address in peers_list {
    if peer_address == self.tcp_address {
        continue; // Don't send to self
    }
    // ...
}
```

Broadcasts to all peers except self.

**3. TCP Connection**:
```rust
if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
    // Send data
} else {
    // Remove dead peer
    self.peers.lock().await.remove(&peer_address);
}
```

Establishes connection or removes unreachable peer.

**4. Message Sending**:
```rust
let request = Request {
    command: "new_block",
    data: serde_json::to_string(&block)?,
};

stream.write_all(serde_json::to_string(&request)?.as_bytes()).await?;
```

Serializes and sends request to peer.

### When Broadcasting Occurs

**New Transaction Received** (`src/handler.rs:45-48`):
```rust
server_handler.broadcaster.lock().await
    .broadcast_item(BroadcastItem::Transaction(tx.clone()))
    .await;
```

**New Block Mined** (`src/miner.rs:147-151`):
```rust
self.broadcaster.lock().await
    .broadcast_item(BroadcastItem::NewBlock(new_block.clone()))
    .await;
```

## Message Types

### 1. TRANSACTION

**Command**: `"transaction"`
**Purpose**: Broadcast new transaction to network

**Sender**: Client → Node → All Peers

**Data Format**:
```json
{
  "sender": "...",
  "recipient": "...",
  "amount": 10.0,
  "fee": 0.1,
  "timestamp": 1699876543,
  "signature": "..."
}
```

**Handler** (`src/server.rs:104-124`):
```rust
TRANSACTION => {
    if let Ok(tx) = serde_json::from_str::<Transaction>(&req.data) {
        // Check if already exists
        if !self.transaction_pool.lock().await.transaction_already_exists(&tx) {
            // Broadcast to other peers
            self.broadcaster.lock().await
                .broadcast_item(BroadcastItem::Transaction(tx.clone()))
                .await;
        }

        // Add to pool
        self.transaction_pool.lock().await.add_transaction(tx);
    }
}
```

**Flow**:
1. Receive transaction
2. Check if already in pool (avoid duplicate broadcasts)
3. Broadcast to other peers (if new)
4. Add to local transaction pool

### 2. NEW_BLOCK

**Command**: `"new_block"`
**Purpose**: Broadcast newly mined block

**Sender**: Miner → All Peers

**Data Format**: Full `Block` structure (JSON)

**Handler** (`src/server.rs:125-139`):
```rust
NEW_BLOCK => {
    if let Ok(block) = serde_json::from_str::<Block>(&req.data) {
        let latest_block = { self.blockchain.read().await.get_last_block().clone() };

        // Skip if already have this block
        if latest_block.index >= block.index || latest_block.hash == block.hash {
            return;
        }

        self.handle_new_block(block).await;
    }
}
```

**`handle_new_block`** (`src/server.rs:178-207`):
```rust
async fn handle_new_block(&self, block: Block) {
    let is_valid = {
        self.blockchain.read().await.is_valid_new_block(&block)
    };

    if is_valid {
        // Add to blockchain
        self.blockchain.write().await.add_block(block.clone());

        // Notify miner (interrupt current mining)
        self.block_tx.lock().await.send(Some(block.clone())).await?;

        // Broadcast to other peers
        self.broadcaster.lock().await
            .broadcast_item(BroadcastItem::NewBlock(block.clone()))
            .await;

        // Save to database
        tokio::spawn(async move {
            database.lock().await.store_block(&block)?;
        });
    }
}
```

**Flow**:
1. Receive block
2. Check if already have it (avoid duplicate processing)
3. Validate block (PoW, signatures, hash linkage)
4. Add to blockchain
5. Notify miner to interrupt
6. Broadcast to other peers
7. Persist to database

### 3. GET_BLOCKCHAIN

**Command**: `"get_blockchain"`
**Purpose**: Request full blockchain from peer

**Sender**: Sync component

**Data**: Empty string

**Handler** (`src/server.rs:140-161`):
```rust
GET_BLOCKCHAIN => {
    let chain = { self.blockchain.read().await.get_chain() };

    // Send each block with delimiter
    for block in chain {
        let block_json = serde_json::to_string(&block)?;
        let block_chunk = format!("{}<END_BLOCK>\n", block_json);

        stream.write_all(block_chunk.as_bytes()).await?;
    }

    // Send end marker
    stream.write_all(b"<END_CHAIN><END_BLOCK>\n").await?;
}
```

**Response Format**:
```
{"index":0,...}<END_BLOCK>
{"index":1,...}<END_BLOCK>
{"index":2,...}<END_BLOCK>
<END_CHAIN><END_BLOCK>
```

**Delimiters**:
- `<END_BLOCK>\n`: Separates individual blocks
- `<END_CHAIN>`: Marks end of blockchain

See [Consensus Documentation](consensus.md#receiving-blockchain-data) for receiver implementation.

### 4. REGISTER

**Command**: `"register"`
**Purpose**: Register node with bootstrap and receive peer list

**Sender**: New node → Bootstrap node

**Data Format**:
```json
{
  "id": "node2",
  "address": "127.0.0.1:5001"
}
```

**Handler** (`src/server.rs:163-175`):
```rust
REGISTER => {
    if let Ok(peer) = serde_json::from_str::<Peer>(&req.data) {
        server_info!("Peer registered: {} at {}", peer.id, peer.address);

        // Add to peer list
        self.peers.lock().await.insert(peer.address.clone());

        // Send back peer list
        let peers = { self.peers.lock().await.clone() };
        let peers_json = serde_json::to_string(&peers)?;

        stream.write_all(peers_json.as_bytes()).await?;
    }
}
```

**Response**: `HashSet<String>` of all registered peer addresses

## Server Components

### TCP Server (P2P)

**Location**: `src/server.rs:83-102`

```rust
pub async fn start_tcp_server(self: Arc<Self>, tcp_address: String) {
    let listener = TcpListener::bind(&tcp_address).await?;
    server_info!("TCP Server listening on {}", tcp_address);

    loop {
        let (stream, addr) = listener.accept().await?;
        server_info!("New connection from {}", addr);

        let handler = self.clone();
        tokio::spawn(async move {
            handler.handle_client(stream).await;
        });
    }
}
```

**Features**:
- Binds to configured TCP address
- Accepts incoming connections
- Spawns new task for each client
- Concurrent connection handling

### HTTP Server (RPC)

**Location**: `src/server.rs:59-81`

```rust
pub async fn start_http_server(self: Arc<Self>, http_address: String) {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(handler.clone()))
            .service(submit_transaction)
            .service(health_check)
            .service(create_wallet)
            .service(sign_and_submit_transaction)
            .service(sign_transaction)
            .service(get_transaction_by_hash)
            .service(get_transactions_by_wallet)
            .service(get_block_by_hash)
            .service(get_all_blocks)
            .service(get_wallet_balance)
    })
    .bind(http_address)?
    .run()
    .await
}
```

**Features**:
- Uses Actix-Web framework
- RESTful JSON API
- Concurrent request handling
- Shared state via `Arc<ServerHandler>`

### Message Handler

**Location**: `src/server.rs:104-177`

```rust
async fn handle_client(&self, mut stream: TcpStream) {
    let mut buffer = [0; 4096];

    if let Ok(n) = stream.read(&mut buffer).await {
        let data = String::from_utf8_lossy(&buffer[..n]);

        if let Ok(req) = serde_json::from_str::<Request>(&data) {
            match req.command.as_str() {
                TRANSACTION => { /* ... */ }
                NEW_BLOCK => { /* ... */ }
                GET_BLOCKCHAIN => { /* ... */ }
                REGISTER => { /* ... */ }
                _ => {
                    server_warn!("Unknown command: {}", req.command);
                }
            }
        }
    }
}
```

**Routing**:
- Reads TCP stream
- Deserializes `Request`
- Routes to appropriate handler based on command
- Handles unknown commands gracefully

## Error Handling

### Connection Failures

**Pattern**: Remove dead peers from peer list

**Example** (`src/broadcaster.rs:69-73`):
```rust
if let Ok(mut stream) = TcpStream::connect(&peer_address).await {
    // Send data
} else {
    // Remove unreachable peer
    self.peers.lock().await.remove(&peer_address);
}
```

**Rationale**:
- Prevents repeatedly attempting to connect to dead nodes
- Self-healing network
- No manual intervention required

### Duplicate Prevention

**Transactions** (`src/server.rs:108-113`):
```rust
if !self.transaction_pool.lock().await.transaction_already_exists(&tx) {
    // Only broadcast if new
    self.broadcaster.lock().await
        .broadcast_item(BroadcastItem::Transaction(tx.clone()))
        .await;
}
```

**Blocks** (`src/server.rs:131-134`):
```rust
if latest_block.index >= block.index || latest_block.hash == block.hash {
    return; // Already have this block
}
```

**Purpose**: Prevents infinite broadcast loops

### Serialization Errors

**Pattern**: Log and continue

**Example** (`src/discover.rs:48-54`):
```rust
let data = match serde_json::to_string(&this_peer) {
    Ok(result) => result,
    Err(err) => {
        discover_error!("failed to serialize peer data: {}", err);
        continue; // Try next peer
    }
};
```

**Rationale**:
- Individual failures don't crash the network
- Graceful degradation
- Logged for debugging

## Summary

**Network Architecture**:
- Hybrid P2P with optional bootstrap node
- Dual servers: TCP (P2P) + HTTP (RPC)
- Self-healing peer list management

**Peer Discovery**:
- Initial contact with bootstrap node
- Receive full peer list
- Periodic refresh every 60 seconds
- Automatic dead peer removal

**Broadcasting**:
- Generic implementation for blocks and transactions
- Broadcasts to all peers except self
- Prevents duplicate broadcasts
- Graceful handling of connection failures

**Message Types**:
- `TRANSACTION`: Propagate new transactions
- `NEW_BLOCK`: Propagate mined blocks
- `GET_BLOCKCHAIN`: Request full chain
- `REGISTER`: Join network and get peer list

**Error Handling**:
- Dead peer removal
- Duplicate prevention
- Graceful degradation
- Comprehensive logging

This design demonstrates the core principles of peer-to-peer networking in a simplified, educational blockchain implementation.
