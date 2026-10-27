//! Offline replay of original mainnet captures across multiple DEX protocols.
//! Optional: --fixture FILE with {"cases":[{"rpc":...}]} or wire_rpc entries.

#[path = "support/grpc_replay.rs"]
mod grpc_replay;

#[path = "support/replay_input.rs"]
mod replay_input;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cases) = replay_input::load(&[
        include_str!("../tests/fixtures/signed_dex_replay_20261009.json"),
        include_str!("../tests/fixtures/signed_replay_20261009.json"),
        include_str!("../tests/fixtures/wrapped_pumpfun_reported.json"),
    ])?
    else {
        return Ok(());
    };
    for mut case in cases {
        let slot = case["rpc"]["slot"].clone();
        let events = grpc_replay::parse(case["rpc"].take(), 0)?;
        let rows: Vec<_> = events
            .iter()
            .filter_map(|e| {
                let source = e.third_program()?;
                let kind = sol_parser_sdk::grpc::types::event_type_from_dex_event(e)
                    .map(|kind| format!("{kind:?}"));
                Some(serde_json::json!({"event":kind,"third_program":source}))
            })
            .collect();
        println!("{}", serde_json::json!({"name":case["name"],"slot":slot,"trades":rows}));
    }
    Ok(())
}
