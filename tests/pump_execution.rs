use serde_json::Value;
use sol_parser_sdk::{
    grpc::{EventType, EventTypeFilter},
    DexEvent,
};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

#[test]
fn reported_create_v2_with_alt_accounts_keeps_create_and_trade_callbacks() {
    use solana_sdk::pubkey::Pubkey;
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_create_alt_20261026.json")).unwrap();
    let filter =
        EventTypeFilter::include_only(vec![EventType::PumpFunCreate, EventType::PumpFunTrade]);
    for (case, method) in
        fixture["cases"].as_array().unwrap().iter().zip(["buy_exact_sol_in", "buy_v2", "buy_v3"])
    {
        let rpc = serde_json::from_value(case["wire_rpc"].clone()).unwrap();
        let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
        let signed = rpc.transaction.transaction.decode().unwrap();
        signed.verify_and_hash_message().unwrap();
        assert_eq!(signed.signatures[0].to_string(), case["signature"].as_str().unwrap());
        let mut events = Vec::new();
        sol_parser_sdk::shredstream::pump_ix::parse_transaction_dex_events_with_filter(
            &signed,
            signed.signatures[0],
            rpc.slot,
            888,
            123456,
            Some(&filter),
            &mut events,
        );
        assert_eq!(events.len(), 2, "{}: {method}", case["signature"]);
        let DexEvent::PumpFunCreate(create) = &events[0] else { panic!("create must come first") };
        let DexEvent::PumpFunTrade(trade) = &events[1] else { panic!("missing trade") };
        assert_eq!(create.ix_name, "create_v2");
        assert_eq!(trade.ix_name, method);
        assert_eq!(trade.mint, create.mint);
        assert_eq!(trade.user, create.user);
        assert_ne!(create.mint, Pubkey::default());
        assert_ne!(create.bonding_curve, Pubkey::default());
        assert!(trade.is_buy && trade.is_created_buy);
        assert!(trade.third_program.is_none());
        assert_eq!(create.token_program, Pubkey::default());
        assert_eq!(trade.token_program, Pubkey::default());
        if method != "buy_exact_sol_in" {
            // Neither the quote mint nor its token program is in static keys.
            assert_eq!(trade.quote_mint, Pubkey::default());
            assert_eq!(trade.quote_token_program, Pubkey::default());
        }
        for event in &events {
            let metadata = event.metadata();
            assert_eq!(
                (metadata.slot, metadata.tx_index, metadata.grpc_recv_us),
                (rpc.slot, 888, 123456)
            );
            assert_eq!(metadata.signature, signed.signatures[0]);
        }

        let addresses = |keys: &[Vec<u8>]| -> Vec<Pubkey> {
            keys.iter().map(|bytes| Pubkey::try_from(bytes.as_slice()).unwrap()).collect()
        };
        let mut resolved = Vec::new();
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &signed,
            &addresses(&meta.loaded_writable_addresses),
            &addresses(&meta.loaded_readonly_addresses),
            signed.signatures[0],
            rpc.slot,
            888,
            123456,
            Some(&filter),
            &mut resolved,
        )
        .unwrap();
        assert_eq!(resolved.len(), 2);
        let DexEvent::PumpFunCreate(resolved_create) = &resolved[0] else {
            panic!("resolved create")
        };
        let DexEvent::PumpFunTrade(resolved_trade) = &resolved[1] else { panic!("resolved trade") };
        assert_eq!(create.creator, resolved_create.creator);
        assert_ne!(resolved_create.token_program, Pubkey::default());
        assert_ne!(resolved_trade.token_program, Pubkey::default());
        assert_ne!(resolved_trade.quote_mint, Pubkey::default());
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx),
                meta: Some(meta),
                index: 888,
                ..Default::default()
            }),
        };
        let grpc = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            123456,
            None,
            Some(&filter),
        );
        assert_eq!(grpc.len(), 2);
        let DexEvent::PumpFunCreate(confirmed_create) = &grpc[0] else { panic!("gRPC create") };
        assert_eq!(confirmed_create.ix_name, "create_v2");
        let DexEvent::PumpFunTrade(confirmed_trade) = &grpc[1] else { panic!("gRPC trade") };
        // Creator input can differ from the creator recorded by the program.
        assert_eq!(
            (&create.name, &create.symbol, &create.uri, create.mint, create.user),
            (
                &confirmed_create.name,
                &confirmed_create.symbol,
                &confirmed_create.uri,
                confirmed_create.mint,
                confirmed_create.user
            )
        );
        assert_eq!(trade.ix_name, confirmed_trade.ix_name);
        assert_eq!(
            (trade.amount, trade.max_sol_cost, trade.spendable_sol_in, trade.min_tokens_out),
            (
                confirmed_trade.amount,
                confirmed_trade.max_sol_cost,
                confirmed_trade.spendable_sol_in,
                confirmed_trade.min_tokens_out
            )
        );
        assert!(confirmed_trade.token_amount > 0);

        for (types, expected) in [
            (vec![EventType::PumpFunCreate], 1),
            (vec![EventType::PumpFunTrade], 1),
            (vec![EventType::PumpSwapTrade], 0),
        ] {
            events.clear();
            sol_parser_sdk::shredstream::pump_ix::parse_transaction_dex_events_with_filter(
                &signed,
                signed.signatures[0],
                rpc.slot,
                888,
                123456,
                Some(&EventTypeFilter::include_only(types)),
                &mut events,
            );
            assert_eq!(events.len(), expected, "{method}: filter");
        }
    }
}

