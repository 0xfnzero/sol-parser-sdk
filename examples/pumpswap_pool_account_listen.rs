//! PumpSwap pool account subscription with memcmp filter
//!
//! Subscribe to account updates for specific PumpSwap pool(s) using memcmp on account data
//! (e.g. mint at offset 32). Uses queue-based API and AccountFilter with memcmp filters.
//!
//! Run: `cargo run --example pumpswap_pool_account_listen --release`

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

    // Example: PumpSwap pool identifiers (mint or pool pubkey at offset 32 in pool account data)
    let pump_usdc = Pubkey::from_str("2uF4Xh61rDwxnG9woyxsVQP7zuA6kLFpb3NvnRQeoiSd").unwrap();
    let wsol_deepseekai = Pubkey::from_str("BJAjivuMVANjpRWtrRfcxzGhnMSywBN19Sa4jAzWxXDx").unwrap();

    println!("=== PumpSwap Pool Account Listen (memcmp) ===\n");
    println!("Monitoring pool accounts with mint at offset 32\n");

    let config = ClientConfig::default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        config,
    )?;

    let transaction_filter = TransactionFilter::default();
    let pool1_filter = AccountFilter {
        account: vec![],
        owner: vec![],
        filters: vec![account_filter_memcmp(32, pump_usdc.to_bytes().to_vec())],
    };
    let pool2_filter = AccountFilter {
        account: vec![],
        owner: vec![],
        filters: vec![account_filter_memcmp(32, wsol_deepseekai.to_bytes().to_vec())],
    };
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::TokenAccount,
        EventType::AccountPumpSwapPool,
    ]);

    let queue = grpc
        .subscribe_dex_events(
            vec![transaction_filter],
            vec![pool1_filter, pool2_filter],
            Some(event_filter),
        )
        .await?;

    println!("Listening for pool / token account updates. Press Ctrl+C to stop.\n");

    let queue_clone = queue.clone();
    tokio::spawn(async move {
        loop {
            if let Some(event) = queue_clone.pop() {
                match &event {
                    DexEvent::TokenAccount(e) => {
                        println!("TokenAccount pubkey={} amount={:?}", e.pubkey, e.amount);
                    }
                    DexEvent::PumpSwapPoolAccount(e) => {
                        println!(
                            "PumpSwapPoolAccount pubkey={} pool(base_mint={}, quote_mint={}, mayhem={}, cashback={})",
                            e.pubkey,
                            e.pool.base_mint,
                            e.pool.quote_mint,
                            e.pool.is_mayhem_mode,
                            e.pool.is_cashback_coin
                        );
                    }
                    _ => {}
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
