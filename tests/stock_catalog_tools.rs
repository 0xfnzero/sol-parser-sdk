//! Exercise the same offline maintenance implementation used by the examples.
#[allow(dead_code)]
#[path = "../examples/support/stock_catalog.rs"]
mod stock_catalog;
use serde_json::{json, Value};
use stock_catalog::*;

fn candidate(pool: &str, rate: Value, reserve: f64) -> Value {
    json!({"accepted":true, "pool":pool, "mint":"M", "quote_mint":"Q", "symbol":"TEST",
        "decimals":6, "catalog_source":"official", "liquid_side_reserve_usd":reserve,
        "program_id":"CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
        "fees":{"snapshot_slot":100, "unix_timestamp":123, "swap_fee_bps":rate, "base_swap_fee_bps":1,
            "stock_transfer_tax":{"basis_points":0, "transfer_hook":false},
            "quote_transfer_tax":{"basis_points":0, "transfer_hook":false}}})
}
fn report(rows: Vec<Value>) -> Value {
    json!({"minimum_liquid_side_usd":500, "volume_filter":false, "candidates":rows})
}

#[test]
fn reserve_threshold_ignores_volume_and_legacy_eligibility() {
    let mut audit = json!({"liquid_side_reserve_usd":500, "buy_output":1, "sell_output":1, "volume":0, "eligible":false});
    assert!(depth_passes(&audit, 500.0));
    for reserve in [json!(499.99), json!(null), json!("500")] {
        audit["liquid_side_reserve_usd"] = reserve;
        assert!(!depth_passes(&audit, 500.0));
    }
    audit["liquid_side_reserve_usd"] = json!(500);
    audit["sell_output"] = json!(0);
    assert!(!depth_passes(&audit, 500.0));
}

#[test]
fn unknown_or_hook_fees_are_not_zero() {
    let mut fees = candidate("pool", json!(8), 500.0)["fees"].clone();
    assert!(nominal_fee_score(&fees, &Value::Null).is_none());
    assert!((nominal_fee_score(&fees, &json!(8)).unwrap() - 8.0).abs() < 1e-8);
    fees["stock_transfer_tax"]["basis_points"] = json!(100);
    assert!((nominal_fee_score(&fees, &json!(8)).unwrap() - 107.92).abs() < 1e-8);
    for rate in [json!(-1), json!(10000), json!("8")] {
        assert!(nominal_fee_score(&fees, &rate).is_none());
    }
    fees["stock_transfer_tax"]["transfer_hook"] = json!(true);
    assert!(nominal_fee_score(&fees, &json!(8)).is_none());
}

#[test]
fn inconsistent_evidence_is_rejected() {
    let mut input = report(vec![]);
    input["minimum_liquid_side_usd"] = json!(0);
    assert!(build_catalog(&input, &json!([])).is_err());
    for (timestamp, base) in
        [(json!(124), json!(1)), (json!(123), json!(2)), (json!(123), Value::Null)]
    {
        let audit = json!([{"pool":"pool", "fees":{"slot":100, "unix_timestamp":timestamp, "base_fee_bps":base}}]);
        assert!(build_catalog(&report(vec![candidate("pool", json!(10), 500.0)]), &audit).is_err());
    }
    assert!(build_catalog(&report(vec![candidate("pool", json!(10), 499.0)]), &json!([])).is_err());
}

#[test]
fn matching_dynamic_snapshot_beats_fixed_pool() {
    let audit = json!([{"pool":"dynamic", "fees":{"slot":100, "unix_timestamp":123, "base_fee_bps":1, "snapshot_start_fee_bps":2}}]);
    let catalog = build_catalog(
        &report(vec![
            candidate("fixed", json!(10), 500.0),
            candidate("dynamic", Value::Null, 500.0),
        ]),
        &audit,
    )
    .unwrap();
    assert_eq!(catalog["pools"][0]["pool"], "dynamic");
    assert_eq!(catalog["pools"][0]["preferred"], true);
    assert_eq!(catalog["pools"][1]["preferred"], false);
    assert!(catalog["pools"][0]["catalog_fee_snapshot"]["swap_fee_bps"].is_null());
}

#[test]
fn stale_fee_and_hook_follow_known_fees() {
    let audit = json!([{"pool":"dynamic", "fees":{"slot":99, "snapshot_start_fee_bps":0}}]);
    let mut hook = candidate("hook", json!(0), 500.0);
    hook["fees"]["stock_transfer_tax"]["transfer_hook"] = json!(true);
    let catalog = build_catalog(
        &report(vec![
            candidate("dynamic", Value::Null, 500.0),
            hook,
            candidate("fixed", json!(10), 500.0),
        ]),
        &audit,
    )
    .unwrap();
    assert_eq!(catalog["pools"][0]["pool"], "fixed");
    assert!(catalog["pools"].as_array().unwrap()[1..]
        .iter()
        .all(|p| p["nominal_snapshot_fee_bps"].is_null()));
}

#[test]
fn lower_fees_then_deeper_reserves_determine_preference() {
    let mut rejected = candidate("rejected", json!(0), 500.0);
    rejected["accepted"] = json!(false);
    let catalog = build_catalog(
        &report(vec![
            candidate("unknown", Value::Null, 9999.0),
            candidate("costly", json!(25), 9999.0),
            candidate("small", json!(8), 500.0),
            candidate("deep", json!(8), 1000.0),
            rejected,
        ]),
        &json!([]),
    )
    .unwrap();
    let names: Vec<_> =
        catalog["pools"].as_array().unwrap().iter().map(|p| p["pool"].as_str().unwrap()).collect();
    assert_eq!(names, ["deep", "small", "costly", "unknown"]);
}

#[test]
fn parity_check_rejects_real_drift_and_integer_precision_loss() {
    assert!(verify_catalog(&json!({"rate":25}), &json!({"rate":25.0})).is_ok());
    assert!(verify_catalog(&json!({"rate":25}), &json!({"rate":25.001})).is_err());
    assert!(verify_catalog(&json!(["a", "b"]), &json!(["b", "a"])).is_err());
    assert!(verify_catalog(
        &json!({"slot":9007199254740992u64}),
        &json!({"slot":9007199254740993u64})
    )
    .is_err());
}

#[test]
fn packaged_catalog_rebuilds_from_recorded_evidence() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // The matching audit is stored alongside the parser evidence for standalone CI.
    let catalog = build_catalog(
        &read_json(root.join("docs/research/stock-catalog-20261022.json")).unwrap(),
        &read_json(root.join("tests/fixtures/stock_catalog_fee_audit.json")).unwrap(),
    )
    .unwrap();
    verify_catalog(&catalog, &read_json(root.join("src/stock_token_catalog.json")).unwrap())
        .unwrap();
    assert_eq!(catalog["tokens"].as_array().unwrap().len(), 33);
    assert_eq!(catalog["pools"].as_array().unwrap().len(), 58);
    for excluded in [
        "52PqYh77gmA6AbM5Hs6gHe9mhGBey5CwmD7sDybaYj27",
        "Bu2A9LUZaMRnf7ZyC1nzREn6dVGVVuPwxz5aFBFkJLTa",
        "8YU7gLMh4T8RuGepxAuigTqZeGfBn3TDS6tnuj951XbW",
        "6E12HKuZkyQoMwhTtN4WFs95uMEoW2zXJMFSE4jNcjHE",
        "5vJcDWLBQSh767DzHACeEmfMh7nGCWZc8eb76v6cCFVr",
    ] {
        assert!(catalog["pools"].as_array().unwrap().iter().all(|p| p["pool"] != excluded));
    }
}