#[test]
fn partial_pump_accounts_do_not_allow_unknown_programs_or_invalid_indices() {
    use solana_sdk::{message::VersionedMessage, pubkey::Pubkey};
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_create_alt_20261026.json")).unwrap();
    let rpc: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_value(fixture["cases"][0]["wire_rpc"].clone()).unwrap();
    let signed = rpc.transaction.transaction.decode().unwrap();
    for invalid_indices in [false, true] {
        let mut transaction = signed.clone();
        let VersionedMessage::V0(message) = &mut transaction.message else { panic!("v0") };
        let program_index = message.instructions[2].program_id_index;
        if invalid_indices {
            for instruction in &mut message.instructions {
                if instruction.program_id_index == program_index {
                    instruction.accounts[0] = u8::MAX;
                }
            }
        } else {
            // Copying the same instruction bytes to a different program proves nothing.
            message.account_keys[program_index as usize] = Pubkey::new_unique();
        }
        let mut events = Vec::new();
        sol_parser_sdk::shredstream::pump_ix::parse_transaction_dex_events_with_filter(
            &transaction,
            signed.signatures[0],
            rpc.slot,
            0,
            0,
            None,
            &mut events,
        );
        assert!(events.is_empty());
    }
}

#[test]
fn wrapped_creation_precedes_its_trades_with_partial_logs() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_execution_20261019.json")).unwrap();
    let mut checked = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let rpc = serde_json::from_value(case["wire_rpc"].clone()).unwrap();
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
        // Keep invocation logs and trade events; recover missing creates from CPI.
        update.transaction.as_mut().unwrap().meta.as_mut().unwrap().log_messages.retain(|log| {
            !log.strip_prefix("Program data: ")
                .and_then(|s| STANDARD.decode(s).ok())
                .is_some_and(|data| data.starts_with(&[27, 114, 169, 77, 222, 235, 99, 118]))
        });
        let events = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update, 0, None, None,
        );
        for (create_index, event) in events.iter().enumerate() {
            let mint = match event {
                DexEvent::PumpFunCreate(c) => c.mint,
                DexEvent::PumpFunCreateV2(c) => c.mint,
                _ => continue,
            };
            for (trade_index, event) in events.iter().enumerate() {
                match event {
                    DexEvent::PumpFunTrade(t)
                    | DexEvent::PumpFunBuy(t)
                    | DexEvent::PumpFunSell(t)
                    | DexEvent::PumpFunBuyExactSolIn(t)
                        if t.mint == mint =>
                    {
                        assert!(
                            create_index < trade_index,
                            "{}: create={create_index}, trade={trade_index}",
                            case["name"]
                        );
                        checked += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(checked > 0);
}

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
fn versioned_cpi_names_keep_filtered_execution_amounts() {
    use sol_parser_sdk::instr::pump::discriminators;
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_holder_20261009.json")).unwrap();
    for (name, disc, case_index, old_name, kind) in [
        ("buy_v3", discriminators::BUY_V3, 0, "buy_exact_quote_in", EventType::PumpFunBuy),
        (
            "buy_exact_quote_in_v3",
            discriminators::BUY_EXACT_QUOTE_IN_V3,
            0,
            "buy_exact_quote_in",
            EventType::PumpFunBuy,
        ),
        ("sell_v3", discriminators::SELL_V3, 1, "sell", EventType::PumpFunSell),
    ] {
        let rpc = serde_json::from_value(fixture["cases"][case_index]["wire_rpc"].clone()).unwrap();
        let (mut meta, mut tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
        let message = tx.message.as_mut().unwrap();
        let ix = message
            .instructions
            .iter_mut()
            .find(|ix| {
                [discriminators::BUY_EXACT_QUOTE_IN_V3, discriminators::SELL_V3]
                    .iter()
                    .any(|d| ix.data.starts_with(d))
            })
            .unwrap();
        ix.data[..8].copy_from_slice(&disc);
        let mut expected = None;
        // Explicitly constructed variant of the saved payload: change only the
        // Borsh method name to exercise versioned CPI names, keep fill bytes.
        for ix in meta.inner_instructions.iter_mut().flat_map(|g| &mut g.instructions) {
            if !ix.data.starts_with(&[
                228, 69, 165, 46, 81, 203, 154, 29, 189, 219, 127, 211, 78, 230, 97, 238,
            ]) {
                continue;
            }
            expected = Some((
                u64::from_le_bytes(ix.data[48..56].try_into().unwrap()),
                u64::from_le_bytes(ix.data[56..64].try_into().unwrap()),
            ));
            let mut old = (old_name.len() as u32).to_le_bytes().to_vec();
            old.extend_from_slice(old_name.as_bytes());
            let offset = ix.data.windows(old.len()).position(|s| s == old).unwrap();
            let mut replacement = (name.len() as u32).to_le_bytes().to_vec();
            replacement.extend_from_slice(name.as_bytes());
            ix.data.splice(offset..offset + old.len(), replacement);
        }
        meta.log_messages.clear();
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx),
                meta: Some(meta),
                ..Default::default()
            }),
        };
        let filter = EventTypeFilter::include_only(vec![kind]);
        let events = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            0,
            None,
            Some(&filter),
        );
        assert_eq!(events.len(), 1, "{name}");
        let t = match &events[0] {
            DexEvent::PumpFunBuy(t) | DexEvent::PumpFunSell(t) => t,
            _ => panic!("{name}"),
        };
        assert_eq!((t.sol_amount, t.token_amount), expected.unwrap(), "{name}");
        assert_ne!(t.token_amount, 0);
        assert_eq!(t.ix_name, name);
    }
}

