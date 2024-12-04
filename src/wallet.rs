use secp256k1::{Secp256k1, SecretKey, PublicKey};
use secp256k1::rand::rngs::OsRng;

pub struct Wallet {
    pub private_key: SecretKey,
    pub public_key: PublicKey,
}

impl Wallet {
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

    pub fn address(&self) -> String {
        format!("{:x}", self.public_key)
    }
}
