use bitcoin::{psbt::ExtractTxError, secp256k1};
use thiserror::Error;

/// Error types for the Breez SDK
#[derive(Error, Debug)]
pub enum SdkError {
    /// Error from the BIP39 library
    #[error("Connect error: {0}")]
    ConnectError(String),

    /// Generic error with a message
    #[error("Generic error: {0}")]
    GenericError(String),

    /// Error from the Ark client
    #[error("Ark client error: {0}")]
    ArkClientError(String),

    /// Error related to the storage
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Error related to payment processing
    #[error("Payment error: {0}")]
    PaymentError(String),

    /// Error related to wallet operations
    #[error("Wallet error: {0}")]
    WalletError(String),

    /// Error related to network operations
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Error when the SDK is not initialized
    #[error("SDK not initialized")]
    NotInitialized,

    /// Error related to address parsing
    #[error("Address parsing error: {0}")]
    AddressParsingError(String),

    /// Error related to transaction processing
    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Invalid network")]
    InvalidNetwork,
}

impl From<ark_client::Error> for SdkError {
    fn from(err: ark_client::Error) -> Self {
        SdkError::ArkClientError(err.to_string())
    }
}

impl From<rusqlite::Error> for SdkError {
    fn from(err: rusqlite::Error) -> Self {
        SdkError::StorageError(err.to_string())
    }
}

impl From<std::io::Error> for SdkError {
    fn from(err: std::io::Error) -> Self {
        SdkError::GenericError(err.to_string())
    }
}

impl From<std::num::TryFromIntError> for SdkError {
    fn from(err: std::num::TryFromIntError) -> Self {
        SdkError::GenericError(err.to_string())
    }
}

impl From<secp256k1::Error> for SdkError {
    fn from(err: secp256k1::Error) -> Self {
        SdkError::GenericError(err.to_string())
    }
}

impl From<bitcoin::address::ParseError> for SdkError {
    fn from(err: bitcoin::address::ParseError) -> Self {
        SdkError::AddressParsingError(err.to_string())
    }
}

impl From<ExtractTxError> for SdkError {
    fn from(err: ExtractTxError) -> Self {
        SdkError::TransactionError(err.to_string())
    }
}
