use sol_parser_sdk::parse_rpc_transaction_with_cost;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
#[test]
fn current_bank_pumpswap_liquidity_intent_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pumpswap_liquidity_20261008.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let p = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(p.events.is_empty(), "{} rollback", c["name"]);
            continue;
        }
        let events = serde_json::to_value(p.events).unwrap();
        let lp: Vec<_> = events
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| {
                e.get("PumpSwapLiquidityAdded").map(|v| ("PumpSwapLiquidityAdded", v)).or_else(
                    || e.get("PumpSwapLiquidityRemoved").map(|v| ("PumpSwapLiquidityRemoved", v)),
                )
            })
            .collect();
        let wants = c["expected"].as_array().unwrap();
        assert_eq!(lp.len(), wants.len(), "{}", c["name"]);
        for ((kind, e), w) in lp.iter().zip(wants) {
            assert_eq!(*kind, w["type"].as_str().unwrap());
            for (k, v) in w.as_object().unwrap() {
                if k == "type" {
                    continue;
                }
                let actual = if k == "pool" || k == "user" || k.ends_with("_account") {
                    serde_json::from_value::<solana_sdk::pubkey::Pubkey>(e[k].clone())
                        .unwrap()
                        .to_string()
                } else {
                    e[k].as_u64().unwrap().to_string()
                };
                assert_eq!(actual, v.as_str().unwrap(), "{} {k}", c["name"]);
            }
        }
    }
}
