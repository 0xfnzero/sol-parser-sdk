//! Test account filling for PumpSwap transactions
//!
//! This example debugs the account filling process to understand
//! why accounts are showing as default Pubkeys.

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

fn main() {
    let tx_sig =
        "3zsihbygW7hoKGtduAyDDFzp4E1eis8gaBzEzzNKr8ma39baffpFcphok9wHFgR3EauDe9vYYsVf4Puh5pZ6UJiS";

    println!("=== Account Filling Debug ===\n");
    println!("Signature: {}\n", tx_sig);

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://solana-rpc.publicnode.com".to_string());

    println!("RPC: {}\n", rpc_url);
    let client = RpcClient::new(rpc_url);

    let signature = Signature::from_str(tx_sig).unwrap();

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: None,
        max_supported_transaction_version: Some(1),
    };

    println!("Fetching transaction...\n");
    let rpc_tx = match client.get_transaction_with_config(&signature, config) {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to fetch: {}", e);
            return;
        }
    };

    println!("✓ Transaction fetched\n");

    // Check converted gRPC format
    println!("=== Converting RPC to gRPC format ===\n");

    match sol_parser_sdk::convert_rpc_to_grpc(&rpc_tx) {
        Ok((grpc_meta, grpc_tx)) => {
            println!("✓ Conversion successful\n");

            // Print account keys
            if let Some(ref msg) = grpc_tx.message {
                println!("=== Message Account Keys ({}) ===", msg.account_keys.len());
                for (i, key_bytes) in msg.account_keys.iter().enumerate() {
                    if key_bytes.len() == 32 {
                        let pk = solana_sdk::pubkey::Pubkey::try_from(&key_bytes[..]).unwrap();
                        println!("  [{}]: {}", i, pk);
                    }
                }
                println!();
            }

            // Print loaded addresses
            println!("=== Loaded Addresses ===");
            println!("Writable: {}", grpc_meta.loaded_writable_addresses.len());
            for (i, addr_bytes) in grpc_meta.loaded_writable_addresses.iter().enumerate() {
                if addr_bytes.len() == 32 {
                    let pk = solana_sdk::pubkey::Pubkey::try_from(&addr_bytes[..]).unwrap();
                    println!("  [{}]: {}", i, pk);
                }
            }
            println!("Readonly: {}", grpc_meta.loaded_readonly_addresses.len());
            for (i, addr_bytes) in grpc_meta.loaded_readonly_addresses.iter().enumerate() {
                if addr_bytes.len() == 32 {
                    let pk = solana_sdk::pubkey::Pubkey::try_from(&addr_bytes[..]).unwrap();
                    println!("  [{}]: {}", i, pk);
                }
            }
            println!();

            // Print inner instructions with account arrays
            println!("=== Inner Instructions ({}) ===", grpc_meta.inner_instructions.len());
            for (group_idx, inner_group) in grpc_meta.inner_instructions.iter().enumerate() {
                println!("\nGroup #{} (outer instruction #{})", group_idx, inner_group.index);
                println!("  {} inner instructions", inner_group.instructions.len());

                for (inner_idx, inner_ix) in inner_group.instructions.iter().enumerate() {
                    println!("\n  Inner instruction #{}", inner_idx);
                    println!("    program_id_index: {}", inner_ix.program_id_index);
                    println!("    data_len: {}", inner_ix.data.len());
                    println!("    accounts: {:?}", inner_ix.accounts);

                    // Check discriminator
                    if inner_ix.data.len() >= 16 {
                        let disc: [u8; 16] = inner_ix.data[..16].try_into().unwrap();
                        println!("    discriminator: {:?}", disc);

                        // Check if it's PumpSwap Sell
                        const PUMPSWAP_SELL: [u8; 16] =
                            [228, 69, 165, 46, 81, 203, 154, 29, 62, 47, 55, 10, 165, 3, 220, 42];

                        if disc == PUMPSWAP_SELL {
                            println!("    ✓ This is PumpSwap Sell event!");
                            println!("\n    === Testing account resolution ===");

                            // Total available accounts
                            let total_keys = if let Some(ref msg) = grpc_tx.message {
                                msg.account_keys.len()
                            } else {
                                0
                            };
                            let total_accounts = total_keys
                                + grpc_meta.loaded_writable_addresses.len()
                                + grpc_meta.loaded_readonly_addresses.len();

                            println!("    Total accounts available: {}", total_accounts);

                            // Try to resolve key accounts
                            let account_names = vec![
                                (0, "pool"),
                                (1, "user"),
                                (3, "base_mint"),
                                (4, "quote_mint"),
                                (7, "pool_base_token_account"),
                                (8, "pool_quote_token_account"),
                                (11, "base_token_program"),
                                (12, "quote_token_program"),
                            ];

                            for (acc_idx, name) in account_names {
                                if let Some(&tx_acc_idx) = inner_ix.accounts.get(acc_idx) {
                                    println!(
                                        "    {} (accounts[{}] = {})",
                                        name, acc_idx, tx_acc_idx
                                    );

                                    // Try to resolve
                                    if let Some(ref msg) = grpc_tx.message {
                                        if (tx_acc_idx as usize) < msg.account_keys.len() {
                                            let key_bytes = &msg.account_keys[tx_acc_idx as usize];
                                            if key_bytes.len() == 32 {
                                                let pk = solana_sdk::pubkey::Pubkey::try_from(
                                                    &key_bytes[..],
                                                )
                                                .unwrap();
                                                println!("      → Resolved to: {}", pk);
                                            }
                                        } else {
                                            let offset =
                                                (tx_acc_idx as usize) - msg.account_keys.len();
                                            if offset < grpc_meta.loaded_writable_addresses.len() {
                                                let key_bytes =
                                                    &grpc_meta.loaded_writable_addresses[offset];
                                                if key_bytes.len() == 32 {
                                                    let pk = solana_sdk::pubkey::Pubkey::try_from(
                                                        &key_bytes[..],
                                                    )
                                                    .unwrap();
                                                    println!("      → Resolved (writable): {}", pk);
                                                }
                                            } else {
                                                let ro_offset = offset
                                                    - grpc_meta.loaded_writable_addresses.len();
                                                if ro_offset
                                                    < grpc_meta.loaded_readonly_addresses.len()
                                                {
                                                    let key_bytes = &grpc_meta
                                                        .loaded_readonly_addresses[ro_offset];
                                                    if key_bytes.len() == 32 {
                                                        let pk =
                                                            solana_sdk::pubkey::Pubkey::try_from(
                                                                &key_bytes[..],
                                                            )
                                                            .unwrap();
                                                        println!(
                                                            "      → Resolved (readonly): {}",
                                                            pk
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    println!(
                                        "    {} (accounts[{}]) - NOT IN ARRAY!",
                                        name, acc_idx
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Now try parsing
            println!("\n\n=== Parsing with sol-parser-sdk ===\n");
            match sol_parser_sdk::parse_rpc_transaction(&rpc_tx, None) {
                Ok(events) => {
                    println!("✓ Parsed {} events\n", events.len());
                    for (i, event) in events.iter().enumerate() {
                        println!("Event #{}: {:?}\n", i + 1, event);
                    }
                }
                Err(e) => {
                    println!("✗ Parse error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Conversion error: {}", e);
        }
    }
}
