use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn local_patched_damm_reward_admins() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/damm_reward_admin_20261009.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 37);
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
                DexEvent::MeteoraDammV2InitializeReward(e) => Some((
                    "MeteoraDammV2InitializeReward",
                    fields!(e, pool, reward_mint, funder, creator, reward_index, reward_duration),
                )),
                DexEvent::MeteoraDammV2UpdateRewardDuration(e) => Some((
                    "MeteoraDammV2UpdateRewardDuration",
                    fields!(e, pool, reward_index, old_reward_duration, new_reward_duration),
                )),
                DexEvent::MeteoraDammV2UpdateRewardFunder(e) => Some((
                    "MeteoraDammV2UpdateRewardFunder",
                    fields!(e, pool, reward_index, old_funder, new_funder),
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

#[test]
fn administration_structured_logs_and_truncation() {
    use sol_parser_sdk::logs::meteora_damm::{
        parse_initialize_reward_from_data, parse_log, parse_update_reward_duration_from_data,
        parse_update_reward_funder_from_data,
    };
    use sol_parser_sdk::EventMetadata;
    use solana_sdk::signature::Signature;
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/damm_reward_admin_20261009.json")).unwrap();
    for c in f["cases"].as_array().unwrap() {
        for item in c["logs"].as_array().unwrap() {
            let kind = item["expected"]["type"].as_str().unwrap();
            if kind == "MeteoraDammV2FundReward" {
                continue;
            }
            let e = parse_log(item["log"].as_str().unwrap(), Signature::default(), 1, 0, None, 0)
                .unwrap();
            let body = match e {
                DexEvent::MeteoraDammV2InitializeReward(e) => {
                    fields!(e, pool, reward_mint, funder, creator, reward_index, reward_duration)
                }
                DexEvent::MeteoraDammV2UpdateRewardDuration(e) => {
                    fields!(e, pool, reward_index, old_reward_duration, new_reward_duration)
                }
                DexEvent::MeteoraDammV2UpdateRewardFunder(e) => {
                    fields!(e, pool, reward_index, old_funder, new_funder)
                }
                _ => panic!("wrong structured event"),
            };
            for (k, v) in item["expected"].as_object().unwrap() {
                if k != "type" {
                    assert_eq!(body[k], v.as_str().unwrap());
                }
            }
            let wire: Vec<u8> = item["bytes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect();
            let parser: fn(&[u8], EventMetadata) -> Option<DexEvent> = match kind {
                "MeteoraDammV2InitializeReward" => parse_initialize_reward_from_data,
                "MeteoraDammV2UpdateRewardDuration" => parse_update_reward_duration_from_data,
                "MeteoraDammV2UpdateRewardFunder" => parse_update_reward_funder_from_data,
                _ => unreachable!(),
            };
            for end in 0..wire.len() - 8 {
                assert!(
                    parser(&wire[8..8 + end], EventMetadata::default()).is_none(),
                    "{kind} {end}"
                );
            }
        }
    }
}
