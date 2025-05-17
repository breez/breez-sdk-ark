pub mod chain;
pub mod error;
pub mod events;
mod logger;
pub mod models;
pub mod persist;
pub mod sdk_builder;

use ark_bdk_wallet::Wallet;
use ark_client::{Client, OfflineClient};
use ark_core::ArkAddress;
use bitcoin::{
    key::Secp256k1,
    secp256k1::{Keypair, SecretKey},
    Address, Amount,
};
use chain::esplora::EsploraBlockchain;
use error::SdkError;
use log::{error, info};
use models::{Config, ConnectRequest, PrepareSendOnchainRequest, PrepareSendOnchainResponse};
use persist::ark::InMemoryDb;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

// Export the persist module for external use
pub use persist::Storage;
// Export events module for external use
pub use events::{EventEmitter, EventListener, SdkEvent};

pub use models::{
    GetBalanceRequest, GetBalanceResponse, ListPaymentsRequest, ListPaymentsResponse, PayAmount,
    Payment, PaymentMethod, PaymentStatus, PaymentType, PrepareSendPaymentRequest,
    PrepareSendPaymentResponse, ReceiveArkRequest, ReceiveArkResponse, ReceiveOnchainRequest,
    ReceiveOnchainResponse, ReceivePaymentRequest, ReceivePaymentResponse, SendDestination,
    SendOnchainRequest, SendOnchainResponse, SendPaymentRequest, SendPaymentResponse,
    SyncWalletRequest, SyncWalletResponse,
};
use tokio::sync::watch;

// Export the builder module
pub use sdk_builder::SdkBuilder;

#[derive(Clone)]
pub struct BreezSdk {
    ark_client: Arc<Client<EsploraBlockchain, Wallet<InMemoryDb>>>,
    config: Config,
    storage: Arc<dyn Storage + Send + Sync>,
    event_emitter: Arc<EventEmitter>,
    shutdown_sender: watch::Sender<()>,
    shutdown_receiver: watch::Receiver<()>,
}

pub async fn connect(request: ConnectRequest) -> Result<BreezSdk, SdkError> {
    let sdk = SdkBuilder::new(request.config, request.mnemonic)
        .build()
        .await?;
    sdk.start()?;
    Ok(sdk)
}

impl BreezSdk {
    /// Creates a new instance of the `BreezSdk`
    ///
    /// # Arguments
    ///
    /// * `config` - The Sdk configuration object
    /// * `storage` - Storage implementation for persistent data    
    /// * `shutdown_sender` - Sender for shutdown signal
    /// * `shutdown_receiver` - Receiver for shutdown signal
    ///
    /// # Returns
    ///
    /// Result containing either the initialized `BreezSdk` or an `SdkError`
    pub async fn new(
        config: Config,
        mnemonic: String,
        storage: Arc<dyn Storage + Send + Sync>,
        shutdown_sender: watch::Sender<()>,
        shutdown_receiver: watch::Receiver<()>,
    ) -> Result<Self, SdkError> {
        // Initialize the Ark client with the server URL and mnemonic from the config
        let mnemonic: bip39::Mnemonic = mnemonic
            .parse()
            .map_err(|e: bip39::Error| SdkError::ConnectError(e.to_string()))?;
        let seed = mnemonic.to_seed("").to_vec();
        let ark_client = Arc::new(Self::init_client(config.clone(), seed).await?);

        Ok(Self {
            ark_client,
            config,
            storage,
            event_emitter: Arc::new(EventEmitter::new()),
            shutdown_sender,
            shutdown_receiver,
        })
    }

