use ark_core::ArkTransaction;
use sdk_common::prelude::{LNInvoice, LNOffer};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::SdkError;

/// Network configuration for the SDK
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Network {
    Bitcoin,
    Testnet,
    Signet,
    Regtest,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Bitcoin => write!(f, "Bitcoin"),
            Network::Testnet => write!(f, "Testnet"),
            Network::Signet => write!(f, "Signet"),
            Network::Regtest => write!(f, "Regtest"),
        }
    }
}

impl From<Network> for bitcoin::Network {
    fn from(network: Network) -> Self {
        match network {
            Network::Bitcoin => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Signet => bitcoin::Network::Signet,
            Network::Regtest => bitcoin::Network::Regtest,
        }
    }
}

pub struct ConnectRequest {
    /// The SDK [Config]
    pub config: Config,
    /// The mnemonic for the wallet
    pub mnemonic: String,
}

/// Configuration for the SDK
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// The network to connect to
    pub network: Network,
    /// The Ark server URL
    pub ark_server_url: String,
    /// The Esplora server URL
    pub esplora_url: String,
    /// Directory for storing data files (e.g., SQLite database)
    pub data_dir: String,
}

impl Config {
    /// Creates a default configuration for the specified network
    ///
    /// # Arguments
    ///
    /// * `network` - The Bitcoin network to use    
    /// * `data_dir` - Directory for storing data files
    ///
    /// # Returns
    ///
    /// A new `Config` instance with default settings for the specified network
    pub fn default_config(network: Network, data_dir: String) -> Result<Self, SdkError> {
        match network {
            Network::Bitcoin => Err(SdkError::InvalidNetwork),
            Network::Testnet => Err(SdkError::InvalidNetwork),
            Network::Signet => Ok(Self {
                network,
                ark_server_url: "https://mutinynet.arkade.sh".to_string(),
                esplora_url: "https://mutinynet.com/api".to_string(),
                data_dir,
            }),
            Network::Regtest => Ok(Self {
                network,
                ark_server_url: "http://localhost:7070".to_string(),
                esplora_url: "http://localhost:30000".to_string(),
                data_dir,
            }),
        }
    }
}

/// Represents a payment in the system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payment {
    /// Unique identifier for the payment
    pub id: String,
    /// Type of payment (e.g., sent, received)
    pub payment_type: PaymentType,
    /// Status of the payment (e.g., pending, completed)
    pub status: PaymentStatus,
    /// Amount in satoshis
    pub amount: u64,
    /// Fee amount in satoshis
    pub fees: u64,
    /// Unix timestamp when the payment was created
    pub timestamp: u64,
    /// Optional description/memo
    pub description: Option<String>,
    /// Optional destination address
    pub destination: Option<String>,
}

/// Type of payment
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PaymentType {
    Sent,
    Received,
}

impl fmt::Display for PaymentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentType::Sent => write!(f, "Sent"),
            PaymentType::Received => write!(f, "Received"),
        }
    }
}

/// Status of a payment
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Expired,
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "Pending"),
            PaymentStatus::Completed => write!(f, "Completed"),
            PaymentStatus::Failed => write!(f, "Failed"),
            PaymentStatus::Expired => write!(f, "Expired"),
        }
    }
}

impl From<ArkTransaction> for Payment {
    fn from(tx: ArkTransaction) -> Self {
        match tx {
            ArkTransaction::Boarding {
                txid,
                amount,
                confirmed_at,
            } => Payment {
                id: txid.to_string(),
                payment_type: PaymentType::Received,
                status: match confirmed_at {
                    Some(_) => PaymentStatus::Completed,
                    None => PaymentStatus::Pending,
                },
                amount: amount.to_sat(),
                fees: 0,
                timestamp: tx.created_at() as u64,
                description: None,
                destination: None,
            },
            ArkTransaction::Round {
                txid,
                amount,
                created_at,
            } => Payment {
                id: txid.to_string(),
                payment_type: match amount.is_positive() {
                    true => PaymentType::Received,
                    false => PaymentType::Sent,
                },
                status: PaymentStatus::Completed,
                amount: amount.to_sat().abs() as u64,
                fees: 0,
                timestamp: created_at as u64,
                description: None,
                destination: None,
            },
            ArkTransaction::Redeem {
                txid,
                amount,
                is_settled,
                created_at,
            } => Payment {
                id: txid.to_string(),
                payment_type: match amount.is_positive() {
                    true => PaymentType::Received,
                    false => PaymentType::Sent,
                },
                status: match is_settled {
                    true => PaymentStatus::Completed,
                    false => PaymentStatus::Pending,
                },
                amount: amount.to_sat().abs() as u64,
                fees: 0,
                timestamp: created_at as u64,
                description: None,
                destination: None,
            },
        }
    }
}

