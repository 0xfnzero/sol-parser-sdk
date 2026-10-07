#[path = "support/rpc_corpus.rs"]
mod corpus;

#[test]
fn independent_oracle_rejects_corrupted_real_transaction_events() {
    let tx = serde_json::from_str(&corpus::fixture("cpmm_0")).unwrap();
    let events = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
    let original: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&events).unwrap()).unwrap();
    corpus::verify("cpmm_0", &original, false);
    let index = original
        .as_array()
        .unwrap()
        .iter()
        .position(|event| event.get("RaydiumCpmmSwap").is_some())
        .unwrap();
    let mut bad_amount = original.clone();
    bad_amount[index]["RaydiumCpmmSwap"]["output_transfer_fee"] = serde_json::json!(0);
    let mut duplicate = original.clone();
    duplicate.as_array_mut().unwrap().push(original[index].clone());
    let mut bad_account = original.clone();
    bad_account[index]["RaydiumCpmmSwap"]["input_vault"] = serde_json::json!(vec![0; 32]);
    for corrupted in [bad_amount, duplicate, bad_account, serde_json::json!([])] {
        assert!(std::panic::catch_unwind(|| corpus::verify("cpmm_0", &corrupted, false)).is_err());
    }
}

#[test]
fn real_rpc_corpus_matches_wire_accounts_movements_and_balances() {
    let mut checked = 0;
    for name in corpus::CASES {
        let tx = serde_json::from_str(&corpus::fixture(name)).unwrap();
        let parsed = sol_parser_sdk::parse_rpc_transaction_with_cost(&tx, None).unwrap();
        let events = serde_json::from_str(&serde_json::to_string(&parsed.events).unwrap()).unwrap();
        checked += corpus::verify(name, &events, false);
        assert_eq!(
            parsed.cost.transaction_fee_lamports,
            tx.transaction.meta.as_ref().map(|m| m.fee)
        );
    }
    assert_eq!(checked, 17);
}

#[test]
fn failed_real_rpc_has_cost_but_no_rolled_back_dex_events() {
    let tx = serde_json::from_str(&corpus::fixture("failed_route")).unwrap();
    let (meta, _) = sol_parser_sdk::convert_rpc_to_grpc(&tx).unwrap();
    assert!(meta.err.is_some());
    assert!(sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap().is_empty());
    let parsed = sol_parser_sdk::parse_rpc_transaction_with_cost(&tx, None).unwrap();
    assert!(parsed.events.is_empty());
    assert_eq!(parsed.cost.transaction_fee_lamports, tx.transaction.meta.as_ref().map(|m| m.fee));
    assert_eq!(sol_parser_sdk::parse_rpc_transaction_cost(&tx).unwrap(), parsed.cost);
}

#[test]
fn exact_filters_match_real_corpus_event_selection() {
    use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
    for name in corpus::CASES {
        let tx = serde_json::from_str(&corpus::fixture(name)).unwrap();
        let full = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
        for kind in [
            EventType::RaydiumCpmmSwap,
            EventType::RaydiumCpmmCollectCreatorFee,
            EventType::PumpSwapSell,
            EventType::PumpSwapBuy,
            EventType::PumpFunBuy,
            EventType::PumpFunSell,
            EventType::MeteoraDlmmSwap,
        ] {
            let filter = EventTypeFilter::include_only(vec![kind]);
            let expected = full
                .iter()
                .filter(|e| filter.should_include_dex_event(e))
                .cloned()
                .collect::<Vec<_>>();
            let actual = sol_parser_sdk::parse_rpc_transaction(&tx, Some(&filter)).unwrap();
            let canonical = |mut events: Vec<sol_parser_sdk::DexEvent>| {
                for event in &mut events {
                    event.metadata_mut().unwrap().grpc_recv_us = 0;
                }
                serde_json::to_string(&events).unwrap()
            };
            assert_eq!(canonical(actual), canonical(expected), "{name} {kind:?}");
        }
    }
}

#[test]
fn real_rpc_and_yellowstone_parallel_sequential_paths_agree() {
    use yellowstone_grpc_proto::prelude::{
        SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
    };
    for name in corpus::CASES {
        let tx = serde_json::from_str(&corpus::fixture(name)).unwrap();
        let rpc = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
        let (meta, transaction) = sol_parser_sdk::convert_rpc_to_grpc(&tx).unwrap();
        let update = SubscribeUpdateTransaction {
            slot: tx.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: transaction.signatures[0].clone(),
                transaction: Some(transaction),
                meta: Some(meta),
                ..Default::default()
            }),
        };
        let time = tx.block_time.map(|seconds| seconds * 1_000_000);
        let parallel =
            sol_parser_sdk::grpc::parse_subscribe_update_transaction(&update, 0, time, None);
        let sequential = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update, 0, time, None,
        );
        let canonical = |mut events: Vec<sol_parser_sdk::DexEvent>| {
            for event in &mut events {
                event.metadata_mut().unwrap().grpc_recv_us = 0;
            }
            let mut rows: Vec<_> =
                events.iter().map(|e| serde_json::to_string(e).unwrap()).collect();
            rows.sort();
            rows
        };
        assert_eq!(canonical(rpc), canonical(parallel.clone()), "{name}: RPC/gRPC");
        assert_eq!(canonical(parallel), canonical(sequential), "{name}: parallel/sequential");
    }
}

#[allow(dead_code)]
#[path = "../examples/support/rpc_capture.rs"]
mod capture;

