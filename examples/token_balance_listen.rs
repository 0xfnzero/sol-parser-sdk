//! Token account balance subscription example
//!
//! Subscribe to a specific token account's updates via Yellowstone gRPC account filter.
//! Uses sol-parser-sdk's queue-based API: subscribe_dex_events returns a queue, pop events and match DexEvent::TokenAccount.
//!
//! Run: `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release`
//! Example: TOKEN_ACCOUNT=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v cargo run --example token_balance_listen --release

use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let account_to_listen = std::env::var("TOKEN_ACCOUNT").unwrap_or_else(|_| {
        eprintln!(
            "Usage: TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release"
        );
        std::process::exit(1);
    });

    println!("=== Token Account Balance Listen ===\n");
    println!("Token account: {}\n", account_to_listen);

    let config = ClientConfig::default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        config,
    )?;

    // No transaction filter (empty = no tx subscription); account-only
    let transaction_filter = TransactionFilter::default();
    let account_filter =
        AccountFilter { account: vec![account_to_listen.clone()], owner: vec![], filters: vec![] };
    let event_filter = EventTypeFilter::include_only(vec![EventType::TokenAccount]);

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    println!("Listening for token account updates. Press Ctrl+C to stop.\n");

    let queue_clone = queue.clone();
    tokio::spawn(async move {
        loop {
            if let Some(event) = queue_clone.pop() {
                if let DexEvent::TokenAccount(e) = event {
                    println!(
                        "TokenAccount pubkey={} amount={:?} owner={}",
                        e.pubkey, e.amount, e.token_owner
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
