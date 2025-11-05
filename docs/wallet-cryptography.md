# Wallet & Cryptography

This document explains how wallets work in Artemis Network, including key generation, address derivation, and cryptographic signing.

## Table of Contents
- [What is a Wallet?](#what-is-a-wallet)
- [Cryptographic Foundation](#cryptographic-foundation)
- [Wallet Structure](#wallet-structure)
- [Key Generation](#key-generation)
- [Address Derivation](#address-derivation)
- [Transaction Signing](#transaction-signing)
- [Signature Verification](#signature-verification)
- [Key Import/Export](#key-importexport)
- [Security Considerations](#security-considerations)

## What is a Wallet?

A **wallet** in Artemis Network is a cryptographic key pair that allows users to:
- **Receive coins**: Via unique address derived from public key
- **Send coins**: By signing transactions with private key
- **Prove ownership**: Without revealing private key

**Not a Container**: Unlike physical wallets, blockchain wallets don't "store" coins. They store keys that prove ownership of coins recorded on the blockchain.

## Cryptographic Foundation

### Elliptic Curve: secp256k1

**Curve**: `secp256k1` (same as Bitcoin and Ethereum)

**Location**: `src/wallet.rs:4`
```rust
use secp256k1::{PublicKey, Secp256k1, SecretKey};
```

**Properties**:
- **Domain Parameters**: Defined by standards (SECG)
- **Key Size**: 256-bit private key, 512-bit uncompressed public key
- **Security**: ~128-bit security level
- **Efficiency**: Fast signature generation and verification

### Why secp256k1?

**Industry Standard**:
- Used by Bitcoin, Ethereum, and many other blockchains
- Well-studied and trusted
- Battle-tested cryptographic security

**Performance**:
- Efficient signature generation
- Fast verification
- Small signature size (65 bytes with recovery ID)

**Recoverable Signatures**:
- Public key can be recovered from signature + message
- No need to transmit public key separately
- Saves transaction size

### Hash Function: SHA-256

**Used For**:
- Address derivation (hash of public key)
- Transaction signing (hash of transaction data)
- Block mining (proof-of-work)

**Location**: `src/utils.rs:3`
```rust
use sha2::{Digest, Sha256};
```

**Properties**:
- **Output Size**: 256 bits (32 bytes)
- **Security**: Collision-resistant, pre-image resistant
- **Standard**: FIPS 180-4

## Wallet Structure

**Location**: `src/wallet.rs:15-18`

```rust
pub struct Wallet {
    pub private_key: SecretKey,  // secp256k1 private key (32 bytes)
    pub public_key: PublicKey,   // secp256k1 public key (33/65 bytes)
}
```

### Fields

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `private_key` | `SecretKey` | 32 bytes | Secret key for signing transactions |
| `public_key` | `PublicKey` | 33 or 65 bytes | Public key corresponding to private key |

**Public Key Formats**:
- **Uncompressed**: 65 bytes (0x04 + x-coordinate + y-coordinate)
- **Compressed**: 33 bytes (0x02/0x03 + x-coordinate)

### Export Format

**Location**: `src/wallet.rs:7-13`

```rust
pub struct ExportWallet {
    pub private_key: String,  // Hex-encoded private key
    pub public_key: String,   // Hex-encoded public key
    pub address: String,      // SHA-256 hash of public key (hex)
}
```

Used for JSON serialization when exporting wallet keys.

## Key Generation

### Creating a New Wallet

**Location**: `src/wallet.rs:22-33`

```rust
pub fn new() -> Self {
    let secp = Secp256k1::new();

    // Use OsRng for cryptographically secure random numbers
    let mut rng = OsRng;

    let (secret_key, public_key) = secp.generate_keypair(&mut rng);

    Wallet {
        private_key: secret_key,
        public_key,
    }
}
```

### Key Generation Process

**1. Initialize secp256k1 Context**:
```rust
let secp = Secp256k1::new();
```

Creates cryptographic context for secp256k1 operations.

**2. Initialize Random Number Generator**:
```rust
let mut rng = OsRng;
```

`OsRng` uses operating system's cryptographically secure random number generator:
- **Linux/Unix**: `/dev/urandom`
- **Windows**: `BCryptGenRandom`
- **macOS**: `SecRandomCopyBytes`

**Why OsRng?**
- Cryptographically secure
- Non-deterministic
- Resistant to prediction attacks
- Not reproducible (each wallet is unique)

**3. Generate Key Pair**:
```rust
let (secret_key, public_key) = secp.generate_keypair(&mut rng);
```

**Process**:
1. Generate random 256-bit number (private key)
2. Ensure it's within secp256k1 curve order
3. Compute public key: `Public = Private × G` (G = generator point)
4. Return both keys

**Mathematical Relationship**:
```
Public Key = Private Key × Generator Point
```

This is a one-way function:
- Easy to compute public key from private key
- **Computationally infeasible** to derive private key from public key

### Private Key

**Format**: 256-bit integer (32 bytes)

**Range**: `1` to `n-1` where `n` is the curve order (~2^256)

**Example** (hex):
```
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

**Security**: Must be kept **absolutely secret**. Anyone with the private key can spend coins.

### Public Key

**Derivation**: `Public = Private × G`

**Formats**:

**Uncompressed** (65 bytes):
```
04 [x-coordinate (32 bytes)] [y-coordinate (32 bytes)]
```

**Compressed** (33 bytes):
```
02/03 [x-coordinate (32 bytes)]
```

Prefix `02` or `03` indicates y-coordinate parity.

**Example** (hex, compressed):
```
03e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

**Usage**: Can be shared publicly without compromising security.

## Address Derivation

**Address** is a hash of the public key, used as a shorter, checksummed identifier.

**Location**: `src/wallet.rs:48-50` and `src/utils.rs:6-15`

```rust
pub fn address(&self) -> String {
    hash_public_key(&self.public_key)
}

pub fn hash_public_key(public_key: &PublicKey) -> String {
    // 1. Serialize public key
    let pub_key_bytes = public_key.serialize();

    // 2. Hash with SHA-256
    let sha256_hash = Sha256::digest(pub_key_bytes);

    // 3. Encode as hex
    hex::encode(sha256_hash)
}
```

### Derivation Process

**1. Serialize Public Key**:
```rust
let pub_key_bytes = public_key.serialize();
```

Converts public key to byte array (33 bytes compressed or 65 bytes uncompressed).

**2. Hash with SHA-256**:
```rust
let sha256_hash = Sha256::digest(pub_key_bytes);
```

Produces 256-bit (32-byte) hash.

**3. Encode as Hexadecimal**:
```rust
hex::encode(sha256_hash)
```

Converts to 64-character hexadecimal string.

### Example

```
Private Key:
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

Public Key (compressed):
03e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

SHA-256 Hash:
9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08

Address (hex):
9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
```

### Why Hash the Public Key?

**Shorter Addresses**:
- Public key: 65 bytes (uncompressed) or 33 bytes (compressed)
- Address: 32 bytes (SHA-256 output)
- Easier to share and display

**Additional Security Layer**:
- Even if SHA-256 is broken, secp256k1 might still be secure
- Defense in depth

**Consistency**:
- Same format regardless of public key compression
- Uniform address length

**Standard Practice**:
- Bitcoin uses HASH160 (RIPEMD-160 of SHA-256)
- Ethereum uses Keccak-256
- Artemis uses SHA-256 for simplicity

## Transaction Signing

Signing proves that the transaction was created by the owner of the private key.

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

### Signing Process

**1. Message Construction**:
```rust
let message_data = format!(
    "{}:{}:{}:{}:{}",
    self.sender, self.recipient, self.amount, self.fee, self.timestamp
);
```

Concatenates transaction fields with `:` separator.

**Example**:
```
9f86d081...:a1b2c3d4...:10.5:0.1:1699876543
```

**2. Hash Message**:
```rust
let message_hash = Sha256::digest(message_data.as_bytes());
let message = Message::from_digest(<[u8; 32]>::from(message_hash));
```

**Why Hash?**
- ECDSA operates on fixed-size (32-byte) messages
- Provides data integrity
- Standard practice (sign hash, not raw data)

**3. Create ECDSA Signature**:
```rust
let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &wallet.private_key);
```

**ECDSA Algorithm**:
1. Generate random nonce `k`
2. Compute point `R = k × G`
3. Compute `r = R.x mod n` (x-coordinate of R)
4. Compute `s = k^-1 (hash + r × private_key) mod n`
5. Return signature `(r, s)` with recovery ID

**Recoverable Signature**:
- Standard ECDSA: `(r, s)` - 64 bytes
- Recoverable: `(r, s, recovery_id)` - 65 bytes
- Recovery ID allows deriving public key from signature

**4. Serialize Signature**:
```rust
let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();
let mut sig_with_recovery = sig_bytes.to_vec();
sig_with_recovery.push(recovery_id as u8);
```

**Format**:
- `sig_bytes`: 64 bytes (`r` + `s`, 32 bytes each)
- `recovery_id`: 1 byte (0-3, typically 0 or 1)
- Total: 65 bytes

**5. Encode as Hex**:
```rust
self.signature = Some(hex::encode(sig_with_recovery));
```

Converts to 130-character hexadecimal string.

### Signature Structure

```
Byte Layout:
[0-31]:   r (32 bytes)
[32-63]:  s (32 bytes)
[64]:     recovery_id (1 byte)

Total: 65 bytes → 130 hex characters
```

## Signature Verification

Verification proves the signature was created by the owner of the private key without revealing the private key.

**Location**: `src/transaction.rs:124-177`

```rust
pub fn verify(&self) -> bool {
    if self.sender == "COINBASE" {
        return true; // No verification for COINBASE
    }

    let secp = Secp256k1::new();

    if let Some(signature_hex) = &self.signature {
        // 1. Decode signature
        let sig_bytes = hex::decode(signature_hex)?;
        let recovery_id_byte = sig_bytes.last().cloned().unwrap_or(0);
        let recovery_id = RecoveryId::try_from(recovery_id_byte as i32)?;

        // 2. Deserialize signature
        let recoverable_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id)?;

        // 3. Recreate message hash
        let message_data = format!(
            "{}:{}:{}:{}:{}",
            self.sender, self.recipient, self.amount, self.fee, self.timestamp
        );
        let message_hash = Sha256::digest(message_data.as_bytes());
        let message = Message::from_digest(<[u8; 32]>::from(message_hash));

        // 4. Recover public key from signature
        let recovered_key = secp.recover_ecdsa(&message, &recoverable_sig)?;

        // 5. Hash recovered public key
        let recovered_pub_key_hash = hash_public_key(&recovered_key);

        // 6. Verify it matches sender address
        return recovered_pub_key_hash == self.sender;
    }

    false
}
```

### Verification Process

**1. Decode Signature**:
```rust
let sig_bytes = hex::decode(signature_hex)?;
let recovery_id = RecoveryId::try_from(sig_bytes[64] as i32)?;
let recoverable_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id)?;
```

Extract `r`, `s`, and recovery ID from 65-byte signature.

**2. Recreate Message Hash**:
```rust
let message_data = format!("{}:{}:{}:{}:{}", ...);
let message_hash = Sha256::digest(message_data.as_bytes());
```

Must hash message **exactly the same way** as signing.

**3. Recover Public Key**:
```rust
let recovered_key = secp.recover_ecdsa(&message, &recoverable_sig)?;
```

**Recovery Algorithm**:
1. Reconstruct point `R` from `r` and recovery ID
2. Compute `public_key = r^-1 (s × R - hash × G)`
3. Return recovered public key

**Mathematical Property**:
```
If signature (r, s) was created with private_key and message_hash:
Then recovered public_key = private_key × G
```

**4. Hash Recovered Public Key**:
```rust
let recovered_pub_key_hash = hash_public_key(&recovered_key);
```

Derive address from recovered public key.

**5. Compare to Sender Address**:
```rust
return recovered_pub_key_hash == self.sender;
```

If recovered address matches sender address, signature is valid.

### Why Public Key Recovery?

**Space Efficiency**:
- No need to include public key in transaction
- Saves 33-65 bytes per transaction

**Simplicity**:
- Transaction only needs sender address
- Public key derived during verification

**Standard Practice**:
- Used by Bitcoin, Ethereum
- Well-established technique

## Key Import/Export

### Exporting Wallet

**Location**: `src/wallet.rs:52-58`

```rust
pub fn export_wallet(&self) -> ExportWallet {
    ExportWallet {
        private_key: hex::encode(self.private_key.secret_bytes()),
        public_key: hex::encode(self.public_key.serialize()),
        address: self.address(),
    }
}
```

**Returns**:
```json
{
  "privateKey": "e3b0c442...",
  "publicKey": "03e3b0c4...",
  "address": "9f86d081..."
}
```

**Use Cases**:
- Backup wallet keys
- Import into another application
- Development/testing

### Importing Wallet

**Location**: `src/wallet.rs:35-45`

```rust
pub fn from_hex_string(public_key: String, private_key: String) -> Result<Wallet, WalletError> {
    // 1. Decode hex strings
    let decoded_public_key = hex::decode(public_key)?;
    let decoded_private_key = hex::decode(private_key)?;

    // 2. Convert to byte array
    let private_key_array: [u8; 32] = decoded_private_key.as_slice().try_into()?;

    // 3. Deserialize keys
    Ok(Wallet {
        public_key: PublicKey::from_slice(&decoded_public_key)?,
        private_key: SecretKey::from_byte_array(&private_key_array)?,
    })
}
```

**Input**: Hex-encoded strings
**Output**: `Wallet` instance

**Error Handling**:
- Invalid hex encoding
- Invalid key format
- Key out of curve range

## Security Considerations

### Private Key Security

**Critical Rules**:
1. **Never share** private keys
2. **Never transmit** over network (unencrypted)
3. **Never log** or print to console
4. **Secure storage**: Encrypt at rest

**In Artemis Network**:
- ⚠️ `/transaction/sign-and-submit` endpoint accepts private key in request body
- ⚠️ **FOR LEARNING PURPOSES ONLY**
- ⚠️ **NEVER use in production**

**Location**: `src/transaction.rs:11-14`
```rust
// WARNING - This struct should be used for learning purposes only.
// Sharing public and private key, inside requests is totally risky.
```

### Proper Transaction Workflow

**Secure**:
1. Create transaction on client
2. Sign transaction locally (private key never leaves device)
3. Submit **signed** transaction to node
4. Node verifies signature

**Insecure (Artemis dev endpoint)**:
1. Send transaction + private key to node
2. Node signs transaction
3. ❌ Private key transmitted over network

### Random Number Generation

**OsRng**:
- Cryptographically secure
- Non-deterministic
- OS-provided entropy

**Bad RNG Consequences**:
- Predictable private keys
- Key recovery from signatures
- Loss of funds

**Example**: PlayStation 3 ECDSA fail (2010) - Sony used constant nonce, private key recovered.

### Address Collision

**Probability**:
- Address space: 2^256 (SHA-256 output)
- Collision probability: ~0 (astronomically low)

**Birthday Attack**:
- Finding collision requires ~2^128 attempts
- Computationally infeasible with current technology

### Quantum Computing

**Future Threat**:
- Shor's algorithm can break ECDSA
- Requires large-scale quantum computer (not yet available)

**Mitigation**:
- Post-quantum cryptography research ongoing
- Address hashing provides some protection (Grover's algorithm only √speedup)

## Summary

**Wallet Components**:
- **Private Key**: 256-bit secret (32 bytes)
- **Public Key**: Derived from private key (33/65 bytes)
- **Address**: SHA-256 hash of public key (32 bytes)

**Cryptographic Algorithms**:
- **Curve**: secp256k1 (Bitcoin/Ethereum standard)
- **Signature**: ECDSA with public key recovery
- **Hash**: SHA-256

**Key Operations**:
- **Generation**: OsRng → secp256k1 key pair
- **Derivation**: Private → Public → Address
- **Signing**: Hash message → ECDSA sign with private key
- **Verification**: Recover public key → Hash → Compare to sender

**Security**:
- Private keys must be kept absolutely secret
- Public keys can be safely shared
- Signatures prove ownership without revealing private key
- Recoverable signatures save space by allowing public key derivation

This cryptographic design follows industry standards while maintaining simplicity for educational purposes.
