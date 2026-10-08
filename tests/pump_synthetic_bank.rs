use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn current_bank_pump_synthetic_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pump_synthetic_20261009.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} rollback", c["name"]);
            continue;
        }
        // Keep fee quantities exact, including values beyond JS safe integers.
        let actual: Vec<_> = parsed
            .events
            .iter()
            .filter_map(|e| match e {
                DexEvent::PumpFunComplete(e) => Some((
                    "PumpFunComplete",
                    fields!(e, user, mint, bonding_curve, timestamp, quote_mint),
                )),
                DexEvent::PumpFunPostCompleteBuy(e) => Some((
                    "PumpFunPostCompleteBuy", fields!(e, user, mint, bonding_curve, quote_mint, timestamp, base_out, quote_in, fee_basis_points, fee, creator_fee_basis_points, creator_fee, buyback_fee, pool_base_reserves_before, pool_quote_reserves_before, pool_base_reserves_after, pool_quote_reserves_after),
                )),
                _ => None,
            })
            .collect();
        let wants = c["expected"].as_array().unwrap();
        assert_eq!(actual.len(), wants.len(), "{}", c["name"]);
        for ((kind, body), want) in actual.iter().zip(wants) {
            assert_eq!(*kind, want["type"].as_str().unwrap());
            for (k, v) in want.as_object().unwrap() {
                if k != "type" {
                    assert_eq!(body.get(k).unwrap(), v.as_str().unwrap(), "{} {k}", c["name"]);
                }
            }
        }
    }
}
