use crate::wallet::Wallet;
use hex;
use secp256k1::ecdsa::Signature;
use secp256k1::{Message, Secp256k1};
use sha2::{Digest, Sha256};

pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: f64,
    pub signature: Option<String>, // Signature is optional until it's signed
}

impl Transaction {
    /// Create a new transaction (unsigned)
    pub fn new(sender: String, recipient: String, amount: f64) -> Self {
        Transaction {
            sender,
            recipient,
            amount,
            signature: None,
        }
    }

    /// Sign the transaction using the sender's wallet private key
    pub fn sign(&mut self, wallet: &Wallet) {
        let secp = Secp256k1::new();

        // Serialize transaction data as bytes
        let message_data = format!("{}:{}:{}", self.sender, self.recipient, self.amount);
        let message_hash = Sha256::digest(message_data.as_bytes());

        // Create a message for signing
        let message = Message::from_slice(&message_hash).expect("32 bytes required for message");

        // Sign the message with the sender's private key
        let sig = secp.sign_ecdsa(&message, &wallet.private_key);

        // Store the signature as a hex string
        self.signature = Some(hex::encode(sig.serialize_compact()));
    }

    /// Verify the transaction's signature
    pub fn verify(&self, sender_public_key: &secp256k1::PublicKey) -> bool {
        let secp = Secp256k1::new();

        // Ensure the transaction is signed
        if let Some(signature_hex) = &self.signature {
            // Deserialize the signature
            let sig_bytes = hex::decode(signature_hex).expect("Invalid signature hex");
            let signature = Signature::from_compact(&sig_bytes).expect("Invalid signature format");

            // Serialize transaction data as bytes
            let message_data = format!("{}:{}:{}", self.sender, self.recipient, self.amount);
            let message_hash = Sha256::digest(message_data.as_bytes());

            // Create a message for verification
            let message =
                Message::from_slice(&message_hash).expect("32 bytes required for message");

            // Verify the signature
            secp.verify_ecdsa(&message, &signature, sender_public_key)
                .is_ok()
        } else {
            false // No signature present
        }
    }
}
