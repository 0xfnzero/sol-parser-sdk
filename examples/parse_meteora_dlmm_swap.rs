//! Parse a specific Meteora DLMM Swap transaction from RPC
//!
//! This example fetches a Meteora DLMM Swap transaction from RPC
//! and parses it using sol-parser-sdk's RPC parsing support.
//!
//! Transaction: 4bo3keYw6cNyKFBPWkyVKBRxx5HR9pqUX7oUCKfaCDHyEHUtcvWf83JMyw8aCw95miYpCibmvZ47yzEU2QgL1SpC
//!
//! Usage:
//! ```bash
//! cargo run --example parse_meteora_dlmm_swap --release
//! ```

use sol_parser_sdk::parse_transaction_from_rpc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

fn main() {
    // 交易签名
    let tx_sig =
        "4bo3keYw6cNyKFBPWkyVKBRxx5HR9pqUX7oUCKfaCDHyEHUtcvWf83JMyw8aCw95miYpCibmvZ47yzEU2QgL1SpC";

    println!("=== Meteora DLMM Swap Transaction Parser ===\n");
    println!("Transaction Signature: {}\n", tx_sig);

    // 连接到 Solana RPC
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://solana-rpc.publicnode.com".to_string());

    println!("Connecting to: {}", rpc_url);
    let client = RpcClient::new(rpc_url);

    // 解析签名
    let signature = Signature::from_str(tx_sig).expect("Failed to parse signature");

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
    } else {
        println!("=== Parsed Events (SDK Format) ===\n");
        for (i, event) in events.iter().enumerate() {
            println!("Event #{}: {:?}\n", i + 1, event);
        }
    }

    println!("\n=== Summary ===");
    println!("✓ sol-parser-sdk successfully parsed the transaction!");
    println!("  The new RPC parsing API supports:");
    println!("  - Direct parsing from RPC (no gRPC streaming needed)");
    println!("  - Inner instruction parsing (16-byte discriminators)");
    println!("  - All 10 DEX protocols");
    println!("  - Perfect for testing and validation");

    println!("\n✓ Example completed!");
}
