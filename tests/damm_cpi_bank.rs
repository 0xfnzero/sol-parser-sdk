use sol_parser_sdk::{parse_rpc_transaction_with_cost, DexEvent};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;
macro_rules! fields {
 ($e:expr, $($k:ident),+) => {{let mut m=BTreeMap::<String,String>::new();$(m.insert(stringify!($k).into(),$e.$k.to_string());)+m}};
}
#[test]
fn local_patched_damm_cpi_lifecycle() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/damm_cpi_20261009.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 55);
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
                DexEvent::MeteoraDammV2CreatePosition(e) => Some((
                    "MeteoraDammV2CreatePosition",
                    fields!(e, pool, owner, position, position_nft_mint),
                )),
                DexEvent::MeteoraDammV2ClosePosition(e) => Some((
                    "MeteoraDammV2ClosePosition",
                    fields!(e, pool, owner, position, position_nft_mint),
                )),
                DexEvent::MeteoraDammV2AddLiquidity(e) => Some((
                    "MeteoraDammV2AddLiquidity",
                    fields!(
                        e,
                        pool,
                        owner,
                        position,
                        token_a_amount,
                        token_b_amount,
                        total_amount_a,
                        total_amount_b,
                        reserve_a_amount,
                        reserve_b_amount,
                        liquidity_delta,
                        token_a_amount_threshold,
                        token_b_amount_threshold
                    ),
                )),
                DexEvent::MeteoraDammV2RemoveLiquidity(e) => Some((
                    "MeteoraDammV2RemoveLiquidity",
                    fields!(
                        e,
                        pool,
                        owner,
                        position,
                        token_a_amount,
                        token_b_amount,
                        total_amount_a,
                        total_amount_b,
                        reserve_a_amount,
                        reserve_b_amount,
                        liquidity_delta,
                        token_a_amount_threshold,
                        token_b_amount_threshold
                    ),
                )),
                DexEvent::MeteoraDammV2Swap(e) => Some((
                    "MeteoraDammV2Swap",
                    fields!(
                        e,
                        pool,
                        trade_direction,
                        swap_mode,
                        included_transfer_fee_amount_in,
                        included_transfer_fee_amount_out,
                        excluded_transfer_fee_amount_out,
                        claiming_fee,
                        protocol_fee
                    ),
                )),
                DexEvent::MeteoraDammV2ClaimPositionFee(e) => Some((
                    "MeteoraDammV2ClaimPositionFee",
                    fields!(e, pool, position, owner, fee_a_claimed, fee_b_claimed),
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
