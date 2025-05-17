use crate::error::SdkError;
use crate::models::{OffchainBalance, Payment, PaymentStatus, PaymentType};
use crate::persist::Storage;
use rusqlite::types::Type;
use rusqlite::{params, Connection};
use serde_json;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// SQLite implementation of the Storage trait
pub struct SqliteStorage {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Creates a new SQLite storage instance
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file
    ///
    /// # Returns
    ///
    /// A new `SqliteStorage` instance
    pub fn new(db_path: &Path) -> Result<Self, SdkError> {
        let connection = Connection::open(db_path)?;
        let storage = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        storage.init()?;
        Ok(storage)
    }

    /// Creates a new in-memory SQLite storage instance for testing
    ///
    /// # Returns
    ///
    /// A new in-memory `SqliteStorage` instance
    pub fn new_in_memory() -> Result<Self, SdkError> {
        let connection = Connection::open_in_memory()?;
        let storage = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        storage.init()?;
        Ok(storage)
    }

    fn init(&self) -> Result<(), SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        // Create payments table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS payments (
          id TEXT PRIMARY KEY,
          payment_type TEXT NOT NULL,
          status TEXT NOT NULL,
          amount INTEGER NOT NULL,
          fees INTEGER NOT NULL,
          timestamp INTEGER NOT NULL,
          description TEXT,          
          destination TEXT
      )",
            [],
        )?;

        // Create settings table for storing metadata like last_sync_offset
        connection.execute(
            "CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
      )",
            [],
        )?;

        Ok(())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        let value = connection.query_row(
            "SELECT value FROM settings WHERE key = ?",
            params![key],
            |row| row.get(0),
        );

        match value {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SdkError::StorageError(e.to_string())),
        }
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        connection.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![key, value],
        )?;

        Ok(())
    }
}

