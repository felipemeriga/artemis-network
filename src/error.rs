use std::array::TryFromSliceError;
use std::io;
use thiserror::Error;
use serde_json::Error as JsonError;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Error converting private key to slice: {source}")]
    PrivateKeyConversion {
        #[from]
        source: TryFromSliceError,
    },
    
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
    BinCode,

    #[error("Error inserting data into the database: {source}")]
    Insert {
        #[from]
        source: io::Error,
    },

    #[error("Error serializing data: {source}")]
    Retrieve {
        #[from]
        source: JsonError,
    },
}