    async fn init_client(
        config: Config,
        seed: Vec<u8>,
    ) -> Result<Client<EsploraBlockchain, Wallet<InMemoryDb>>, SdkError> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&seed[..32])?;

        let keypair = Keypair::from_secret_key(&secp, &secret_key);

        // Initialize blockchain and wallet implementations
        let blockchain = Arc::new(EsploraBlockchain::new(config.esplora_url.to_string())?);
        let wallet = Wallet::new(
            keypair,
            secp,
            config.network.into(),
            &config.esplora_url,
            InMemoryDb::default(),
        )
        .map_err(|e| SdkError::WalletError(e.to_string()))?;
        let wallet = Arc::new(wallet);

        // Create the offline client
        let offline_client = OfflineClient::new(
            "breez-sdk-ark-client".to_string(),
            keypair,
            blockchain,
            wallet,
            config.ark_server_url,
        );

        // Connect to the Ark server and get server info
        let client = offline_client.connect().await?;

        Ok(client)
    }

    /// Registers a listener to receive SDK events
    ///
    /// # Arguments
    ///
    /// * `listener` - An implementation of the `EventListener` trait
    ///
    /// # Returns
    ///
    /// A unique identifier for the listener, which can be used to remove it later
    pub fn add_event_listener(&self, listener: Box<dyn EventListener>) -> String {
        self.event_emitter.add_listener(listener)
    }

    /// Removes a previously registered event listener
    ///
    /// # Arguments
    ///
    /// * `id` - The listener ID returned from `add_event_listener`
    ///
    /// # Returns
    ///
    /// `true` if the listener was found and removed, `false` otherwise
    pub fn remove_event_listener(&self, id: &str) -> bool {
        self.event_emitter.remove_listener(id)
    }

    /// Starts the SDK's background tasks
    ///
    /// This method initiates the following background tasks:
    /// 1. `periodic_sync`: the wallet with the Ark network    
    ///
    pub fn start(&self) -> Result<(), SdkError> {
        // TODO: Implement start functionality
        self.periodic_sync();
        Ok(())
    }

    fn periodic_sync(&self) {
        let sdk = self.clone();
        let mut shutdown_receiver = sdk.shutdown_receiver.clone();
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_receiver.changed() => {
                        info!("Periodic sync loop shutdown signal received");
                        return;
                    }
                    _ = interval.tick() => {
                        // Perform the sync operation
                        if let Err(e) = sdk.sync_wallet_internal().await {
                            error!("Periodic wallet sync failed: {e:?}");
                        }
                    }
                }
            }
        });
    }

    /// Stops the SDK's background tasks
    ///
    /// This method stops the background tasks started by the `start()` method.
    /// It should be called before your application terminates to ensure proper cleanup.
    ///
    /// # Returns
    ///
    /// Result containing either success or an `SdkError` if the background task couldn't be stopped
    pub fn disconnect(&self) -> Result<(), SdkError> {
        self.shutdown_sender
            .send(())
            .map_err(|_| SdkError::GenericError("Failed to send shutdown signal".to_string()))?;

        Ok(())
    }

    /// Returns the balance of the wallet in satoshis
    pub async fn get_balance(
        &self,
        _request: GetBalanceRequest,
    ) -> Result<GetBalanceResponse, SdkError> {
        // Retrieve the persisted offchain balance from storage
        let balance = self.storage.get_offchain_balance()?;

        Ok(GetBalanceResponse { balance })
    }

    /// Synchronizes the wallet with the Ark network
    /// As part of this sync we also attempt to join a round
    pub async fn sync_wallet(
        &self,
        _request: SyncWalletRequest,
    ) -> Result<SyncWalletResponse, SdkError> {
        let mut rng = StdRng::from_entropy();
        if let Err(e) = self.ark_client.board(&mut rng).await {
            error!("Failed to board: {e:?}");
            return Err(SdkError::GenericError(e.to_string()));
        }
        self.sync_wallet_internal().await?;
        Ok(SyncWalletResponse {})
    }

    async fn sync_wallet_internal(&self) -> Result<(), SdkError> {
        let start_time = Instant::now();

        // 1. Sync balance
        let ark_balance = self.ark_client.offchain_balance().await?;
        info!("Synced balance: {}", ark_balance.total().to_sat());

        // Convert to our OffchainBalance model
        let offchain_balance = models::OffchainBalance {
            pending_sats: ark_balance.pending().to_sat(),
            confirmed_sats: ark_balance.confirmed().to_sat(),
        };

        // Persist the balance to storage
        self.storage.save_offchain_balance(&offchain_balance)?;

        // 2. Sync transactions
        self.sync_payments_to_storage().await?;

        let elapsed = start_time.elapsed();
        info!("Wallet sync completed in {:?}", elapsed);
        self.event_emitter.emit(&SdkEvent::Synced {});

        Ok(())
    }

    /// Generates a new deposit address for receiving funds into the Ark wallet
    pub async fn receive_onchain(
        &self,
        _request: ReceiveOnchainRequest,
    ) -> Result<ReceiveOnchainResponse, SdkError> {
        let boarding_address = self.ark_client.get_boarding_address()?;
        Ok(ReceiveOnchainResponse {
            deposit_address: boarding_address.to_string(),
        })
    }

    /// Prepares a transaction to send funds on-chain without broadcasting it
    pub async fn prepare_send_onchain(
        &self,
        request: PrepareSendOnchainRequest,
    ) -> Result<PrepareSendOnchainResponse, SdkError> {
        info!(
            "Preparing on-chain transaction for amount: {}",
            request.receiver_amount_sats
        );

        // TODO: We should calculat the correct fees or get from the user the fee preference.
        Ok(PrepareSendOnchainResponse {
            receiver_amount_sats: request.receiver_amount_sats,
            fee_sats: 0,
        })
    }

    /// Initiates a withdrawal to move funds from Ark to on-chain Bitcoin
    pub async fn send_onchain(
        &self,
        request: SendOnchainRequest,
    ) -> Result<SendOnchainResponse, SdkError> {
        info!(
            "Initiating on-chain withdrawal to address: {}",
            request.onchain_address
        );

        // let mut rng = StdRng::from_entropy();
        // let txid = self
        //     .ark_client
        //     .off_board(
        //         &mut rng,
        //         Address::from_str(&request.onchain_address)?
        //             .require_network(self.config.clone().network.into())?,
        //         Amount::from_sat(request.prepare_send_onchain_response.receiver_amount_sats),
        //     )
        //     .await?;

        let txid = self
            .ark_client
            .send_on_chain(
                Address::from_str(&request.onchain_address)?
                    .require_network(self.config.clone().network.into())?,
                Amount::from_sat(request.prepare_send_onchain_response.receiver_amount_sats),
            )
            .await?;

        Ok(SendOnchainResponse {
            tx_id: txid.to_string(),
        })
    }

    /// Generates a payment destination based on the requested payment method
    ///
    /// This method handles different payment methods (Ark address, Bitcoin address, BOLT11, BOLT12)
    /// and returns the appropriate destination for receiving funds.
    ///
    /// # Arguments
    ///
    /// * `request` - Contains the payment method and amount information
    ///
    /// # Returns
    ///
    /// * `Ok(ReceivePaymentResponse)` - Contains the destination and fee information
    /// * `Err(SdkError)` - If there was an error generating the payment destination
    pub async fn receive_payment(
        &self,
        request: ReceivePaymentRequest,
    ) -> Result<ReceivePaymentResponse, SdkError> {
        info!(
            "Generating payment destination for method: {:?}",
            request.payment_method
        );

        match request.payment_method {
            PaymentMethod::ArkAddress {
                receiver_amount_sat,
            } => {
                // For Ark payments, we just need to return the Ark address
                let (ark_address, _) = self.ark_client.get_offchain_address()?;

                let fee_sat = 0;

                // TODO: in case of amount is given we should return bip21 url
                Ok(ReceivePaymentResponse {
                    destination: ark_address.encode(),
                    fee_sat,
                })
            }
            PaymentMethod::BitcoinAddress {
                receiver_amount_sat,
            } => {
                // For Bitcoin address payments, we generate an on-chain address
                let address = self.ark_client.get_boarding_address()?.to_string();

                let fee_sat = 0;

                // TODO: in case of amount is given we should return bip21 url
                Ok(ReceivePaymentResponse {
                    destination: address,
                    fee_sat,
                })
            }
            PaymentMethod::Bolt11Invoice { .. } => Err(SdkError::GenericError(
                "BOLT11 invoice generation is not yet implemented".to_string(),
            )),
            PaymentMethod::Bolt12Offer => Err(SdkError::GenericError(
                "BOLT12 offer generation is not yet implemented".to_string(),
            )),
        }
    }

    /// Synchronizes payments to persistent storage
    async fn sync_payments_to_storage(&self) -> Result<(), SdkError> {
        let ark_transactions = self.ark_client.transaction_history().await?;
        info!("Syncing ark_transactions: {:#?}", ark_transactions);

        // Convert all transactions to payments
        let mut payments = Vec::with_capacity(ark_transactions.len());
        for ark_transaction in ark_transactions {
            let payment = Payment::from(ark_transaction);
            info!("Converted payment: {:?}", payment);
            payments.push(payment);
        }

        // Save all payments at once and delete any that don't exist in the list
        self.storage.save_payments(&payments)?;

        Ok(())
    }

    /// Lists payments from the storage with pagination
    ///
    /// This method provides direct access to the payment history stored in the database.
    /// It returns payments in reverse chronological order (newest first).
    ///
    /// # Arguments
    ///
    /// * `request` - Contains pagination parameters (offset and limit)
    ///
    /// # Returns
    ///
    /// * `Ok(ListPaymentsResponse)` - Contains the list of payments if successful
    /// * `Err(SdkError)` - If there was an error accessing the storage
    ///
    pub async fn list_payments(
        &self,
        request: ListPaymentsRequest,
    ) -> Result<ListPaymentsResponse, SdkError> {
        info!("Listing payments with filter: {:?}", request);

        // Retrieve payments from storage with pagination parameters
        let payments = self.storage.list_payments(request.offset, request.limit)?;

        // Return the payments in the response
        Ok(ListPaymentsResponse { payments })
    }

    /// Prepares a payment to a destination
    ///
    /// This method analyzes the destination string and prepares the appropriate payment type.
    /// It supports Ark addresses and potentially other payment types in the future.
    ///
    /// # Arguments
    ///
    /// * `request` - Contains the destination and optional amount information
    ///
    /// # Returns
    ///
    /// * `Ok(PrepareSendResponse)` - Contains the parsed destination and estimated fees
    /// * `Err(SdkError)` - If there was an error preparing the payment
    pub async fn prepare_send_payment(
        &self,
        request: PrepareSendPaymentRequest,
    ) -> Result<PrepareSendPaymentResponse, SdkError> {
        info!("Preparing payment to destination: {}", request.destination);

        // Try to parse as an Ark address. TODO: We should use input parser to parse this.
        if let Ok(ark_address) = ArkAddress::decode(&request.destination) {
            // Get the amount to send
            let receiver_amount_sat = match request.amount {
                Some(PayAmount::Specific {
                    receiver_amount_sat,
                }) => receiver_amount_sat,
                Some(PayAmount::Drain) => {
                    // Get the current balance and use all available funds
                    let balance_response = self.get_balance(GetBalanceRequest {}).await?;
                    let total_balance = balance_response.balance.total_sats();
                    // We don't handle fees here, they'll be calculated during actual sending
                    total_balance
                }
                None => {
                    return Err(SdkError::GenericError(
                        "Amount is required for Ark address payments".to_string(),
                    ))
                }
            };

            // For Ark payments, we don't have separate fees
            // The fees will be calculated during the actual send operation
            let fees_sat = None;

            Ok(PrepareSendPaymentResponse {
                destination: SendDestination::ArkAddress {
                    address: ark_address.to_string(),
                    receiver_amount_sat,
                },
                fees_sat,
            })
        } else {
            // Could add support for other destination types here (BOLT11, BOLT12, etc.)
            Err(SdkError::GenericError(format!(
                "Unsupported destination format: {}",
                request.destination
            )))
        }
    }

    /// Sends a payment based on a previously prepared payment request
    ///
    /// # Arguments
    ///
    /// * `request` - Contains the prepared payment information from prepare_send_payment
    ///
    /// # Returns
    ///
    /// * `Ok(SendPaymentResponse)` - Contains the payment details if successful
    /// * `Err(SdkError)` - If there was an error sending the payment
    pub async fn send_payment(
        &self,
        request: SendPaymentRequest,
    ) -> Result<SendPaymentResponse, SdkError> {
        info!(
            "Sending payment with prepared response: {:?}",
            request.prepare_response
        );

        match &request.prepare_response.destination {
            SendDestination::ArkAddress {
                address,
                receiver_amount_sat,
            } => {
                let ark_address = ArkAddress::decode(address)
                    .map_err(|_| SdkError::AddressParsingError(address.clone()))?;

                let amount = Amount::from_sat(*receiver_amount_sat);

                // Use the Ark client to send the VTXO
                let psbt = self
                    .ark_client
                    .send_vtxo(ark_address, amount)
                    .await
                    .map_err(|e| {
                        SdkError::GenericError(format!("Failed to send payment: {}", e))
                    })?;
                let txid = psbt.extract_tx()?.compute_txid();
                // Create a payment record
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let payment = Payment {
                    id: txid.to_string(),
                    payment_type: PaymentType::Sent,
                    status: PaymentStatus::Pending,
                    amount: *receiver_amount_sat,
                    fees: 0, // We don't have separate fees for Ark payments
                    timestamp,
                    description: None,
                    destination: Some(address.clone()),
                };

                // Save the payment to storage
                self.sync_wallet_internal().await?;

                Ok(SendPaymentResponse { payment })
            }
            SendDestination::Bolt11 { invoice, .. } => {
                return Err(SdkError::GenericError(
                    "BOLT11 payments are not yet implemented".to_string(),
                ));
            }
            SendDestination::Bolt12 { offer, .. } => {
                return Err(SdkError::GenericError(
                    "BOLT12 payments are not yet implemented".to_string(),
                ));
            }
        }
    }

    /// Configures a global SDK logger that will log to file and will forward log events to
    /// an optional application-specific logger.
    ///
    /// If called, it should be called before any SDK methods (for example, before `connect`).
    ///
    /// It must be called only once in the application lifecycle. Alternatively, If the application
    /// already uses a globally-registered logger, this method shouldn't be called at all.
    ///
    /// ### Arguments
    ///
    /// - `log_dir`: Location where the SDK log file will be created. The directory must already exist.
    ///
    /// - `app_logger`: Optional application logger.
    ///
    /// If the application is to use it's own logger, but would also like the SDK to log SDK-specific
    /// log output to a file in the configured `log_dir`, then do not register the
    /// app-specific logger as a global logger and instead call this method with the app logger as an arg.
    ///
    /// ### Errors
    ///
    /// An error is thrown if the log file cannot be created in the working directory.
    ///
    /// An error is thrown if a global logger is already configured.
    ///
    pub fn init_logging(
        log_dir: &str,
        app_logger: Option<Box<dyn log::Log>>,
    ) -> anyhow::Result<()> {
        // Initialize the logger using the logger module
        crate::logger::SdkLogger::init(log_dir, app_logger)
    }
}
