use crate::error::WalletError;
use hex;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
        // TODO - Remove unwrap
        let decoded_public_key = hex::decode(public_key)?;
        let decoded_private_key = hex::decode(private_key)?;

        let private_key_array: [u8; 32] = decoded_private_key.as_slice().try_into().unwrap();

        Ok(Wallet {
            public_key: PublicKey::from_slice(&decoded_public_key)?,
            private_key: SecretKey::from_byte_array(&private_key_array)?,
        })
    }

    pub fn public_key_from_hex_string(public_key: String) -> Result<PublicKey, WalletError> {
        let decoded_public_key = hex::decode(public_key)?;
        PublicKey::from_slice(&decoded_public_key)
            .map_err(|err| WalletError::Secp256k1Error { source: err })
    }

    /// Generates a hashed wallet address derived from the public key
    pub fn address(&self) -> String {
        // serialize the public key as bytes
        let pub_key_bytes = self.public_key.serialize();

        // Hash the public key using SHA-256
        let sha256_hash = Sha256::digest(&pub_key_bytes);

        // Return the address as a hex-encoded string
        hex::encode(sha256_hash)
    }

    pub fn export_wallet(&self) -> ExportWallet {
        ExportWallet {
            private_key: hex::encode(self.private_key.secret_bytes()),
            public_key: hex::encode(self.public_key.serialize()),
            address: self.address(),
        }
    }
}
