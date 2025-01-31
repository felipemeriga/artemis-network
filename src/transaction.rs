use crate::wallet::Wallet;
use hex;
use ordered_float::OrderedFloat;
use secp256k1::ecdsa::Signature;
use secp256k1::{Message, Secp256k1};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

        // serialize transaction data as bytes (include fee in hash)
        let message_data = format!(
            "{}:{}:{}:{}:{}",
            self.sender, self.recipient, self.amount, self.fee, self.timestamp
        );
        let message_hash = Sha256::digest(message_data.as_bytes());

        // Create a message for signing
        let message = Message::from_digest(<[u8; 32]>::from(message_hash));

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

            // serialize transaction data as bytes (include fee in hash)
            let message_data = format!(
                "{}:{}:{}:{}:{}",
                self.sender, self.recipient, self.amount, self.fee, self.timestamp
            );
            let message_hash = Sha256::digest(message_data.as_bytes());

            // Create a message for verification
            let message =
                Message::from_digest(<[u8; 32]>::from(message_hash));

            // Verify the signature
            secp.verify_ecdsa(&message, &signature, sender_public_key)
                .is_ok()
        } else {
            false // No signature present
        }
    }

    /// Check if sender has sufficient balance before signing the transaction
    pub fn has_sufficient_balance(&self, balances: &HashMap<String, f64>) -> bool {
        if let Some(balance) = balances.get(&self.sender) {
            *balance >= *(self.amount + self.fee)
        } else {
            false // Sender not found in balance list
        }
    }
}
