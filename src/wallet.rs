use hex;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

pub struct Wallet {
    pub private_key: SecretKey,
    pub public_key: PublicKey,
}

impl Wallet {
    /// Creates a new wallet with a random keypair
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

    /// Generates a hashed wallet address derived from the public key
    pub fn address(&self) -> String {
        // Serialize the public key as bytes
        let pub_key_bytes = self.public_key.serialize();

        // Hash the public key using SHA-256
        let sha256_hash = Sha256::digest(&pub_key_bytes);

        // Return the address as a hex-encoded string
        hex::encode(sha256_hash)
    }
}
