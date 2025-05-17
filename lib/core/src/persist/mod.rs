pub(crate) mod ark;
pub(crate) mod sqlite;

use crate::error::SdkError;
use crate::models::{OffchainBalance, Payment, PaymentStatus, PaymentType};

/// Trait for persistent storage implementations
pub trait Storage {
    /// Save a payment to the storage
    fn save_payment(&self, payment: &Payment) -> Result<(), SdkError>;

    /// Save a list of payments and delete any payments that don't exist in the list
    fn save_payments(&self, payments: &[Payment]) -> Result<(), SdkError>;

    /// Get a payment by ID
    fn get_payment(&self, id: &str) -> Result<Option<Payment>, SdkError>;

    /// List payments with pagination
    fn list_payments(&self, offset: u32, limit: u32) -> Result<Vec<Payment>, SdkError>;

    /// Save the offchain balance
    fn save_offchain_balance(&self, balance: &OffchainBalance) -> Result<(), SdkError>;

    /// Get the offchain balance
    fn get_offchain_balance(&self) -> Result<OffchainBalance, SdkError>;
}