// Request/Response structures for the SDK methods

/// Request for getting the wallet balance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBalanceRequest {}

/// Represents the offchain balance with pending and confirmed amounts
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OffchainBalance {
    /// Pending balance in satoshis
    pub pending_sats: u64,
    /// Confirmed balance in satoshis
    pub confirmed_sats: u64,
}

impl OffchainBalance {
    /// Create a new OffchainBalance from pending and confirmed amounts
    pub fn new(pending_sats: u64, confirmed_sats: u64) -> Self {
        Self {
            pending_sats,
            confirmed_sats,
        }
    }

    /// Get the total balance (pending + confirmed)
    pub fn total_sats(&self) -> u64 {
        self.pending_sats + self.confirmed_sats
    }
}

/// Response for getting the wallet balance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    /// The offchain balance details
    pub balance: OffchainBalance,
}

/// Request for syncing the wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncWalletRequest {}

/// Response for syncing the wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncWalletResponse {}

/// Request for receiving on-chain funds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveOnchainRequest {}

/// Response for receiving on-chain funds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveOnchainResponse {
    /// Deposit address
    pub deposit_address: String,
}

/// Request for preparing an on-chain send
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareSendOnchainRequest {
    /// Amount in satoshis
    pub receiver_amount_sats: u64,
}

/// Response for preparing an on-chain send
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareSendOnchainResponse {
    /// Amount in satoshis
    pub receiver_amount_sats: u64,
    /// Estimated fee in satoshis
    pub fee_sats: u64,
}

/// Request for sending on-chain funds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendOnchainRequest {
    /// The prepare response
    pub prepare_send_onchain_response: PrepareSendOnchainResponse,
    /// Bitcoin address to send to
    pub onchain_address: String,
}

/// Response for sending on-chain funds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendOnchainResponse {
    /// Transaction ID of the onchain transaction
    pub tx_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethod {
    Bolt11Invoice { receiver_amount_sat: u64 },
    Bolt12Offer,
    BitcoinAddress { receiver_amount_sat: Option<u64> },
    ArkAddress { receiver_amount_sat: Option<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivePaymentRequest {
    pub payment_method: PaymentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivePaymentResponse {
    pub destination: String,
    pub fee_sat: u64,
}

/// Request for receiving Ark payments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveArkRequest {}

/// Response for receiving Ark payments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveArkResponse {
    /// Ark address
    pub address: String,
}

/// An argument when calling [crate::sdk::LiquidSdk::prepare_send_payment].
#[derive(Debug, Serialize, Clone)]
pub struct PrepareSendPaymentRequest {
    /// The destination we intend to pay to.
    /// Supports Ark addresses, BIP21 URIs, BOLT11 invoices, BOLT12 offers
    pub destination: String,

    /// Should only be set when paying directly onchain or to a BIP21 URI
    /// where no amount is specified, or when the caller wishes to drain
    pub amount: Option<PayAmount>,
}

#[derive(Debug, Serialize, Clone)]
pub enum PayAmount {
    /// The amount in satoshi that will be received
    Specific { receiver_amount_sat: u64 },

    /// Indicates that all available Bitcoin funds should be sent
    Drain,
}

/// Specifies the supported destinations which can be payed by the SDK
#[derive(Clone, Debug, Serialize)]
pub enum SendDestination {
    ArkAddress {
        address: String,
        receiver_amount_sat: u64,
    },
    Bolt11 {
        invoice: LNInvoice,
        /// A BIP353 address, in case one was used to resolve this BOLT11
        bip353_address: Option<String>,
    },
    Bolt12 {
        offer: LNOffer,
        receiver_amount_sat: u64,
        /// A BIP353 address, in case one was used to resolve this BOLT12
        bip353_address: Option<String>,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct PrepareSendPaymentResponse {
    pub destination: SendDestination,
    pub fees_sat: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SendPaymentRequest {
    pub prepare_response: PrepareSendPaymentResponse,
}

/// Returned when calling [crate::sdk::LiquidSdk::send_payment].
#[derive(Debug, Serialize)]
pub struct SendPaymentResponse {
    pub payment: Payment,
}

/// Request for listing payments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListPaymentsRequest {
    /// Number of payments to skip
    pub offset: u32,
    /// Maximum number of payments to return
    pub limit: u32,
}

/// Response for listing payments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListPaymentsResponse {
    /// List of payments
    pub payments: Vec<Payment>,
}
