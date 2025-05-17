use crate::chain::esplora::EsploraBlockchain;
use crate::error::SdkError;
use crate::models::Config;
use crate::persist::sqlite::SqliteStorage;
use crate::persist::Storage;
use crate::BreezSdk;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;

/// Builder for creating and configuring a BreezSdk instance
pub struct SdkBuilder {
    config: Config,
    storage: Option<Arc<dyn Storage + Send + Sync>>,
    chain_service: Option<Arc<EsploraBlockchain>>,
    mnemonic: String,
}

impl SdkBuilder {
    /// Creates a new SdkBuilder with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The SDK configuration
    /// * `mnemonic` - The mnemonic for the wallet
    ///
    /// # Returns
    ///
    /// A new SdkBuilder instance
    pub fn new(config: Config, mnemonic: String) -> Self {
        Self {
            config,
            storage: None,
            chain_service: None,
            mnemonic,
        }
    }

    /// Sets a custom storage implementation
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage implementation
    ///
    /// # Returns
    ///
    /// The updated SdkBuilder instance
    pub fn storage(mut self, storage: Arc<dyn Storage + Send + Sync>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Sets a custom chain service implementation
    ///
    /// # Arguments
    ///
    /// * `chain_service` - The chain service implementation
    ///
    /// # Returns
    ///
    /// The updated SdkBuilder instance
    pub fn chain_service(mut self, chain_service: Arc<EsploraBlockchain>) -> Self {
        self.chain_service = Some(chain_service);
        self
    }

    /// Builds the BreezSdk instance
    ///
    /// # Returns
    ///
    /// A Result containing either the initialized BreezSdk or an SdkError
    pub async fn build(self) -> Result<BreezSdk, SdkError> {
        // Create default storage if not provided
        let storage = match self.storage {
            Some(storage) => storage,
            None => {
                let path =
                    PathBuf::from(&self.config.data_dir).join(self.config.network.to_string());
                let db_path = path.join("breez-sdk-ark.db");
                fs::create_dir_all(&path)?;
                Arc::new(SqliteStorage::new(&db_path)?)
            }
        };

        // Create shutdown channel
        let (shutdown_sender, shutdown_receiver) = watch::channel(());

        // Create the SDK instance
        BreezSdk::new(
            self.config,
            self.mnemonic,
            storage,
            shutdown_sender,
            shutdown_receiver,
        )
        .await
    }
}
