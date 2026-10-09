use base64::{engine::general_purpose::STANDARD, Engine};
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use sol_parser_sdk::{parse_rpc_transaction, DexEvent};

#[test]
fn signed_historical_wire_and_rpc_replay() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_replay_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize(&wire).unwrap();
        let count = case["expected"]["signature_count"].as_u64().unwrap() as usize;
        assert_eq!(tx.signatures.len(), count);
        assert_eq!(tx.signatures[0].to_string(), case["expected"]["signature"].as_str().unwrap());
        assert_eq!(tx.message.instructions().len(), case["expected"]["instruction_count"].as_u64().unwrap() as usize);
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        // Signature authentication is an independent test oracle, not a parser hot-path operation.
        let message = tx.message.serialize();
        for (i, sig) in tx.signatures.iter().enumerate() {
            let key = &tx.message.static_account_keys()[i];
            assert!(sig.verify(key.as_ref(), &message));
            let mut bad_message = message.clone();
            *bad_message.last_mut().unwrap() ^= 1;
            assert!(!sig.verify(key.as_ref(), &bad_message));
            let mut bad_signature = sig.as_ref().to_vec(); bad_signature[0] ^= 1;
            let bad_signature = solana_sdk::signature::Signature::try_from(bad_signature.as_slice()).unwrap();
            assert!(!bad_signature.verify(key.as_ref(), &message));
        }
        assert!(wincode::deserialize::<VersionedTransaction>(&wire[..wire.len()-1]).is_err());
        let mut rpc = case["rpc"].clone();
        // Feed the same verified wire to RPC parsing, preserving historical metadata.
        rpc["transaction"] = serde_json::json!([case["wire"], "base64"]);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(rpc).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        match case["name"].as_str().unwrap() {
            "pumpfun" => assert!(events.iter().any(|e| matches!(e, DexEvent::PumpFunBuy(t) if t.sol_amount == 977777777 && t.token_amount == 30765521374696))),
            "pumpswap" => assert!(events.iter().any(|e| matches!(e, DexEvent::PumpSwapBuy(t) if t.quote_amount_in == 10000000 && t.base_amount_out == 7317003080))),
            "generated_legacy_two_signers" => assert!(events.is_empty()),
            _ => panic!("unrecognized fixture"),
        }
    }
}

#[test]
fn signed_multi_alt_cpi_and_real_failed_mainnet_replay() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_dex_replay_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize(&wire).unwrap();
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        assert_eq!(tx.message.address_table_lookups().unwrap().len(), case["expected"]["lookup_count"].as_u64().unwrap() as usize);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(case["rpc"].clone()).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        if !case["expected"]["succeeded"].as_bool().unwrap() {
            assert!(events.is_empty());
            continue;
        }
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.metadata().signature == tx.signatures[0]));
        let swaps: Vec<_> = events.iter().filter_map(|e| match e {
            DexEvent::RaydiumCpmmSwap(t) => Some(("RaydiumCpmm", t.pool_id, t.input_token_mint, t.output_token_mint, t.input_amount, t.output_amount)),
            DexEvent::RaydiumClmmSwap(t) => Some(("RaydiumClmm", t.pool_state, t.input_mint, t.output_mint, if t.zero_for_one {t.amount_0} else {t.amount_1}, if t.zero_for_one {t.amount_1} else {t.amount_0})),
            DexEvent::OrcaWhirlpoolSwap(t) => Some(("OrcaWhirlpool", t.whirlpool, if t.a_to_b {t.token_mint_a} else {t.token_mint_b}, if t.a_to_b {t.token_mint_b} else {t.token_mint_a}, t.input_amount, t.output_amount)),
            DexEvent::MeteoraDlmmSwap(t) => Some(("MeteoraDlmm", t.pool, if t.swap_for_y {t.token_x_mint} else {t.token_y_mint}, if t.swap_for_y {t.token_y_mint} else {t.token_x_mint}, t.amount_in, t.amount_out)),
            _ => None,
        }).collect();
        assert_eq!(swaps.len(), 3);
        for (swap, expected) in swaps.iter().zip(case["expected"]["legs"].as_array().unwrap()) {
            assert_eq!(swap.0, expected["protocol"].as_str().unwrap());
            assert_eq!(swap.1.to_string(), expected["pool"].as_str().unwrap());
            if swap.0 == "RaydiumClmm" {
                // Legacy swap omits mint accounts; account_filler deliberately leaves them unknown.
                // Route adapters resolve its mints from token-balance metadata in the other SDKs.
                assert_eq!(swap.2, solana_sdk::pubkey::Pubkey::default());
                assert_eq!(swap.3, solana_sdk::pubkey::Pubkey::default());
            } else {
                assert_eq!(swap.2.to_string(), expected["input_mint"].as_str().unwrap());
                assert_eq!(swap.3.to_string(), expected["output_mint"].as_str().unwrap());
            }
            assert_eq!(swap.4, expected["specified_amount"].as_str().unwrap().parse::<u64>().unwrap());
            assert_eq!(swap.5, expected["actual_output_amount"].as_str().unwrap().parse::<u64>().unwrap());
        }
    }
}

