use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn current_bank_dlmm_hook_and_rollback() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/dlmm_hook_rewards_20261009.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
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
                DexEvent::MeteoraDlmmClaimReward(e) => Some((
                    "MeteoraDlmmClaimReward",
                    fields!(e, pool, position, owner, reward_index, total_reward, active_bin_id),
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
