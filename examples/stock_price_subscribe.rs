//! GRPC_ENDPOINT=... GRPC_AUTH_TOKEN=... cargo run --example stock_price_subscribe
//! Explicitly enables the screened catalog. No subscription is enabled by default.
use sol_parser_sdk::{
    grpc::{ClientConfig, YellowstoneGrpc},
    stock_prices::{StockPriceConfig, StockPriceSubscription, StockTokenCatalog},
};

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
        ClientConfig::default(),
    )?;
    let config =
        StockPriceConfig { catalog: StockTokenCatalog::screened_defaults(), ..Default::default() };
    // Before subscribing, callers may extend/replace config.catalog.tokens/pools.
    // Supply independently collected live liquidity through update_liquidity().
    let mut subscription = StockPriceSubscription::subscribe(grpc, config).await?;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tokio::time::sleep(std::time::Duration::from_millis(1)) => {
                let now_us = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_micros() as i64;
                // Yield to cancellation even when the producer never falls idle.
                for _ in 0..64 {
                    let Some(event) = subscription.pop(now_us) else { break };
                    println!("{}", serde_json::to_string(&event)?);
                }
            }
        }
    }
    subscription.stop().await;
    Ok(())
}
