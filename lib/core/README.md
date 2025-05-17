# Breez SDK Ark Implementation

This is an implementation of the Breez SDK using the Ark client for Bitcoin and Lightning Network functionality.

## Overview

The Breez SDK Ark implementation provides a Rust-based SDK for integrating Bitcoin and Lightning Network functionality into applications using the Ark client. It offers a simple, high-level API for common operations such as:

- Sending and receiving on-chain Bitcoin
- Sending and receiving Lightning payments
- Creating and paying Lightning invoices
- Managing wallet balances and transactions

## Project Structure

- `src/lib.rs` - Main SDK interface with method implementations
- `src/models.rs` - Data models and request/response structures
- `src/error.rs` - Error types and conversions
- `src/persist/` - Storage implementations for persistent data
  - `mod.rs` - Storage trait definition
  - `sqlite.rs` - SQLite implementation of the Storage trait
- `src/chain/` - Blockchain service implementations
  - `mod.rs` - ChainService trait definition
- `src/events.rs` - Event handling functionality
- `src/sdk_builder.rs` - Builder pattern for SDK initialization
- `src/logger.rs` - Logging functionality

## Getting Started

### Prerequisites

- Rust 1.60 or higher
- SQLite development libraries

### Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
breez-sdk-ark = { git = "https://github.com/breez/breez-sdk-ark" }
```

### Basic Usage

```rust
use breez_sdk_ark::{BreezSdk, SdkBuilder, models::Config, models::Network};
use std::path::PathBuf;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SDK
    let config = Config {
        network: Network::Bitcoin,
        ark_server_url: "https://ark-server.example.com".to_string(),
        mnemonic: "your mnemonic here".to_string(),
        seed: None,
        api_key: None,
    };
    
    let sdk = SdkBuilder::new()
        .config(config)
        .storage_path(PathBuf::from("./data"))
        .build()
        .await?;
    
    // Get wallet balance
    let balance = sdk.get_balance(Default::default())?;
    println!("Balance: {} sats", balance.balance_sats);
    
    // Sync wallet with network
    sdk.sync_wallet(Default::default()).await?;
    
    Ok(())
}
```

## Development Status

This SDK is currently in development and not all features are fully implemented. The current implementation provides placeholder functionality for most methods, which will be replaced with actual Ark client integration in future updates.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
