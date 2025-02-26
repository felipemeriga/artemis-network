use crate::error::WalletError;
use secp256k1::PublicKey;
use sha2::{Digest, Sha256};

// Function to hash a public key (use the same hashing scheme as used for addresses)
pub fn hash_public_key(public_key: &secp256k1::PublicKey) -> String {
    // serialize the public key as bytes
    let pub_key_bytes = public_key.serialize();

    // Hash the public key using SHA-256
    let sha256_hash = Sha256::digest(pub_key_bytes);

    // Return the address as a hex-encoded string
    hex::encode(sha256_hash)
}

#[allow(dead_code)]
pub fn public_key_from_hex_string(public_key: String) -> Result<PublicKey, WalletError> {
    let decoded_public_key = hex::decode(public_key)?;
    PublicKey::from_slice(&decoded_public_key)
        .map_err(|err| WalletError::Secp256k1Error { source: err })
}
