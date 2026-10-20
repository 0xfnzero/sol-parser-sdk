//! Token mint (decimals/supply) subscription example
//!
//! Subscribe to token mint account updates to get decimals and supply. Uses EventType::TokenAccount
//! (mint accounts are parsed as TokenInfo when they match token program owner).
//!
//! Run: `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release`

use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mint_account = std::env::var("MINT_ACCOUNT").unwrap_or_else(|_| {
        eprintln!(
            "Usage: MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release"
        );
        std::process::exit(1);
    });

    println!("=== Token Mint (Decimals) Listen ===\n");
    println!("Mint account: {}\n", mint_account);

    let config = ClientConfig::default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        config,
    )?;

    let transaction_filter = TransactionFilter::default();
    let account_filter =
        AccountFilter { account: vec![mint_account.clone()], owner: vec![], filters: vec![] };
    let event_filter = EventTypeFilter::include_only(vec![EventType::TokenAccount]);

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    println!("Listening for mint (TokenInfo) updates. Press Ctrl+C to stop.\n");

    let queue_clone = queue.clone();
    tokio::spawn(async move {
        loop {
            if let Some(event) = queue_clone.pop() {
                if let DexEvent::TokenInfo(e) = event {
                    println!(
                        "TokenInfo pubkey={} decimals={} supply={}",
                        e.pubkey, e.decimals, e.supply
                    );
                }
            } else {
                tokio::task::yield_now().await;
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    println!("Stopped.");
    Ok(())
}
