use std::io;
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

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Error converting data to bincode")]
    BinCodeError,

    // Database errors
    #[error("Error inserting data into the database: {source}")]
    DatabaseInsertError {
        #[from]
        source: io::Error,
    },
}
