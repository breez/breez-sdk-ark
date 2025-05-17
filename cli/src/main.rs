mod commands;
mod persist;

use crate::commands::CliHelper;
use crate::persist::CliPersistence;
use anyhow::anyhow;
use anyhow::Result;
use breez_sdk_ark::models::ConnectRequest;
use breez_sdk_ark::models::{Config, Network};
use breez_sdk_ark::{connect, BreezSdk, EventListener, SdkEvent};
use clap::Parser;
use commands::CommandResult;
use commands::{execute_command, Commands};
use log::{error, info};
use rustyline::error::ReadlineError;
use rustyline::hint::HistoryHinter;
use rustyline::Editor;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(version, about = "CLI client for Breez SDK with Ark", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to the data directory
    #[arg(short, long, default_value = "./.data")]
    data_dir: String,

    /// Network to use (bitcoin, testnet, signet, regtest)
    #[arg(long, default_value = "regtest")]
    network: String,
}

fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(stripped)
    } else {
        PathBuf::from(path)
    }
}

/// Parse a command string into a Commands enum using clap
fn parse_command(input: &str) -> Result<Commands> {
    // Handle exit command specially since it's not exposed in non-interactive mode
    if input.trim() == "exit" || input.trim() == "quit" {
        return Ok(Commands::Exit {});
    }

    // Create args for clap by adding program name at the beginning
    let mut args = vec!["breez-cli".to_string()];
    args.extend(shlex::split(input).ok_or_else(|| anyhow!("Failed to parse command"))?);

    // Parse the command using clap
    let cmd = Commands::try_parse_from(&args)?;
    Ok(cmd)
}

struct CliEventListener {}

impl EventListener for CliEventListener {
    fn on_event(&self, event: &SdkEvent) {
        info!("Event received: {:?}", event);
    }
}

async fn run_interactive_mode(data_dir: PathBuf, network: Network) -> Result<()> {
    // Create data directory if it doesn't exist
    fs::create_dir_all(&data_dir)?;

    // Initialize persistence
    let persistence = CliPersistence {
        data_dir: data_dir.clone(),
    };

    // Get or create mnemonic
    let mnemonic = persistence.get_or_create_mnemonic()?;
    println!("Using mnemonic: {}", mnemonic);

    // Create SDK configuration
    let config = Config::default_config(network, data_dir.to_string_lossy().to_string())?;

    // Initialize logging
    BreezSdk::init_logging(&data_dir.to_string_lossy(), None)?;

    // Connect to the SDK
    let sdk = connect(ConnectRequest {
        config,
        mnemonic: mnemonic.to_string(),
    })
    .await?;

    // Register event listener
    let _listener_id = sdk.add_event_listener(Box::new(CliEventListener {}));

    // Initialize rustyline
    let helper = CliHelper {
        hinter: HistoryHinter {},
    };
    let mut rl = Editor::new()?;
    rl.set_helper(Some(helper));

    // Load history from file
    let history_file = persistence.history_file();
    if rl.load_history(&history_file).is_err() {
        error!("Failed to load history");
    }

    // Display welcome message
    println!("Welcome to the Breez SDK Ark CLI!");
    println!("Type 'help' to see available commands or 'exit' to quit.");

    // Main loop
    loop {
        let readline = rl.readline("ark> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                // Parse and execute command
                match parse_command(line) {
                    Ok(cmd) => {
                        if let Commands::Exit {} = cmd {
                            break;
                        }

                        let res = execute_command(cmd, &sdk).await;
                        show_results(res)?;
                    }
                    Err(e) => println!("Error parsing command: {}", e),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Err(err) = rl.save_history(&history_file) {
        error!("Failed to save history: {}", err);
    }

    // Stop the SDK
    sdk.disconnect()?;

    Ok(())
}

fn show_results(result: Result<String>) -> Result<()> {
    let result_str = match result {
        Ok(r) => r,
        Err(err) => serde_json::to_string_pretty(&CommandResult {
            success: false,
            message: err.to_string(),
        })?,
    };

    println!("{result_str}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Expand data directory path
    let data_dir = expand_path(&cli.data_dir);

    // Parse network
    let network = match cli.network.to_lowercase().as_str() {
        "bitcoin" | "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => return Err(anyhow!("Invalid network: {}", cli.network)),
    };

    // Run in interactive mode
    run_interactive_mode(data_dir, network).await
}