#[test]
fn captured_base64_and_json_encodings_agree_and_reject_wire_corruption() {
    for name in corpus::CASES {
        let tx = serde_json::from_str(&corpus::fixture(name)).unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/rpc_corpus/{name}_wire.json"));
        let wire: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        capture::verify_wire(&tx, &wire).unwrap();
        let mut bad_slot = wire.clone();
        bad_slot["slot"] = serde_json::json!(0);
        let mut bad_key = wire.clone();
        bad_key["transaction"]["message"]["accountKeys"][0] =
            serde_json::json!("11111111111111111111111111111111");
        let mut bad_data = wire.clone();
        bad_data["transaction"]["message"]["instructions"][0]["data"] = serde_json::json!("1");
        let mut bad_loaded_keys = wire.clone();
        bad_loaded_keys["meta"]["loadedAddresses"] =
            serde_json::json!({"writable":[],"readonly":[]});
        for corrupted in [bad_slot, bad_key, bad_data] {
            assert!(capture::verify_wire(&tx, &corrupted).is_err(), "{name}: corruption accepted");
        }
        if bad_loaded_keys["meta"]["loadedAddresses"] != wire["meta"]["loadedAddresses"] {
            assert!(capture::verify_wire(&tx, &bad_loaded_keys).is_err());
        }
    }
}

#[test]
fn real_orca_price_limit_preserves_full_u128_precision() {
    let wire_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rpc_corpus/cpmm_0_wire.json");
    let wire: serde_json::Value =
        serde_json::from_slice(&std::fs::read(wire_path).unwrap()).unwrap();
    let mut keys = wire["transaction"]["message"]["accountKeys"].as_array().unwrap().clone();
    keys.extend(wire["meta"]["loadedAddresses"]["writable"].as_array().unwrap().iter().cloned());
    keys.extend(wire["meta"]["loadedAddresses"]["readonly"].as_array().unwrap().iter().cloned());
    let raw = wire["meta"]["innerInstructions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["instructions"].as_array().unwrap())
        .find(|ix| {
            keys[ix["programIdIndex"].as_u64().unwrap() as usize]
                == "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        })
        .unwrap();
    let data = solana_sdk::bs58::decode(raw["data"].as_str().unwrap()).into_vec().unwrap();
    let expected = u128::from_le_bytes(data[24..40].try_into().unwrap());
    assert!(expected > u128::from(u64::MAX));
    let tx = serde_json::from_str(&corpus::fixture("cpmm_0")).unwrap();
    let events = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
    let event = events
        .iter()
        .find_map(|event| match event {
            sol_parser_sdk::DexEvent::OrcaWhirlpoolSwap(swap) => Some(swap),
            _ => None,
        })
        .unwrap();
    assert_eq!(event.sqrt_price_limit, expected);
}

#[test]
fn real_clmm_instruction_accounts_survive_missing_logs() {
    let mut payload: serde_json::Value =
        serde_json::from_str(&corpus::fixture("dlmm_route")).unwrap();
    payload["meta"]["logMessages"] = serde_json::json!([]);
    let tx = serde_json::from_value(payload).unwrap();
    let events = sol_parser_sdk::parse_rpc_transaction(&tx, None).unwrap();
    let event = events
        .iter()
        .find_map(|event| match event {
            sol_parser_sdk::DexEvent::RaydiumClmmSwap(swap) => Some(swap),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        event.input_token_account,
        "7tZ1fHSRmwbBQXT3fgNrzQNFufJx36yBJswgpTzyKTHs".parse().unwrap()
    );
    assert_eq!(
        event.output_token_account,
        "2mi8e3FM7iAqAKnLFJ3TXabPTm9ZrWB58GGpoqiVszg2".parse().unwrap()
    );
    assert!(event.is_base_input);
    assert_eq!(event.other_amount_threshold, 0);
    assert_eq!(event.sqrt_price_limit_x64, 0);
    assert_eq!(event.ix_name, "swap");
}

#[test]
fn discovery_matches_wire_program_and_discriminator_without_logs() {
    let load = |name: &str| -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/rpc_corpus/{name}_wire.json"));
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    for (name, program, disc) in [
        (
            "cpmm_0",
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
            [143, 190, 90, 218, 196, 30, 51, 222],
        ),
        (
            "cpmm_collect",
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
            [20, 22, 86, 123, 198, 28, 219, 132],
        ),
        (
            "pumpfun_sell_v2",
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
            [93, 246, 130, 60, 231, 233, 64, 178],
        ),
    ] {
        let mut wire = load(name);
        wire["meta"]["logMessages"] = serde_json::Value::Null;
        assert!(capture::matches_instruction(&wire, program, &disc).unwrap());
        assert!(!capture::matches_instruction(&wire, "11111111111111111111111111111111", &disc)
            .unwrap());
        assert!(!capture::matches_instruction(&wire, program, &[0; 8]).unwrap());
        wire["transaction"]["message"]["instructions"][0]["programIdIndex"] =
            serde_json::json!(999999);
        assert!(capture::matches_instruction(&wire, program, &disc).is_err());
    }
    let mut wire = load("dlmm_0");
    wire["meta"]["logMessages"] = serde_json::json!(["Program log: Instruction: Swap"]);
    assert!(!capture::matches_instruction(
        &wire,
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
        &[248, 198, 158, 145, 225, 117, 135, 200]
    )
    .unwrap());
}
