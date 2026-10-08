use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn local_patched_damm_reward_renewals() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/damm_reward_renewal_20261009.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for c in cases {
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let parsed = parse_rpc_transaction_with_cost(&tx, None).unwrap();
        if !c["wire_rpc"]["meta"]["err"].is_null() {
            assert!(parsed.events.is_empty(), "{} rollback", c["name"]);
            continue;
        }
        // Compare typed u128 values directly: JSON Value cannot hold this fixture's liquidity.
        let actual: Vec<_> = parsed
            .events
            .iter()
            .filter_map(|e| match e {
                DexEvent::MeteoraDammV2WithdrawIneligibleReward(e) => Some((
                    "MeteoraDammV2WithdrawIneligibleReward",
                    fields!(e, pool, reward_mint, amount),
                )),
                DexEvent::MeteoraDammV2WithdrawDeadLiquidityReward(e) => Some((
                    "MeteoraDammV2WithdrawDeadLiquidityReward",
                    fields!(e, pool, reward_mint, amount),
                )),
                DexEvent::MeteoraDammV2FundReward(e) => Some((
                    "MeteoraDammV2FundReward",
                    fields!(
                        e,
                        pool,
                        funder,
                        mint_reward,
                        reward_index,
                        amount,
                        transfer_fee_excluded_amount_in,
                        reward_duration_end,
                        pre_reward_rate,
                        post_reward_rate
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
