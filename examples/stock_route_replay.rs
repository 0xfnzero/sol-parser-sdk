//! Offline: cargo run --example stock_route_replay
//! Optional: --fixture FILE containing {"cases":[{"rpc":...}]} signed RPC captures.
//! For a new signature, use rpc_transaction_replay --fetch off the hot path.
use sol_parser_sdk::{analyze_rpc_transaction_routes, stock_prices::StockTokenCatalog};

#[path = "support/replay_input.rs"]
mod replay_input;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cases) = replay_input::load(&[include_str!(
        "../tests/fixtures/cross_dex_stock_routes_20261020.json"
    )])?
    else {
        return Ok(());
    };
    let catalog = StockTokenCatalog::screened_defaults();
    for mut case in cases {
        let rpc = serde_json::from_value(case["rpc"].take())?;
        let route = analyze_rpc_transaction_routes(&rpc, &[])?;
        let conversions: Vec<_> = catalog.conversion_legs(&route).map(|l| l.position).collect();
        // Every DEX leg is shown. No launch-platform event filter is applied.
        let legs: Vec<_> = route.legs.iter().map(|l| serde_json::json!({
            "position":l.position,"dex":l.protocol,"program_id":l.program.to_string(),
            "pool":l.pool.to_string(),"input_mint":l.input_mint.map(|m|m.to_string()),
            "output_mint":l.output_mint.map(|m|m.to_string()),"stonkfun_mode":l.stonkfun_mode,
            "catalog_stock_conversion":conversions.contains(&l.position),
            "executed_input_raw":l.actual_input_amount,"executed_output_raw":l.actual_output_amount,
        })).collect();
        println!(
            "{}",
            serde_json::json!({"signature":route.signature.to_string(),
            "succeeded":route.succeeded,"legs":legs,"unknown_invocations":route.unknown_invocations,
            "note":"Historical legs, not a proven linear path or live quotes. Missing amounts remain null."})
        );
    }
    Ok(())
}
