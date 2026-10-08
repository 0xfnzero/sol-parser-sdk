use sol_parser_sdk::parse_rpc_transaction_with_cost;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

#[test]
fn current_bank_dbc_boundary_and_rollback() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/dbc_boundary_20261008.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for case in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(case["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !case["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} retained rolled-back events", case["name"]);
            if case["name"].as_str().unwrap().contains("after_curve_completed") {
                assert!(case["rolled_back_curve_complete_payloads"].as_u64().unwrap() > 0);
            }
            continue;
        }
        let events: serde_json::Value = serde_json::to_value(parsed.events).unwrap();
        let swaps: Vec<_> =
            events.as_array().unwrap().iter().filter_map(|e| e.get("MeteoraDbcSwap")).collect();
        let complete: Vec<_> = events
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e.get("MeteoraDbcCurveComplete"))
            .collect();
        assert_eq!(swaps.len(), 1);
        assert_eq!(complete.len(), 1);
        let e = swaps[0];
        for (key, want) in case["expected_swap"].as_object().unwrap() {
            if want.is_boolean() {
                assert_eq!(&e[key], want);
            } else {
                assert_eq!(e[key].as_u64().unwrap().to_string(), want.as_str().unwrap(), "{key}");
            }
        }
        let v = &case["validation"];
        assert_eq!(e["included_fee_input_amount"], v["bank_consumed_quote"]);
        assert_eq!(e["output_amount"], v["bank_base_credit"]);
        assert_eq!(e["referral_fee"], v["bank_referral_credit"]);
        assert_ne!(e["amount_left"], v["bank_unconsumed_quote"]);
        assert_eq!(
            e["amount_left"].as_u64().unwrap() + v["initial_requested_input_fee"].as_u64().unwrap()
                - v["recalculated_fill_fee"].as_u64().unwrap(),
            v["bank_unconsumed_quote"].as_u64().unwrap()
        );
        assert_eq!(complete[0]["quote_reserve"], e["quote_reserve_amount"]);
        assert!(
            e["quote_reserve_amount"].as_u64().unwrap()
                >= e["migration_threshold"].as_u64().unwrap()
        );
    }
}