#[test]
fn execution_survives_missing_or_truncated_logs() {
    for fixture in [
        include_str!("fixtures/pump_execution_20261019.json"),
        include_str!("fixtures/pump_routes_20261009.json"),
        include_str!("fixtures/pump_execution_original.json"),
        include_str!("fixtures/wrapped_pumpfun_reported.json"),
        include_str!("fixtures/pump_holder_20261009.json"),
        include_str!("fixtures/pump_multi_hop_20261025.json"),
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
                for kind in [EventType::PumpFunBuy, EventType::PumpFunSell] {
                    let specific = EventTypeFilter::include_only(vec![kind]);
                    let expected: Vec<_> = baseline
                        .iter()
                        .filter(|v| {
                            v.get("PumpFunTrade").is_some_and(|t| {
                                t["is_buy"].as_bool() == Some(kind == EventType::PumpFunBuy)
                                    && t["ix_name"].as_str().is_some_and(|name| {
                                        name.starts_with("buy") || name.starts_with("sell")
                                    })
                            })
                        })
                        .cloned()
                        .collect();
                    let actual = trades(
                        &sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
                            &u,
                            0,
                            None,
                            Some(&specific),
                        ),
                    );
                    assert_eq!(actual, expected, "{} {kind:?} mode={mode}", c["name"]);
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

#[test]
fn official_multi_hop_reports_curve_input_and_preserves_executed_quote_amounts() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/pump_multi_hop_20261025.json")).unwrap();
    let rpc = serde_json::from_value(fixture["cases"][0]["wire_rpc"].clone()).unwrap();
    let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
    let signed = rpc.transaction.transaction.decode().unwrap();
    signed.verify_and_hash_message().unwrap();
    assert_eq!(
        signed.signatures[0].to_string(),
        fixture["cases"][0]["signature"].as_str().unwrap()
    );
    let filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
    let mut shred = Vec::new();
    sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter(
        &signed,
        signed.signatures[0],
        rpc.slot,
        77,
        123456,
        Some(&filter),
        &mut shred,
    );
    assert_eq!(shred.len(), 1);
    let DexEvent::PumpFunTrade(input) = &shred[0] else { panic!("missing curve callback") };
    assert_eq!(input.ix_name, "multi_hop_swap");
    assert_eq!(input.mint.to_string(), "Aijxkeq5L9iaDYZxfaXUhnHQj4cYeo8um6a3t5vzpump");
    assert_eq!(input.quote_mint.to_string(), "6Vk3fAQWYEot6JbQb8Nx8j5wZeDnMhzk8FajjHhZpump");
    assert!(input.is_buy && input.is_created_buy);
    assert_eq!((input.sol_amount, input.token_amount, input.quote_amount), (0, 0, 0));
    assert_eq!((input.metadata.tx_index, input.metadata.grpc_recv_us), (77, 123456));
    let source = input.third_program.as_ref().unwrap();
    assert_eq!(source.instruction_index, 5);
    assert_eq!(source.account_indices, signed.message.instructions()[5].accounts);
    assert_eq!(source.instruction_data, signed.message.instructions()[5].data);
    assert_eq!(source.accounts.len(), 26);

    for mode in ["full", "no_logs", "truncated_logs"] {
        let mut meta = meta.clone();
        if mode == "no_logs" {
            meta.log_messages.clear();
        }
        if mode == "truncated_logs" {
            meta.log_messages.truncate(meta.log_messages.len() / 2);
        }
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx.clone()),
                meta: Some(meta),
                index: 77,
                ..Default::default()
            }),
        };
        let grpc = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            123456,
            None,
            Some(&filter),
        );
        assert_eq!(grpc.len(), 1, "{mode}");
        let DexEvent::PumpFunTrade(fill) = &grpc[0] else { panic!("missing execution") };
        assert_eq!(fill.ix_name, input.ix_name, "{mode}");
        assert_eq!(fill.mint, input.mint);
        assert_eq!(fill.quote_mint, input.quote_mint);
        assert_eq!(fill.user, input.user);
        assert_eq!(fill.third_program, input.third_program);
        // Independently checked TradeEvent bytes and recipient token balance.
        // This curve is quoted in the intermediate token, not SOL.
        assert_eq!(
            (fill.sol_amount, fill.token_amount, fill.quote_amount),
            (0, 255_705_136_795_915, 11_728_308_720_756),
            "{mode}"
        );
    }
    for filter in [
        EventTypeFilter::include_only(vec![EventType::PumpSwapTrade]),
        EventTypeFilter::exclude_types(vec![EventType::PumpFunTrade]),
    ] {
        let mut events = Vec::new();
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter(
            &signed,
            signed.signatures[0],
            rpc.slot,
            0,
            0,
            Some(&filter),
            &mut events,
        );
        assert!(events.iter().all(|e| !matches!(e, DexEvent::PumpFunTrade(_))));
    }
}
