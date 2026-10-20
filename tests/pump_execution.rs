use serde_json::Value;
use sol_parser_sdk::{
    grpc::{EventType, EventTypeFilter},
    DexEvent,
};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

fn trades(events: &[DexEvent]) -> Vec<Value> {
    events
        .iter()
        .filter_map(|e| {
            let mut v = serde_json::to_value(e).unwrap();
            let (kind, body) = v.as_object_mut()?.iter_mut().next()?;
            if ![
                "PumpFunTrade",
                "PumpFunBuy",
                "PumpFunSell",
                "PumpFunBuyExactSolIn",
                "PumpSwapBuy",
                "PumpSwapSell",
            ]
            .contains(&kind.as_str())
            {
                return None;
            }
            // Compare execution and identity; instruction context enrichment can depend on logs.
            let fields = [
                "mint",
                "user",
                "pool",
                "is_buy",
                "token_amount",
                "quote_amount",
                "sol_amount",
                "is_created_buy",
                "mayhem_mode",
                "ix_name",
                "third_program",
                "parse_status",
                "base_amount_out",
                "base_amount_in",
                "quote_amount_in",
                "quote_amount_out",
            ];
            body.as_object_mut()?.retain(|k, _| fields.contains(&k.as_str()));
            let kind = if kind.starts_with("PumpFun") { "PumpFunTrade" } else { kind.as_str() };
            Some(serde_json::json!({kind: body}))
        })
        .collect()
}

#[test]
fn execution_survives_missing_or_truncated_logs() {
    for fixture in [
        include_str!("fixtures/pump_execution_20261019.json"),
        include_str!("fixtures/pump_routes_20261009.json"),
        include_str!("fixtures/pump_execution_original.json"),
        include_str!("fixtures/wrapped_pumpfun_reported.json"),
    ] {
        let fixture: Value = serde_json::from_str(fixture).unwrap();
        for c in fixture["cases"].as_array().unwrap() {
            let rpc = serde_json::from_value(c["wire_rpc"].clone()).unwrap();
            let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
            let mut update = SubscribeUpdateTransaction {
                slot: rpc.slot,
                transaction: Some(SubscribeUpdateTransactionInfo {
                    signature: tx.signatures[0].clone(),
                    transaction: Some(tx),
                    meta: Some(meta),
                    ..Default::default()
                }),
            };
            let filter = EventTypeFilter::include_only(vec![
                EventType::PumpFunTrade,
                EventType::PumpSwapTrade,
            ]);
            let baseline =
                trades(&sol_parser_sdk::parse_rpc_transaction(&rpc, Some(&filter)).unwrap());
            assert!(
                !baseline.is_empty(),
                "{} {:?}",
                c["name"],
                sol_parser_sdk::parse_rpc_transaction(&rpc, Some(&filter)).unwrap()
            );
            for mode in 0..3 {
                let mut u = update.clone();
                let m = u.transaction.as_mut().unwrap().meta.as_mut().unwrap();
                if mode == 1 {
                    m.log_messages.clear();
                }
                if mode == 2 {
                    m.log_messages.truncate(m.log_messages.len() / 2);
                }
                for events in [
                    sol_parser_sdk::grpc::parse_subscribe_update_transaction(
                        &u,
                        0,
                        None,
                        Some(&filter),
                    ),
                    sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
                        &u,
                        0,
                        None,
                        Some(&filter),
                    ),
                ] {
                    assert_eq!(trades(&events), baseline, "{} mode={mode}", c["name"]);
                }
            }
            // Independently read the official TradeEvent wire prefix: mint, SOL, tokens.
            // This catches equal-but-wrong log/CPI results, and repeated same-mint buys.
            let m = update.transaction.as_ref().unwrap().meta.as_ref().unwrap();
            let expected_tokens: Vec<u64> = m
                .inner_instructions
                .iter()
                .flat_map(|g| &g.instructions)
                .filter(|ix| {
                    ix.data.starts_with(&[
                        228, 69, 165, 46, 81, 203, 154, 29, 189, 219, 127, 211, 78, 230, 97, 238,
                    ])
                })
                .map(|ix| u64::from_le_bytes(ix.data[56..64].try_into().unwrap()))
                .collect();
            if !expected_tokens.is_empty() {
                let actual: Vec<u64> = baseline
                    .iter()
                    .filter_map(|v| v.get("PumpFunTrade")?.get("token_amount")?.as_u64())
                    .collect();
                assert_eq!(actual, expected_tokens, "{}", c["name"]);
            }
            // With no execution evidence, compiled limits must never masquerade as fills.
            let m = update.transaction.as_mut().unwrap().meta.as_mut().unwrap();
            m.log_messages.clear();
            for group in &mut m.inner_instructions {
                group
                    .instructions
                    .retain(|ix| !ix.data.starts_with(&[228, 69, 165, 46, 81, 203, 154, 29]));
            }
            for event in sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
                &update,
                0,
                None,
                Some(&filter),
            ) {
                if let DexEvent::PumpFunTrade(t)
                | DexEvent::PumpFunBuy(t)
                | DexEvent::PumpFunSell(t)
                | DexEvent::PumpFunBuyExactSolIn(t) = event
                {
                    assert_eq!(
                        (t.sol_amount, t.quote_amount, t.token_amount),
                        (0, 0, 0),
                        "{}",
                        c["name"]
                    );
                }
            }
            update.transaction.as_mut().unwrap().meta.as_mut().unwrap().err =
                Some(Default::default());
            assert!(sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
                &update, 0, None, None
            )
            .is_empty());
        }
    }
}
