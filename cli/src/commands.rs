use breez_sdk_ark::SendOnchainRequest;
use breez_sdk_ark::{
    models::PrepareSendOnchainRequest, BreezSdk, GetBalanceRequest, ListPaymentsRequest, PayAmount,
    PaymentMethod, PrepareSendPaymentRequest, ReceiveOnchainRequest, ReceivePaymentRequest,
    SendPaymentRequest, SyncWalletRequest,
};
use clap::arg;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::{Completer, Helper, Hinter, Validator};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::borrow::Cow;
use std::borrow::Cow::Owned;
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct WalletConfig {
    network: String,
    mnemonic: String,
}

#[derive(Clone, clap::clap_derive::Parser)]
pub(crate) enum Commands {
    /// Synchronize wallet with the Ark network
    Sync {},

    /// Generate a new on-chain deposit address
    ReceiveOnchain {},

    /// Send on-chain to a bitcoin address
    PayOnchain {
        /// The Bitcoin address to send to
        #[arg(short = 'a', long = "address")]
        address: String,

        /// The amount to send in satoshis
        #[arg(short = 'm', long = "amount")]
        amount: u64,
    },

    /// Get your wallet balance
    GetBalance {},

    /// List payments
    ListPayments {
        /// Number of payments to show
        #[arg(short, long, default_value = "10")]
        limit: u32,

        /// Number of payments to skip
        #[arg(short, long, default_value = "0")]
        offset: u32,
    },

    /// Send payment to a destination (Ark address, BOLT11 invoice, etc.)
    SendPayment {
        /// The destination to send to (Ark address, BOLT11 invoice, etc.)
        #[arg(short, long)]
        destination: String,

        /// The amount to send in satoshis
        #[arg(short, long)]
        amount: u64,
    },

    /// Generate a payment destination (Ark address, Bitcoin address, etc.)
    ReceivePayment {
        /// The payment method to use (ark, bitcoin, bolt11)
        #[arg(short, long)]
        method: String,

        /// Optional amount in satoshis
        #[arg(short, long)]
        amount: Option<u64>,
    },

    /// Exit the interactive shell (interactive mode only)
    #[command(hide = true)]
    Exit {},
}

#[derive(Helper, Completer, Hinter, Validator)]
pub(crate) struct CliHelper {
    #[rustyline(Hinter)]
    pub(crate) hinter: HistoryHinter,
}

impl Highlighter for CliHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }
}

#[derive(Serialize)]
pub(crate) struct CommandResult<T: Serialize> {
    pub success: bool,
    pub message: T,
}

macro_rules! command_result {
    ($expr:expr) => {{
        to_string_pretty(&CommandResult {
            success: true,
            message: $expr,
        })?
    }};
}

macro_rules! wait_confirmation {
    ($prompt:expr,$result:expr) => {
        print!("{}", $prompt);
        std::io::stdout().flush()?;

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if !['y', 'Y'].contains(&(buf.as_bytes()[0] as char)) {
            return Ok(command_result!($result));
        }
    };
}

pub(crate) async fn execute_command(
    command: Commands,
    sdk: &BreezSdk,
) -> Result<String, anyhow::Error> {
    Ok(match command {
        Commands::Sync {} => {
            sdk.sync_wallet(SyncWalletRequest {}).await?;
            println!("Wallet synchronized successfully");
            command_result!("Wallet synchronized successfully")
        }
        Commands::ReceiveOnchain {} => {
            let response = sdk.receive_onchain(ReceiveOnchainRequest {}).await?;
            command_result!(response)
        }
        Commands::PayOnchain { address, amount } => {
            // First, prepare the transaction to get fee information
            let prepare_response = sdk
                .prepare_send_onchain(PrepareSendOnchainRequest {
                    receiver_amount_sats: amount,
                })
                .await?;

            wait_confirmation!(
                format!(
                    "Preparing to send {} sats to {}\nFee: {} sats\nTotal amount (including fees): {} sats\nDo you want to proceed? (y/n): ",
                    amount,
                    address,
                    prepare_response.fee_sats,
                    amount + prepare_response.fee_sats
                ),
                "Aborting payment"
            );

            let response = sdk
                .send_onchain(SendOnchainRequest {
                    prepare_send_onchain_response: prepare_response,
                    onchain_address: address,
                })
                .await?;

            command_result!(response)
        }

        Commands::GetBalance {} => {
            let response = sdk.get_balance(GetBalanceRequest {}).await?;
            command_result!(response)
        }
        Commands::ListPayments { limit, offset } => {
            let request = ListPaymentsRequest { offset, limit };
            let response = sdk.list_payments(request).await?;

            println!("Recent payments:");
            println!(
                "{:<40} {:<10} {:<10} {:<12} {:<8} Date",
                "ID", "Type", "Status", "Amount (sats)", "Fee"
            );
            println!("{}", "-".repeat(100));

            for payment in response.payments {
                let date = chrono::DateTime::from_timestamp(payment.timestamp.try_into()?, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                println!(
                    "{:<40} {:<10} {:<10} {:<12} {:<8} {}",
                    &payment.id,
                    payment.payment_type.to_string(),
                    payment.status.to_string(),
                    payment.amount,
                    payment.fees,
                    date
                );
            }
            "".to_string()
        }
        Commands::SendPayment {
            destination,
            amount,
        } => {
            // First, prepare the payment to get fee information
            let prepare_response = sdk
                .prepare_send_payment(PrepareSendPaymentRequest {
                    destination: destination.clone(),
                    amount: Some(PayAmount::Specific {
                        receiver_amount_sat: amount,
                    }),
                })
                .await?;

            // Show the payment details and fees to the user
            println!("Preparing payment to: {}", destination);
            println!("Amount: {} sats", amount);

            if let Some(fees_sat) = prepare_response.fees_sat {
                println!("Fee: {} sats", fees_sat);
                println!("Total amount (including fees): {} sats", amount + fees_sat);
            } else {
                println!("Fee: 0");
            }

            // Prompt the user for confirmation
            wait_confirmation!(
                "Do you want to proceed with this payment? (y/n): ",
                "Payment cancelled by user."
            );

            // User confirmed, proceed with the payment
            let response = sdk
                .send_payment(SendPaymentRequest { prepare_response })
                .await?;

            command_result!(response)
        }
        Commands::ReceivePayment { method, amount } => {
            // Parse the payment method from the user input
            let payment_method = match method.to_lowercase().as_str() {
                "ark" => PaymentMethod::ArkAddress {
                    receiver_amount_sat: amount,
                },
                "bitcoin" => PaymentMethod::BitcoinAddress {
                    receiver_amount_sat: amount,
                },
                "bolt11" => {
                    if let Some(amt) = amount {
                        PaymentMethod::Bolt11Invoice {
                            receiver_amount_sat: amt,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Amount is required for BOLT11 invoices"));
                    }
                }
                "bolt12" => PaymentMethod::Bolt12Offer,
                _ => PaymentMethod::Bolt12Offer,
            };

            // Create the request
            let request = ReceivePaymentRequest { payment_method };

            // Call the SDK to generate the payment destination
            let response = sdk.receive_payment(request).await?;

            // Display the result to the user
            command_result!(response)
        }
        Commands::Exit {} => {
            command_result!("Exiting...")
        }
    })
}
