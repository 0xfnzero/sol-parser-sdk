use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn current_bank_whirlpool_liquidity_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/whirlpool_liquidity_20261008.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 30);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} rollback", c["name"]);
            if c["name"] == "close_nonempty" {
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
                DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => Some((
                    "OrcaWhirlpoolLiquidityIncreased",
                    fields!(
                        e,
                        whirlpool,
                        position,
                        tick_lower_index,
                        tick_upper_index,
                        liquidity,
                        token_a_amount,
                        token_b_amount,
                        token_a_transfer_fee,
                        token_b_transfer_fee
                    ),
                )),
                DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => Some((
                    "OrcaWhirlpoolLiquidityDecreased",
                    fields!(
                        e,
                        whirlpool,
                        position,
                        tick_lower_index,
                        tick_upper_index,
                        liquidity,
                        token_a_amount,
                        token_b_amount,
                        token_a_transfer_fee,
                        token_b_transfer_fee
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

#[cfg(feature = "parse-borsh")]
#[test]
fn official_whirlpool_liquidity_borsh_layout_matches_bank_logs() {
    use base64::Engine;
    use sol_parser_sdk::core::events::{
        OrcaWhirlpoolLiquidityDecreasedEvent, OrcaWhirlpoolLiquidityIncreasedEvent,
    };
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/whirlpool_liquidity_20261008.json")).unwrap();
    let mut count = 0;
    for c in f["cases"].as_array().unwrap() {
        if !c["raw"]["meta"]["err"].is_null() {
            continue;
        }
        let mut wants = c["expected"].as_array().unwrap().iter();
        for line in c["raw"]["meta"]["logMessages"].as_array().unwrap() {
            let Some(encoded) = line.as_str().unwrap().strip_prefix("Program data: ") else {
                continue;
            };
            let data = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
            let body = if data.starts_with(&[30, 7, 144, 181, 102, 254, 155, 161]) {
                let e =
                    borsh::from_slice::<OrcaWhirlpoolLiquidityIncreasedEvent>(&data[8..]).unwrap();
                fields!(
                    e,
                    whirlpool,
                    position,
                    tick_lower_index,
                    tick_upper_index,
                    liquidity,
                    token_a_amount,
                    token_b_amount,
                    token_a_transfer_fee,
                    token_b_transfer_fee
                )
            } else if data.starts_with(&[166, 1, 36, 71, 112, 202, 181, 171]) {
                let e =
                    borsh::from_slice::<OrcaWhirlpoolLiquidityDecreasedEvent>(&data[8..]).unwrap();
                fields!(
                    e,
                    whirlpool,
                    position,
                    tick_lower_index,
                    tick_upper_index,
                    liquidity,
                    token_a_amount,
                    token_b_amount,
                    token_a_transfer_fee,
                    token_b_transfer_fee
                )
            } else {
                continue;
            };
            let want = wants.next().unwrap();
            for (k, v) in want.as_object().unwrap() {
                if k != "type" {
                    assert_eq!(body.get(k).unwrap(), v.as_str().unwrap());
                }
            }
            count += 1;
        }
        assert!(wants.next().is_none());
    }
    assert!(count >= 7);
}
