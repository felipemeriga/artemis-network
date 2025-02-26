use crate::error::WalletError;
use crate::utils::hash_public_key;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWallet {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

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

    pub fn from_hex_string(public_key: String, private_key: String) -> Result<Wallet, WalletError> {
        let decoded_public_key = hex::decode(public_key)?;
        let decoded_private_key = hex::decode(private_key)?;

        let private_key_array: [u8; 32] = decoded_private_key.as_slice().try_into()?;

        Ok(Wallet {
            public_key: PublicKey::from_slice(&decoded_public_key)?,
            private_key: SecretKey::from_byte_array(&private_key_array)?,
        })
    }

    /// Generates a hashed wallet address derived from the public key
    pub fn address(&self) -> String {
        hash_public_key(&self.public_key)
    }

    pub fn export_wallet(&self) -> ExportWallet {
        ExportWallet {
            private_key: hex::encode(self.private_key.secret_bytes()),
            public_key: hex::encode(self.public_key.serialize()),
            address: self.address(),
        }
    }
}
