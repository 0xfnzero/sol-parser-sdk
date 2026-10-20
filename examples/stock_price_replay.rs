//! Replay actual saved stock trades; optionally provide --fixture FILE.
//! Offline execution metadata goes only to the gRPC parser, never to Shred.
use sol_parser_sdk::stock_prices::{StockPriceConfig, StockPriceTracker, StockTokenCatalog};

#[path = "support/grpc_replay.rs"]
mod grpc_replay;

#[path = "support/replay_input.rs"]
mod replay_input;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cases) =
        replay_input::load(&[include_str!("../tests/fixtures/stock_price_replay_20261020.json")])?
    else {
        return Ok(());
    };
    let mut tracker = StockPriceTracker::new(StockPriceConfig {
        catalog: StockTokenCatalog::screened_defaults(),
        ..Default::default()
    })?;
    for mut case in cases {
        // Independent captures have no shared ordering/continuity guarantee.
        tracker.reset();
        let replay_us = 1_000_000;
        let events = grpc_replay::parse(case["rpc"].take(), replay_us)?;
        let prices: Vec<_> = events.iter().filter_map(|e| tracker.process(e, replay_us)).collect();
        println!(
            "{}",
            serde_json::json!({"name":case["name"],"signature":case["signature"],
            "replay":true,"prices":prices})
        );
    }
    Ok(())
}
