use crate::wallet::Wallet;
use ordered_float::OrderedFloat;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{Message, Secp256k1};

use crate::utils::hash_public_key;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

// WARNING - This struct should be used for learning purposes only.
// Sharing public and private key, inside requests is a totally risky.
// Ideally, you should sign your transaction locally, and submit it through the node.
// You can use this struct for debugging purposes only
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignTransactionRequest {
    pub transaction: Transaction,
    pub public_key_hex: String,
    pub private_key_hex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    #[serde(with = "ordered_float_serde")]
    pub amount: OrderedFloat<f64>,
    #[serde(with = "ordered_float_serde")]
    pub fee: OrderedFloat<f64>, // NEW: Transaction fee
    pub timestamp: i64,
    pub signature: Option<String>, // Signature is optional until it's signed
}

mod ordered_float_serde {
    use ordered_float::OrderedFloat;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(value.into_inner())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let float_value = f64::deserialize(deserializer)?;
        Ok(OrderedFloat(float_value))
    }
}

impl Eq for Transaction {}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee //Transactions with the same fee, will be prioritized
            .cmp(&other.fee) // OrderedFloat reverses the order internally, so we need to use self before other
            .then_with(|| other.timestamp.cmp(&self.timestamp)) // If the fee is the same, the older transaction will be selected as the priority
    }
}

impl PartialEq<Self> for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.fee == other.fee && self.timestamp == other.timestamp
    }
}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Transaction {
    /// Create a new transaction (unsigned)
    #[allow(dead_code)]
    pub fn new(sender: String, recipient: String, amount: f64, fee: f64, timestamp: i64) -> Self {
        Transaction {
            sender,
            recipient,
            amount: OrderedFloat(amount),
            fee: OrderedFloat(fee),
            timestamp,
            signature: None,
        }
    }

    /// Sign the transaction using the sender's wallet private key
    pub fn sign(&mut self, wallet: &Wallet) {
        let secp = Secp256k1::new();

        // Serialize transaction data as bytes (including fee in the hash)
        let message_data = format!(
            "{}:{}:{}:{}:{}",
            self.sender, self.recipient, self.amount, self.fee, self.timestamp
        );
        let message_hash = Sha256::digest(message_data.as_bytes());

        // Create a message for signing
        let message = Message::from_digest(<[u8; 32]>::from(message_hash));

        // Sign the message with the sender's private key
        let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &wallet.private_key);

        // Serialize the recoverable signature to compact format (including recovery ID)
        let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();

        // Convert the recovery ID to an integer (0 or 1)
        let recovery_id_byte = recovery_id as u8;

        // Append the recovery ID to the signature bytes (64 bytes + 1 byte for recovery ID)
        let mut sig_with_recovery = sig_bytes.to_vec(); // Copy the signature bytes
        sig_with_recovery.push(recovery_id_byte); // Append recovery ID as a byte

        // Store the signature as a hex string
        self.signature = Some(hex::encode(sig_with_recovery));
    }

    /// Verify the transaction's signature
    pub fn verify(&self) -> bool {
        // TODO - Add this to handlers, to make sure no COINBASE transaction is sent through RPC nodes
        if self.sender == "COINBASE" {
            return true; // No signature needed for coinbase
        }

        let secp = Secp256k1::new();

        if let Some(signature_hex) = &self.signature {
            // Deserialize the signature as bytes
            let sig_bytes = match hex::decode(signature_hex) {
                Ok(bytes) => bytes,
                Err(_) => return false, // Return false if decoding the signature fails
            };

            // The recovery ID is the last byte of the signature bytes
            let recovery_id_byte = sig_bytes.last().cloned().unwrap_or(0); // Default to 0 if no recovery id
            let recovery_id = match RecoveryId::try_from(recovery_id_byte as i32) {
                Ok(id) => id,
                Err(_) => return false, // Return false if recovery ID is invalid
            };

            // Create a RecoverableSignature from the signature bytes and recovery ID
            let recoverable_sig =
                match RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id) {
                    Ok(sig) => sig,
                    Err(_) => return false, // Return false if signature deserialization fails
                };

            // Serialize transaction data (excluding signature) as bytes
            let message_data = format!(
                "{}:{}:{}:{}:{}",
                self.sender, self.recipient, self.amount, self.fee, self.timestamp
            );
            let message_hash = Sha256::digest(message_data.as_bytes());

            // Create a message for signature verification
            let message = Message::from_digest(<[u8; 32]>::from(message_hash));

            // Recover the public key using the recoverable signature
            let recovered_key = match secp.recover_ecdsa(&message, &recoverable_sig) {
                Ok(key) => key,
                Err(_) => return false, // Return false if recovery fails
            };

            // Hash the recovered public key to compare it to the sender's address
            let recovered_pub_key_hash = hash_public_key(&recovered_key);

            // Verify if the recovered address matches the sender's address
            return recovered_pub_key_hash == self.sender;
        }

        false // Return false if no signature is present
    }

    pub fn hash(&self) -> String {
        let tx_data = format!(
            "{}:{}:{}:{}:{}",
            self.sender, self.recipient, self.amount, self.fee, self.timestamp
        );

        let tx_hash = Sha256::digest(tx_data.as_bytes());
        hex::encode(tx_hash)
    }
}