impl Storage for SqliteStorage {
    fn save_payment(&self, payment: &Payment) -> Result<(), SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        connection.execute(
            "INSERT OR REPLACE INTO payments (
                id, payment_type, status, amount, fees, timestamp, description, destination
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payment.id,
                payment.payment_type.to_string(),
                payment.status.to_string(),
                payment.amount,
                payment.fees,
                payment.timestamp,
                payment.description,
                payment.destination,
            ],
        )?;

        Ok(())
    }

    fn save_payments(&self, payments: &[Payment]) -> Result<(), SdkError> {
        // Acquire the lock on the connection
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        // Start a transaction to ensure atomicity
        let tx = connection.transaction()?;

        // First, collect all payment IDs to keep
        let mut payment_ids = Vec::with_capacity(payments.len());

        // Insert or update all payments in the list
        for payment in payments {
            tx.execute(
                "INSERT OR REPLACE INTO payments (
                    id, payment_type, status, amount, fees, timestamp, description, destination
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    payment.id,
                    payment.payment_type.to_string(),
                    payment.status.to_string(),
                    payment.amount,
                    payment.fees,
                    payment.timestamp,
                    payment.description,
                    payment.destination,
                ],
            )?;

            payment_ids.push(&payment.id);
        }

        // Delete any payments not in the list
        if !payment_ids.is_empty() {
            // Create placeholders for the IN clause
            let placeholders = payment_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let query = format!("DELETE FROM payments WHERE id NOT IN ({})", placeholders);

            // Convert payment_ids to a Vec of rusqlite::types::ToSql trait objects
            let params: Vec<&dyn rusqlite::types::ToSql> = payment_ids
                .iter()
                .map(|id| id as &dyn rusqlite::types::ToSql)
                .collect();

            tx.execute(&query, &params[..])?;
        } else {
            // If the payments list is empty, delete all payments
            tx.execute("DELETE FROM payments", [])?;
        }

        // Commit the transaction
        tx.commit()?;

        Ok(())
    }

    fn get_payment(&self, id: &str) -> Result<Option<Payment>, SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        let mut stmt = connection.prepare(
            "SELECT id, payment_type, status, amount, fees, timestamp, description, destination
             FROM payments
             WHERE id = ?",
        )?;

        let payment = stmt.query_row(params![id], |row| {
            let payment_type_str: String = row.get(1)?;
            let status_str: String = row.get(2)?;

            let payment_type = match payment_type_str.as_str() {
                "Sent" => PaymentType::Sent,
                "Received" => PaymentType::Received,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        1,
                        "Invalid payment type".to_string(),
                        Type::Text,
                    ))
                }
            };

            let status = match status_str.as_str() {
                "Pending" => PaymentStatus::Pending,
                "Completed" => PaymentStatus::Completed,
                "Failed" => PaymentStatus::Failed,
                "Expired" => PaymentStatus::Expired,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        2,
                        "Invalid payment status".to_string(),
                        Type::Text,
                    ))
                }
            };

            Ok(Payment {
                id: row.get(0)?,
                payment_type,
                status,
                amount: row.get(3)?,
                fees: row.get(4)?,
                timestamp: row.get(5)?,
                description: row.get(6)?,
                destination: row.get(7)?,
            })
        });

        match payment {
            Ok(payment) => Ok(Some(payment)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SdkError::StorageError(e.to_string())),
        }
    }

    fn list_payments(&self, offset: u32, limit: u32) -> Result<Vec<Payment>, SdkError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SdkError::StorageError("Failed to lock connection".to_string()))?;

        let mut stmt = connection.prepare(
            "SELECT id, payment_type, status, amount, fees, timestamp, description, destination
             FROM payments
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
        )?;

        let payment_iter = stmt.query_map(params![limit, offset], |row| {
            let payment_type_str: String = row.get(1)?;
            let status_str: String = row.get(2)?;

            let payment_type = match payment_type_str.as_str() {
                "Sent" => PaymentType::Sent,
                "Received" => PaymentType::Received,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        1,
                        "Invalid payment type".to_string(),
                        Type::Text,
                    ))
                }
            };

            let status = match status_str.as_str() {
                "Pending" => PaymentStatus::Pending,
                "Completed" => PaymentStatus::Completed,
                "Failed" => PaymentStatus::Failed,
                "Expired" => PaymentStatus::Expired,
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        2,
                        "Invalid payment status".to_string(),
                        Type::Text,
                    ))
                }
            };

            Ok(Payment {
                id: row.get(0)?,
                payment_type,
                status,
                amount: row.get(3)?,
                fees: row.get(4)?,
                timestamp: row.get(5)?,
                description: row.get(6)?,
                destination: row.get(7)?,
            })
        })?;

        let mut payments = Vec::new();
        for payment in payment_iter {
            payments.push(payment?);
        }

        Ok(payments)
    }

    fn save_offchain_balance(
        &self,
        balance: &crate::models::OffchainBalance,
    ) -> Result<(), SdkError> {
        // Serialize the OffchainBalance struct to JSON
        let json_value = serde_json::to_string(balance)
            .map_err(|e| SdkError::StorageError(format!("Failed to serialize balance: {}", e)))?;

        // Store the serialized JSON under a single key
        self.set_setting("offchain_balance", &json_value)
    }

    fn get_offchain_balance(&self) -> Result<crate::models::OffchainBalance, SdkError> {
        // Retrieve the serialized JSON from settings
        match self.get_setting("offchain_balance")? {
            Some(json_value) => {
                // Deserialize the JSON back to an OffchainBalance struct
                serde_json::from_str(&json_value).map_err(|e| {
                    SdkError::StorageError(format!("Failed to deserialize balance: {}", e))
                })
            }
            None => {
                // Return default balance if no value is found
                Ok(crate::models::OffchainBalance::default())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OffchainBalance, Payment, PaymentStatus, PaymentType};

    fn create_test_payment(id: &str, payment_type: PaymentType, status: PaymentStatus) -> Payment {
        Payment {
            id: id.to_string(),
            payment_type,
            status,
            amount: 1000,
            fees: 10,
            timestamp: 1620000000,
            description: Some("Test payment".to_string()),
            destination: Some("test_destination".to_string()),
        }
    }

    #[test]
    fn test_init() {
        let storage = SqliteStorage::new_in_memory().unwrap();
        // If we got here without error, initialization succeeded
        assert!(true);
    }

    #[test]
    fn test_save_and_get_payment() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Create a test payment
        let payment = create_test_payment("test_id_1", PaymentType::Sent, PaymentStatus::Completed);

        // Save the payment
        storage.save_payment(&payment).unwrap();

        // Retrieve the payment
        let retrieved_payment = storage.get_payment("test_id_1").unwrap().unwrap();

        // Verify the retrieved payment matches the original
        assert_eq!(payment.id, retrieved_payment.id);
        assert_eq!(payment.amount, retrieved_payment.amount);
        assert_eq!(
            payment.payment_type.to_string(),
            retrieved_payment.payment_type.to_string()
        );
        assert_eq!(
            payment.status.to_string(),
            retrieved_payment.status.to_string()
        );
    }

    #[test]
    fn test_get_nonexistent_payment() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Try to retrieve a payment that doesn't exist
        let result = storage.get_payment("nonexistent_id").unwrap();

        // Verify that None is returned
        assert!(result.is_none());
    }

    #[test]
    fn test_list_payments() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Create and save multiple test payments
        let payment1 =
            create_test_payment("test_id_3", PaymentType::Sent, PaymentStatus::Completed);
        let payment2 =
            create_test_payment("test_id_4", PaymentType::Received, PaymentStatus::Pending);
        let payment3 =
            create_test_payment("test_id_5", PaymentType::Received, PaymentStatus::Completed);

        storage.save_payment(&payment1).unwrap();
        storage.save_payment(&payment2).unwrap();
        storage.save_payment(&payment3).unwrap();

        // List all payments
        let all_payments = storage.list_payments(0, 10).unwrap();
        assert_eq!(3, all_payments.len());

        // Test pagination
        let first_page = storage.list_payments(0, 2).unwrap();
        assert_eq!(2, first_page.len());

        let second_page = storage.list_payments(2, 2).unwrap();
        assert_eq!(1, second_page.len());
    }

    #[test]
    fn test_save_and_get_offchain_balance() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Create a test balance
        let balance = OffchainBalance::new(5000, 10000);

        // Save the balance
        storage.save_offchain_balance(&balance).unwrap();

        // Retrieve the balance
        let retrieved_balance = storage.get_offchain_balance().unwrap();

        // Verify the retrieved balance matches the original
        assert_eq!(balance.pending_sats, retrieved_balance.pending_sats);
        assert_eq!(balance.confirmed_sats, retrieved_balance.confirmed_sats);
        assert_eq!(balance.total_sats(), retrieved_balance.total_sats());
    }

    #[test]
    fn test_update_offchain_balance() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Create and save an initial balance
        let initial_balance = OffchainBalance::new(1000, 2000);
        storage.save_offchain_balance(&initial_balance).unwrap();

        // Create and save an updated balance
        let updated_balance = OffchainBalance::new(3000, 4000);
        storage.save_offchain_balance(&updated_balance).unwrap();

        // Retrieve the balance and verify it was updated
        let retrieved_balance = storage.get_offchain_balance().unwrap();
        assert_eq!(updated_balance.pending_sats, retrieved_balance.pending_sats);
        assert_eq!(
            updated_balance.confirmed_sats,
            retrieved_balance.confirmed_sats
        );
        assert_eq!(updated_balance.total_sats(), retrieved_balance.total_sats());
    }

    #[test]
    fn test_default_offchain_balance() {
        let storage = SqliteStorage::new_in_memory().unwrap();

        // Retrieve the balance without saving one first
        let default_balance = storage.get_offchain_balance().unwrap();

        // Verify the default values are used
        assert_eq!(0, default_balance.pending_sats);
        assert_eq!(0, default_balance.confirmed_sats);
        assert_eq!(0, default_balance.total_sats());
    }
}
