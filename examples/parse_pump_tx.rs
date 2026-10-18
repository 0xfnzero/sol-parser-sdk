//! Parse a specific PumpFun transaction from RPC
//!
//! This example fetches a PumpFun transaction from RPC
//! and parses it using sol-parser-sdk's RPC parsing support.
//!
//! If you see "Transaction not found (RPC returned null)": the tx may be too old.
//! Many public RPCs only keep recent blocks; use an archive RPC (e.g. Helius, QuickNode)
//! or set SOLANA_RPC_URL to one.
//!
//! Usage:
//! ```bash
//! cargo run --example parse_pump_tx --release
//! # Custom signature (optional):
//! TX_SIGNATURE=your_tx_sig cargo run --example parse_pump_tx --release
//! # Custom RPC (optional):
//! SOLANA_RPC_URL=https://api.mainnet-beta.solana.com cargo run --example parse_pump_tx --release
//! ```

use sol_parser_sdk::parse_transaction_from_rpc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

fn main() {
    // 交易签名：优先从环境变量 TX_SIGNATURE 读取，否则用默认示例
    let default_sig =
        "4ARos3Yq1SMNE3izDpy9YdKrRgMmD7UCDxvnVrBRaGCJHiifPHSmQWc4DPfjUaEUcxS7puGmEN64K7EZdeXLJaj2";
    let tx_sig = std::env::var("TX_SIGNATURE").unwrap_or_else(|_| default_sig.to_string());

    println!("=== PumpFun Transaction Parser ===\n");
    println!("Transaction Signature: {}\n", tx_sig);

    // 连接到 Solana RPC（默认使用官方 mainnet 公开 RPC）
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    println!("Connecting to: {}", rpc_url);
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
            eprintln!("\nNote: If the error says 'Transaction not found (RPC returned null)', the tx may be pruned.");
            eprintln!("Use an archive RPC (e.g. Helius, QuickNode) or set SOLANA_RPC_URL.");
            eprintln!(
                "Example: export SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
            );
            std::process::exit(1);
        }
    };

    println!("✓ Parsing completed!");
    println!("  Found {} DEX events\n", events.len());

    // 显示解析结果（完整事件数据格式，与 gRPC 解析结果一致）
    if events.is_empty() {
        println!("⚠ No DEX events found in this transaction.");
    } else {
        println!("=== Parsed Events (SDK Format) ===\n");
        for (i, event) in events.iter().enumerate() {
            println!("Event #{}:\n{:#?}\n", i + 1, event);
        }
    }

    println!("\n=== Summary ===");
    println!("✓ sol-parser-sdk successfully parsed the transaction!");
    println!("  The new RPC parsing API supports:");
    println!("  - Direct parsing from RPC (no gRPC streaming needed)");
    println!("  - Inner instruction parsing (16-byte discriminators)");
    println!("  - All 10 DEX protocols (including PumpFun)");
    println!("  - Perfect for testing and validation");

    println!("\n✓ Example completed!");
}