#[test]
fn official_sanitizer_signed_wire_boundary_contract() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_wire_boundaries_20261009.json")).unwrap();
    let alt: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_alt_load_rejections_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap().iter().chain(alt["cases"].as_array().unwrap()) {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let decoded = wincode::deserialize::<VersionedTransaction>(&wire);
        // wincode accepts trailing bytes for stream decoding; these fixtures require a complete wire.
        let valid = decoded.as_ref().is_ok_and(|tx| tx.sanitize().is_ok() && wincode::serialize(tx).unwrap().len() == wire.len());
        assert_eq!(valid, case["valid_structure"].as_bool().unwrap(), "{}", case["name"]);
        if valid {
            let tx = decoded.unwrap();
            assert_eq!(tx.verify_with_results().into_iter().all(|ok| ok), case["valid_signature"].as_bool().unwrap(), "{}", case["name"]);
        }
    }
}

#[test]
fn signed_customizable_pool_public_parse_boundaries() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_clmm_boundaries_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize(&wire).unwrap();
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(case["rpc"].clone()).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        assert_eq!(events.len(), usize::from(case["valid_instruction"].as_bool().unwrap()), "{}", case["name"]);
    }
}

#[test]
fn captured_cpmm_signed_bank_fee_and_failure_replay() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_cpmm_bank_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize(&wire).unwrap();
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        let mut rpc = case["rpc"].clone();
        rpc["transaction"] = serde_json::json!([case["wire"], "base64"]);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(rpc).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        assert_eq!(events.len(), usize::from(case["succeeded"].as_bool().unwrap()), "{}", case["name"]);
        if let Some(DexEvent::RaydiumCpmmSwap(swap)) = events.first() {
            // CPMM event amounts are vault credits/debits; Token-2022 trader amounts differ.
            assert_eq!(swap.input_amount, case["vault_credit"].as_u64().unwrap());
            assert_eq!(swap.output_amount, case["vault_debit"].as_u64().unwrap());
            assert_eq!(swap.metadata.signature, tx.signatures[0]);
        } else { assert!(events.is_empty()); }
    }
}

#[test]
fn signed_same_pool_multileg_bank_preserves_invocation_settlement() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_multileg_bank_20261009.json")).unwrap();
    let identical: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_identical_alt_bank_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap().iter().chain(identical["cases"].as_array().unwrap()) {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize_exact(&wire).unwrap();
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        if let Some(tables) = case["lookup_tables"].as_array() {
            let lookups = tx.message.address_table_lookups().unwrap();
            for side in ["writable", "readonly"] {
                let expected: Vec<_> = lookups.iter().flat_map(|lookup| {
                    let table = tables.iter().find(|table| table["key"].as_str().unwrap() == lookup.account_key.to_string()).unwrap();
                    let indexes = if side == "writable" { &lookup.writable_indexes } else { &lookup.readonly_indexes };
                    indexes.iter().map(move |index| table["addresses"][usize::from(*index)].clone())
                }).collect();
                assert_eq!(case["rpc"]["meta"]["loadedAddresses"][side], serde_json::json!(expected));
            }
        }
        let mut rpc = case["rpc"].clone();
        rpc["transaction"] = serde_json::json!([case["wire"], "base64"]);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(rpc).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        let route = sol_parser_sdk::analyze_rpc_transaction_routes(&rpc, &[]).unwrap();
        let success = case["succeeded"].as_bool().unwrap();
        assert_eq!(route.succeeded, success);
        assert_eq!(route.legs.len(), 2);
        assert_eq!(route.legs[0].pool, route.legs[1].pool);
        assert_eq!(route.legs[0].trader, route.legs[1].trader);
        if let Some(identity) = case["identity"].as_object() {
            for leg in &route.legs {
                assert_eq!(leg.pool.to_string(), identity["pool"].as_str().unwrap());
                assert_eq!(leg.trader.to_string(), identity["trader"].as_str().unwrap());
                assert_eq!(leg.input_account.to_string(), identity["input_account"].as_str().unwrap());
                assert_eq!(leg.output_account.to_string(), identity["output_account"].as_str().unwrap());
                assert_eq!(leg.input_mint.unwrap().to_string(), identity["input_mint"].as_str().unwrap());
                assert_eq!(leg.output_mint.unwrap().to_string(), identity["output_mint"].as_str().unwrap());
            }
        }
        assert_eq!(events.len(), if success { 2 } else { 0 });
        for (i, leg) in route.legs.iter().enumerate() {
            assert_eq!(leg.position.outer_index, (i + 1) as u32);
            assert_eq!(leg.specified_amount, case["requested_gross_inputs"][i].as_u64().unwrap());
            if success {
                let expected = &case["legs"][i];
                assert_eq!(leg.actual_input_amount, expected["gross_input"].as_u64());
                assert_eq!(leg.actual_output_amount, if case["fee_output_legs"][i].as_bool().unwrap() { None } else { expected["net_output"].as_u64() });
                let DexEvent::RaydiumCpmmSwap(swap) = &events[i] else { panic!("CPMM event") };
                assert_eq!(swap.input_amount, expected["vault_credit"].as_u64().unwrap());
                assert_eq!(swap.output_amount, expected["vault_debit"].as_u64().unwrap());
            } else {
                assert_eq!(leg.actual_input_amount, None);
                assert_eq!(leg.actual_output_amount, None);
            }
        }
    }
}

