//! Debug PumpSwap transaction parsing

use base64::Engine as _;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

fn main() {
    let tx_sig =
        "3zsihbygW7hoKGtduAyDDFzp4E1eis8gaBzEzzNKr8ma39baffpFcphok9wHFgR3EauDe9vYYsVf4Puh5pZ6UJiS";

    println!("=== Debug PumpSwap Transaction ===\n");
    println!("Signature: {}\n", tx_sig);

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://solana-rpc.publicnode.com".to_string());

    println!("RPC: {}\n", rpc_url);
    let client = RpcClient::new(rpc_url);

    let signature = Signature::from_str(tx_sig).unwrap();

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: None,
        max_supported_transaction_version: Some(0),
    };

    println!("Fetching transaction...\n");
    let rpc_tx = match client.get_transaction_with_config(&signature, config) {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to fetch: {}", e);
            return;
        }
    };

    println!("Transaction fetched successfully!");
    println!("Slot: {}", rpc_tx.slot);
    println!("Block time: {:?}", rpc_tx.block_time);

    // Print meta info
    if let Some(ref meta) = rpc_tx.transaction.meta {
        println!("\n=== Transaction Meta ===");
        println!("Fee: {}", meta.fee);
        println!("Compute units: {:?}", meta.compute_units_consumed);

        // Check loaded addresses
        println!("\n=== Loaded Addresses ===");
        let loaded_opt: Option<solana_transaction_status::UiLoadedAddresses> =
            meta.loaded_addresses.clone().into();
        if let Some(ref loaded) = loaded_opt {
            println!("Writable: {} addresses", loaded.writable.len());
            for (i, addr) in loaded.writable.iter().enumerate() {
                println!("  [{}]: {}", i, addr);
            }
            println!("Readonly: {} addresses", loaded.readonly.len());
            for (i, addr) in loaded.readonly.iter().enumerate() {
                println!("  [{}]: {}", i, addr);
            }
        } else {
            println!("⚠ No loaded_addresses field!");
        }

        // Print log messages
        let logs: Option<Vec<String>> = meta.log_messages.clone().into();
        if let Some(logs) = logs {
            println!("\n=== Logs ({} lines) ===", logs.len());
            for (i, log) in logs.iter().enumerate() {
                if log.contains("Program 6EF8")
                    || log.contains("Program pAMM")
                    || log.contains("invoke")
                    || log.contains("success")
                {
                    println!("{}: {}", i, log);
                }
            }
        }

        // Print inner instructions
        let inner_instructions: Option<Vec<_>> = meta.inner_instructions.clone().into();
        if let Some(inner) = inner_instructions {
            println!("\n=== Inner Instructions ({} groups) ===", inner.len());

            // Get account keys from transaction for program ID lookup
            let account_keys: Vec<solana_sdk::pubkey::Pubkey> =
                if let solana_transaction_status::EncodedTransaction::Binary(data, _) =
                    &rpc_tx.transaction.transaction
                {
                    let bytes = base64::engine::general_purpose::STANDARD.decode(data).unwrap();
                    let versioned_tx: solana_sdk::transaction::VersionedTransaction =
                        bincode::deserialize(&bytes).unwrap();
                    match &versioned_tx.message {
                        solana_sdk::message::VersionedMessage::Legacy(m) => m.account_keys.clone(),
                        solana_sdk::message::VersionedMessage::V0(m) => m.account_keys.clone(),
                    }
                } else {
                    vec![]
                };

            for inner_group in inner.iter() {
                println!(
                    "\nOuter instruction #{}, {} inner instructions:",
                    inner_group.index,
                    inner_group.instructions.len()
                );

                for (i, ix) in inner_group.instructions.iter().enumerate() {
                    if let solana_transaction_status::UiInstruction::Compiled(compiled) = ix {
                        let data_bytes =
                            bs58::decode(&compiled.data).into_vec().unwrap_or_default();

                        // Get program ID
                        let program_id =
                            if (compiled.program_id_index as usize) < account_keys.len() {
                                Some(&account_keys[compiled.program_id_index as usize])
                            } else {
                                None
                            };

                        println!(
                            "  Inner #{}: program_id_index={}, accounts={:?}, data_len={}",
                            i,
                            compiled.program_id_index,
                            compiled.accounts,
                            data_bytes.len()
                        );

                        if let Some(pid) = program_id {
                            println!("    Program ID: {}", pid);

                            // Check if it's PumpSwap
                            if pid.to_string() == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" {
                                println!("    ✓ This is PumpSwap!");
                            }
                        }

                        if data_bytes.len() >= 16 {
                            let disc: [u8; 16] = data_bytes[..16].try_into().unwrap();
                            println!("    Discriminator (16 bytes): {:?}", disc);
                        } else if data_bytes.len() >= 8 {
                            let disc: [u8; 8] = data_bytes[..8].try_into().unwrap();
                            println!("    Discriminator (8 bytes): {:?}", disc);
                        }
                    }
                }
            }
        } else {
            println!("\n⚠ No inner instructions found!");
        }
    }

    // Print main instructions
    println!("\n=== Main Instructions ===");
    if let solana_transaction_status::EncodedTransaction::Binary(data, _) =
        &rpc_tx.transaction.transaction
    {
        let bytes = base64::engine::general_purpose::STANDARD.decode(data).unwrap();
        let versioned_tx: solana_sdk::transaction::VersionedTransaction =
            bincode::deserialize(&bytes).unwrap();

        match &versioned_tx.message {
            solana_sdk::message::VersionedMessage::Legacy(m) => {
                println!("Message type: Legacy");
                println!("Account keys: {}", m.account_keys.len());
                for (i, key) in m.account_keys.iter().enumerate() {
                    println!("  [{}]: {}", i, key);
                }
                println!("\nInstructions: {}", m.instructions.len());
                for (i, ix) in m.instructions.iter().enumerate() {
                    println!(
                        "  Instruction #{}: program_id_index={}, data_len={}",
                        i,
                        ix.program_id_index,
                        ix.data.len()
                    );
                    let program_key = &m.account_keys[ix.program_id_index as usize];
                    println!("    Program: {}", program_key);

                    if ix.data.len() >= 8 {
                        let disc: [u8; 8] = ix.data[..8].try_into().unwrap();
                        println!("    Discriminator: {:?}", disc);
                    }
                }
            }
            solana_sdk::message::VersionedMessage::V0(m) => {
                println!("Message type: V0");
                println!("Account keys: {}", m.account_keys.len());
                for (i, key) in m.account_keys.iter().enumerate() {
                    println!("  [{}]: {}", i, key);
                }
                println!("\nInstructions: {}", m.instructions.len());
                for (i, ix) in m.instructions.iter().enumerate() {
                    println!(
                        "  Instruction #{}: program_id_index={}, data_len={}",
                        i,
                        ix.program_id_index,
                        ix.data.len()
                    );
                    if (ix.program_id_index as usize) < m.account_keys.len() {
                        let program_key = &m.account_keys[ix.program_id_index as usize];
                        println!("    Program: {}", program_key);
                    }

                    if ix.data.len() >= 8 {
                        let disc: [u8; 8] = ix.data[..8].try_into().unwrap();
                        println!("    Discriminator: {:?}", disc);
                    }
                }
            }
        };
    }

    println!("\n=== Now checking sol-parser-sdk parsing ===");

    // First, check the converted gRPC format
    match sol_parser_sdk::parse_rpc_transaction(&rpc_tx, None) {
        Ok(events) => {
            println!("✓ Parsed {} events", events.len());
            if events.is_empty() {
                println!("\n⚠ No events parsed!");
                println!("Let me check what was converted...");

                // Manually parse to see what's happening
                if let Ok((grpc_meta, grpc_tx)) = sol_parser_sdk::convert_rpc_to_grpc(&rpc_tx) {
                    println!("\n=== Converted gRPC Format ===");
                    println!(
                        "Loaded writable addresses: {}",
                        grpc_meta.loaded_writable_addresses.len()
                    );
                    println!(
                        "Loaded readonly addresses: {}",
                        grpc_meta.loaded_readonly_addresses.len()
                    );
                    println!("Inner instructions groups: {}", grpc_meta.inner_instructions.len());

                    if let Some(msg) = &grpc_tx.message {
                        println!("Account keys in message: {}", msg.account_keys.len());
                        println!("Main instructions: {}", msg.instructions.len());

                        for (i, ix) in msg.instructions.iter().enumerate() {
                            println!(
                                "\nMain instruction #{}: program_id_index={}, data_len={}",
                                i,
                                ix.program_id_index,
                                ix.data.len()
                            );
                        }

                        for (idx, inner_group) in grpc_meta.inner_instructions.iter().enumerate() {
                            println!(
                                "\n=== Inner Group #{} (outer_idx={}) ===",
                                idx, inner_group.index
                            );
                            println!("  {} inner instructions", inner_group.instructions.len());

                            for (i, inner_ix) in inner_group.instructions.iter().enumerate() {
                                println!(
                                    "  Inner #{}: program_id_index={}, data_len={}",
                                    i,
                                    inner_ix.program_id_index,
                                    inner_ix.data.len()
                                );

                                // Try to get the program ID
                                let total_keys = msg.account_keys.len()
                                    + grpc_meta.loaded_writable_addresses.len()
                                    + grpc_meta.loaded_readonly_addresses.len();

                                println!("    Total available keys: {}", total_keys);

                                if (inner_ix.program_id_index as usize) < total_keys {
                                    println!("    ✓ Program ID index is valid");
                                } else {
                                    println!(
                                        "    ✗ Program ID index {} exceeds total keys {}",
                                        inner_ix.program_id_index, total_keys
                                    );
                                }

                                if inner_ix.data.len() >= 16 {
                                    let disc: [u8; 16] = inner_ix.data[..16].try_into().unwrap();
                                    println!("    Discriminator: {:?}", disc);
                                }
                            }
                        }
                    }
                }
            } else {
                println!("\n=== Parsed Events (SDK Format) ===\n");
                for (i, event) in events.iter().enumerate() {
                    println!("Event #{}: {:?}\n", i + 1, event);
                }
            }
        }
        Err(e) => {
            println!("✗ Parse error: {}", e);
        }
    }
}

/// Helper function to expose conversion for debugging.
#[allow(dead_code)]
fn convert_rpc_to_grpc_debug(
    rpc_tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
) -> Result<
    (
        yellowstone_grpc_proto::prelude::TransactionStatusMeta,
        yellowstone_grpc_proto::prelude::Transaction,
    ),
    Box<dyn std::error::Error>,
> {
    use yellowstone_grpc_proto::prelude::*;

    let _rpc_meta = rpc_tx.transaction.meta.as_ref().ok_or("No meta")?;

    // This is a simplified version - just for debugging
    Ok((TransactionStatusMeta::default(), Transaction::default()))
}
