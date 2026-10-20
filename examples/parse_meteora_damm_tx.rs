//! Parse a specific Meteora DAMM transaction from RPC
//!
//! This example fetches a Meteora DAMM (Dynamic AMM) transaction from RPC
//! and parses it using sol-parser-sdk's RPC parsing support.
//!
//! Usage:
//! ```bash
//! # Provide transaction signature via environment variable
//! TX_SIGNATURE=<your_tx_sig> cargo run --example parse_meteora_damm_tx --release
//!
//! # Optional: Use custom RPC endpoint
//! SOLANA_RPC_URL=https://your-rpc.com TX_SIGNATURE=<your_tx_sig> cargo run --example parse_meteora_damm_tx --release
//! ```

use sol_parser_sdk::{parse_transaction_from_rpc, DexEvent};
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

fn main() {
    // 交易签名 - 通过环境变量提供
    let tx_sig = std::env::var("TX_SIGNATURE")
        .unwrap_or_else(|_| {
            eprintln!("❌ Error: Please provide a Meteora DAMM transaction signature\n");
            eprintln!("Usage:");
            eprintln!("  TX_SIGNATURE=<your_tx_sig> cargo run --example parse_meteora_damm_tx --release\n");
            eprintln!("Example:");
            eprintln!("  TX_SIGNATURE=5curEt85cQhAK6R9pntSJ4fmYCiPEG22NjZyGrnGSbNwAkHJMN25T9Efp1n9Tf9vGXhnDXMQYrCNpoRHQTMcZ1s9 \\");
            eprintln!("    cargo run --example parse_meteora_damm_tx --release\n");
            std::process::exit(1);
        });

    println!("=== Meteora DAMM Transaction Parser ===\n");
    println!("Transaction Signature: {}\n", tx_sig);

    // 连接到 Solana RPC
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://solana-rpc.publicnode.com".to_string());

    println!("Using configured RPC endpoint");
    let client = RpcClient::new(rpc_url);

    // 解析签名
    let signature = Signature::from_str(&tx_sig).expect("Failed to parse signature");

    // 使用 sol-parser-sdk 直接解析交易
    println!("\n=== Parsing with sol-parser-sdk ===");
    println!("Fetching and parsing transaction...\n");

    let events = match parse_transaction_from_rpc(&client, &signature, None) {
        Ok(events) => events,
        Err(e) => {
            eprintln!("✗ Failed to parse transaction: {}", e);
            eprintln!("\nNote: You might need to use a different RPC endpoint.");
            eprintln!("Set SOLANA_RPC_URL environment variable to use a custom endpoint.");
            eprintln!("Example: export SOLANA_RPC_URL=https://your-rpc-endpoint.com");
            std::process::exit(1);
        }
    };

    println!("✓ Parsing completed!");
    println!("  Found {} DEX events\n", events.len());

    // 显示解析结果（完整事件数据格式，与 gRPC 解析结果一致）
    if events.is_empty() {
        println!("⚠ No DEX events found in this transaction.");
        println!("  This might not be a Meteora DAMM transaction.");
    } else {
        println!("=== Parsed Events ===\n");
        for (i, event) in events.iter().enumerate() {
            match event {
                DexEvent::MeteoraDammV2Swap(e) => {
                    println!("┌─────────────────────────────────────────────────────────────");
                    println!("│ Event #{}: 🔄 Meteora DAMM SWAP (V2)", i + 1);
                    println!("├─────────────────────────────────────────────────────────────");
                    println!("│ Pool       : {}", e.pool);
                    println!(
                        "│ Direction  : {}",
                        if e.trade_direction == 0 { "A→B" } else { "B→A" }
                    );
                    println!("│ Amount In  : {}", e.amount_in);
                    println!("│ Min Out    : {}", e.minimum_amount_out);
                    println!("│ Actual Out : {}", e.output_amount);
                    println!("│ Actual In  : {}", e.actual_amount_in);
                    println!("│ LP Fee     : {}", e.lp_fee);
                    println!("│ Protocol   : {}", e.protocol_fee);
                    println!(
                        "│ Referral   : {} (has_referral: {})",
                        e.referral_fee, e.has_referral
                    );
                    println!("│ Sqrt Price : {}", e.next_sqrt_price);
                    println!("└─────────────────────────────────────────────────────────────\n");
                }
                DexEvent::MeteoraDammV2AddLiquidity(e) => {
                    println!("┌─────────────────────────────────────────────────────────────");
                    println!("│ Event #{}: ➕ Meteora DAMM ADD LIQUIDITY (V2)", i + 1);
                    println!("├─────────────────────────────────────────────────────────────");
                    println!("│ Pool       : {}", e.pool);
                    println!("│ Position   : {}", e.position);
                    println!("│ Token A In : {}", e.token_a_amount);
                    println!("│ Token B In : {}", e.token_b_amount);
                    println!("└─────────────────────────────────────────────────────────────\n");
                }
                DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
                    println!("┌─────────────────────────────────────────────────────────────");
                    println!("│ Event #{}: ➖ Meteora DAMM REMOVE LIQUIDITY (V2)", i + 1);
                    println!("├─────────────────────────────────────────────────────────────");
                    println!("│ Pool       : {}", e.pool);
                    println!("│ Position   : {}", e.position);
                    println!("│ Token A Out: {}", e.token_a_amount);
                    println!("│ Token B Out: {}", e.token_b_amount);
                    println!("└─────────────────────────────────────────────────────────────\n");
                }
                DexEvent::MeteoraDammV2CreatePosition(e) => {
                    println!("┌─────────────────────────────────────────────────────────────");
                    println!("│ Event #{}: 📍 Meteora DAMM CREATE POSITION (V2)", i + 1);
                    println!("├─────────────────────────────────────────────────────────────");
                    println!("│ Pool       : {}", e.pool);
                    println!("│ Position   : {}", e.position);
                    println!("│ Owner      : {}", e.owner);
                    println!("└─────────────────────────────────────────────────────────────\n");
                }
                DexEvent::MeteoraDammV2ClosePosition(e) => {
                    println!("┌─────────────────────────────────────────────────────────────");
                    println!("│ Event #{}: 📍 Meteora DAMM CLOSE POSITION (V2)", i + 1);
                    println!("├─────────────────────────────────────────────────────────────");
                    println!("│ Pool       : {}", e.pool);
                    println!("│ Position   : {}", e.position);
                    println!("│ Owner      : {}", e.owner);
                    println!("└─────────────────────────────────────────────────────────────\n");
                }
                _ => {
                    println!("Event #{}: {:?}\n", i + 1, event);
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!("✓ sol-parser-sdk successfully parsed the transaction!");
    println!("  The new RPC parsing API supports:");
    println!("  - Direct parsing from RPC (no gRPC streaming needed)");
    println!("  - Inner instruction parsing (16-byte discriminators)");
    println!("  - All 10 DEX protocols (including Meteora DAMM)");
    println!("  - Meteora DAMM V2 events (Swap, AddLiquidity, RemoveLiquidity, CreatePosition, ClosePosition)");
    println!("  - Perfect for testing and validation");

    println!("\n✓ Example completed!");
}
