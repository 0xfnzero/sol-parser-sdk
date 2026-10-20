//! Subscribe to all ATAs for one or more mints using memcmp filter
//!
//! SPL Token / Token-2022 Associated Token Accounts store the mint pubkey at offset 0.
//! This example uses AccountFilter with memcmp(offset=0, mint_bytes) to receive updates
//! for any ATA of the given mint(s).
//!
//! Run: `cargo run --example mint_all_ata_account_listen --release`
//! Optional: MINT=<pubkey> to monitor a specific mint (default: PUMP and USDC).

use sol_parser_sdk::grpc::{
    account_filter_memcmp, AccountFilter, ClientConfig, EventType, EventTypeFilter,
    TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let pump = Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn").unwrap();
    let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    let mints: Vec<Pubkey> = if let Ok(m) = std::env::var("MINT") {
        vec![Pubkey::from_str(&m).map_err(|_| "Invalid MINT pubkey")?]
    } else {
        vec![pump, usdc]
    };

    println!("=== Mint ATA Account Listen (memcmp offset 0) ===\n");
    println!("Monitoring ATAs for mint(s): {:?}\n", mints);

    let config = ClientConfig::default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        config,
    )?;

    let transaction_filter = TransactionFilter::default();
    let account_filters: Vec<AccountFilter> = mints
        .iter()
        .map(|mint| AccountFilter {
            account: vec![],
            owner: vec![],
            filters: vec![account_filter_memcmp(0, mint.to_bytes().to_vec())],
        })
        .collect();
    let event_filter = EventTypeFilter::include_only(vec![EventType::TokenAccount]);

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], account_filters, Some(event_filter))
        .await?;

    println!("Listening for ATA balance updates. Press Ctrl+C to stop.\n");

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