#[test]
fn actual_signed_nested_cpi_bank_failure_cannot_invent_settlement() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_nested_cpi_bank_20261009.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["wire"].as_str().unwrap()).unwrap();
        let tx: VersionedTransaction = wincode::deserialize_exact(&wire).unwrap();
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        let mut rpc = case["rpc"].clone();
        rpc["transaction"] = serde_json::json!([case["wire"], "base64"]);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(rpc).unwrap();
        let events = parse_rpc_transaction(&rpc, None).unwrap();
        let route = sol_parser_sdk::analyze_rpc_transaction_routes(&rpc, &[]).unwrap();
        let success = case["succeeded"].as_bool().unwrap();
        assert_eq!(route.succeeded, success);
        assert_eq!(route.legs.len(), 1);
        let leg = &route.legs[0];
        assert_eq!(leg.position.stack_height, Some(if success { 2 } else { 3 }));
        assert_eq!(leg.specified_amount, 10001);
        assert_eq!(events.len(), usize::from(success));
        if success {
            assert_eq!(leg.actual_input_amount, Some(10001));
            assert_eq!(leg.actual_output_amount, Some(468207));
            assert_eq!(case["token_deltas"][leg.input_account.to_string()].as_i64(), Some(-10001));
            assert_eq!(case["token_deltas"][leg.output_account.to_string()].as_u64(), Some(468207));
            let DexEvent::RaydiumCpmmSwap(swap) = &events[0] else {panic!("CPMM")};
            assert_eq!((swap.input_amount, swap.output_amount), (9800, 468207));
            assert_eq!(swap.metadata.signature, tx.signatures[0]);
        } else {
            assert_eq!(leg.actual_input_amount, None);
            assert_eq!(leg.actual_output_amount, None);
            assert!(case["token_deltas"].as_object().unwrap().values().all(|v| v.as_i64() == Some(0)));
        }
        // Successful SPL calls remain in failed execution metadata; never treat them as committed.
        assert!(case["rpc"]["meta"]["logMessages"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().ends_with(" success") && v.as_str().unwrap().contains("Tokenkeg")));
    }
}

#[test]
fn mixed_signed_legacy_v0_legacy_framed_stream_isolation() {
    let legacy: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_replay_20261009.json")).unwrap();
    let bank: serde_json::Value = serde_json::from_str(include_str!("fixtures/signed_identical_alt_bank_20261009.json")).unwrap();
    let legacy = legacy["cases"].as_array().unwrap().iter().find(|c| c["name"] == "generated_legacy_two_signers").unwrap();
    let v0 = bank["cases"].as_array().unwrap().iter().find(|c| c["name"] == "identical-intents-two-alt").unwrap();
    let cases = [legacy, v0, legacy];
    let wires: Vec<_> = cases.iter().map(|c| STANDARD.decode(c["wire"].as_str().unwrap()).unwrap()).collect();
    let stream: Vec<_> = wires.iter().flatten().copied().collect();
    assert!(wincode::deserialize_exact::<VersionedTransaction>(&stream).is_err());
    let mut offset = 0;
    for (i, case) in cases.iter().enumerate() {
        let tx: VersionedTransaction = wincode::deserialize(&stream[offset..]).unwrap();
        tx.sanitize().unwrap();
        assert_eq!(tx.signatures.len(), 2);
        assert!(tx.verify_with_results().into_iter().all(|valid| valid));
        let encoded = wincode::serialize(&tx).unwrap();
        assert_eq!(encoded, wires[i]);
        assert_eq!(tx.message.address_table_lookups().map(|t| t.len()).unwrap_or(0), if i == 1 { 2 } else { 0 });
        let mut rpc = case["rpc"].clone();
        rpc["transaction"] = serde_json::json!([case["wire"], "base64"]);
        let rpc: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(rpc).unwrap();
        assert_eq!(parse_rpc_transaction(&rpc, None).unwrap().len(), if i == 1 { 2 } else { 0 });
        offset += encoded.len();
    }
    assert_eq!(offset, stream.len());
    let damaged = &stream[..stream.len() - 1];
    assert!(wincode::deserialize::<VersionedTransaction>(damaged).is_ok());
    assert!(wincode::deserialize::<VersionedTransaction>(&damaged[wires[0].len()..]).is_ok());
    assert!(wincode::deserialize::<VersionedTransaction>(&damaged[wires[0].len() + wires[1].len()..]).is_err());
}
