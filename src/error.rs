use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Hex Decode Error: {source}")]
    HexDecodeError {
        #[from]
        source: hex::FromHexError,
    },
    #[error("Error converting private and public key: {source}")]
    Secp256k1Error {
        #[from]
        source: secp256k1::Error,
    },
}
