//! Nonce account subscription example
//!
//! Subscribe to a nonce account's state changes via Yellowstone gRPC account filter.
//!
//! Run: `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release`

use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let nonce_account = std::env::var("NONCE_ACCOUNT").unwrap_or_else(|_| {
        eprintln!("Usage: NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release");
        std::process::exit(1);
    });

    println!("=== Nonce Account Listen ===\n");
    println!("Nonce account: {}\n", nonce_account);

    let config = ClientConfig::default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        config,
    )?;

    let transaction_filter = TransactionFilter::default();
    let account_filter =
        AccountFilter { account: vec![nonce_account.clone()], owner: vec![], filters: vec![] };
    let event_filter = EventTypeFilter::include_only(vec![EventType::NonceAccount]);

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    println!("Listening for nonce account updates. Press Ctrl+C to stop.\n");

    let queue_clone = queue.clone();
    tokio::spawn(async move {
        loop {
            if let Some(event) = queue_clone.pop() {
                if let DexEvent::NonceAccount(e) = event {
                    println!(
                        "NonceAccount pubkey={} nonce={} authority={}",
                        e.pubkey, e.nonce, e.authority
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
