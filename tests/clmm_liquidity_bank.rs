use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn current_bank_clmm_liquidity_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/clmm_liquidity_20261008.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} rollback", c["name"]);
            if c["name"].as_str().unwrap().ends_with("close_nonempty") {
                let count = c["bank_validation"]["rolled_back_liquidity_events"].as_u64().unwrap();
                assert!(count > 0);
            }
            continue;
        }
        // Compare typed quantities without a lossy numeric conversion.
        let actual: Vec<_> = parsed
            .events
            .iter()
            .filter_map(|e| match e {
                DexEvent::RaydiumClmmIncreaseLiquidity(e) => Some((
                    "RaydiumClmmIncreaseLiquidity",
                    fields!(
                        e,
                        pool,
                        user,
                        personal_position,
                        position_nft_mint,
                        liquidity,
                        amount_0,
                        amount_1,
                        amount_0_transfer_fee,
                        amount_1_transfer_fee
                    ),
                )),
                DexEvent::RaydiumClmmDecreaseLiquidity(e) => Some((
                    "RaydiumClmmDecreaseLiquidity",
                    fields!(
                        e,
                        pool,
                        user,
                        personal_position,
                        position_nft_mint,
                        liquidity,
                        decrease_amount_0,
                        decrease_amount_1,
                        fee_amount_0,
                        fee_amount_1,
                        transfer_fee_0,
                        transfer_fee_1
                    ),
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
