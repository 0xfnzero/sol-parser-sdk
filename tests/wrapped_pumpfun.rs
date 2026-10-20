use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    core::events::*,
    grpc::{EventType, EventTypeFilter},
    wrapped_pumpfun_parser::*,
    wrapped_pumpfun_rpc_analyzer::analyze_wrapped_pumpfun,
};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

#[test]
fn reported_and_independent_transactions_match_real_cpi_or_stay_raw() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/wrapped_pumpfun_reported.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    // Infer only from the originally reported Rapid transaction; all recent
    // signatures are held out and cannot change the rule under test.
    let training =
        cases.iter().find(|c| c["signature"].as_str().unwrap().starts_with("2r4uXUJ")).unwrap();
    let rpc = serde_json::from_value(training["wire_rpc"].clone()).unwrap();
    let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
    let original_rule =
        analyze_wrapped_pumpfun(&tx, &meta).into_iter().find_map(|a| a.candidate_rule).unwrap();
    let recent_training =
        cases.iter().find(|c| c["signature"].as_str().unwrap().starts_with("5rY6dX")).unwrap();
    let recent_rpc = serde_json::from_value(recent_training["wire_rpc"].clone()).unwrap();
    let (recent_meta, recent_tx) = sol_parser_sdk::convert_rpc_to_grpc(&recent_rpc).unwrap();
    let recent_rule = analyze_wrapped_pumpfun(&recent_tx, &recent_meta)
        .into_iter()
        .find_map(|a| a.candidate_rule)
        .unwrap();
    let rules = [original_rule, recent_rule];
    let pump_filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
    let other_filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]);
    let mut restored_count = 0;
    let mut held_out_count = 0;
    for c in cases {
        let rpc = serde_json::from_value(c["wire_rpc"].clone()).unwrap();
        let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
        let message = tx.message.as_ref().unwrap();
        let keys: Vec<Pubkey> = message
            .account_keys
            .iter()
            .chain(&meta.loaded_writable_addresses)
            .chain(&meta.loaded_readonly_addresses)
            .map(|k| Pubkey::try_from(k.as_slice()).unwrap())
            .collect();
        let update = yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(yellowstone_grpc_proto::prelude::SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx.clone()),
                meta: Some(meta.clone()),
                index: 77,
                ..Default::default()
            }),
        };
        let grpc = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            123456,
            None,
            Some(&pump_filter),
        );
        assert!(!grpc.is_empty());
        for e in grpc {
            let DexEvent::PumpFunTrade(t) = e else { panic!("unexpected Pump subscription event") };
            let source = t.third_program.as_ref().expect("gRPC wrapper provenance");
            let outer = &message.instructions[source.instruction_index];
            assert_eq!(source.program_id, keys[outer.program_id_index as usize]);
            assert_eq!(source.instruction_data, outer.data);
            assert_eq!(source.account_indices, outer.accounts);
            assert_eq!(
                source.accounts,
                outer.accounts.iter().map(|i| Some(keys[*i as usize])).collect::<Vec<_>>()
            );
            assert_eq!(t.parse_status, PumpFunParseStatus::ExecutionConfirmed);
            assert_eq!((t.metadata.tx_index, t.metadata.grpc_recv_us), (77, 123456));
        }
        assert!(sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update,
            123456,
            None,
            Some(&other_filter)
        )
        .is_empty());
        let wire = STANDARD.decode(c["wire_rpc"]["transaction"][0].as_str().unwrap()).unwrap();
        let signed: VersionedTransaction = wincode::deserialize_exact(&wire).unwrap();
        assert_eq!(signed.signatures[0].to_string(), c["signature"].as_str().unwrap());
        let loaded = |v: &[Vec<u8>]| {
            v.iter().map(|k| Pubkey::try_from(k.as_slice()).unwrap()).collect::<Vec<_>>()
        };
        let mut events = vec![];
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &signed,
            &loaded(&meta.loaded_writable_addresses),
            &loaded(&meta.loaded_readonly_addresses),
            signed.signatures[0],
            rpc.slot,
            77,
            123456,
            Some(&pump_filter),
            &mut events,
        )
        .unwrap();
        for event in events {
            let DexEvent::PumpFunTrade(candidate) = event else { continue };
            assert_eq!(candidate.parse_status, PumpFunParseStatus::Unresolved);
            assert_eq!((candidate.sol_amount, candidate.token_amount), (0, 0));
            let raw = Box::new(
                candidate.third_program.as_ref().unwrap().with_metadata(candidate.metadata.clone()),
            );
            let Some((rule, (data, accounts))) =
                rules.iter().find_map(|rule| restore_instruction(&raw, rule).map(|v| (rule, v)))
            else {
                continue;
            };
            let actual: Vec<_> = meta
                .inner_instructions
                .iter()
                .filter(|g| g.index as usize == raw.instruction_index)
                .flat_map(|g| &g.instructions)
                .filter(|ix| {
                    keys[ix.program_id_index as usize]
                        == sol_parser_sdk::instr::pump::PROGRAM_ID_PUBKEY
                        && ix.data.get(..8).and_then(pump_trade_name).is_some()
                })
                .collect();
            assert_eq!(actual.len(), 1);
            assert_eq!(data.as_slice(), actual[0].data, "{}", c["signature"]);
            assert_eq!(
                accounts,
                actual[0].accounts.iter().map(|i| keys[*i as usize]).collect::<Vec<_>>()
            );
            let DexEvent::PumpFunTrade(trade) =
                parse_wrapped_pumpfun_instruction(&raw, &rule, Some(&pump_filter)).unwrap()
            else {
                panic!()
            };
            assert_eq!(trade.ix_name, "buy_exact_quote_in_v2");
            assert_eq!(trade.parse_status, PumpFunParseStatus::InstructionDecoded);
            assert_eq!(
                trade.spendable_quote_in,
                u64::from_le_bytes(data[8..16].try_into().unwrap())
            );
            assert_eq!((trade.sol_amount, trade.token_amount, trade.quote_amount), (0, 0, 0));
            assert_eq!((trade.metadata.tx_index, trade.metadata.grpc_recv_us), (77, 123456));
            assert_eq!(trade.third_program.unwrap().program_id, raw.program_id);
            assert!(parse_wrapped_pumpfun_instruction(&raw, &rule, Some(&other_filter)).is_none());
            // Changed opcode, missing ALT, changed aliases and malformed rules
            // must fail closed without guessing, panicking, or fetching accounts.
            let mut bad = (*raw).clone();
            bad.instruction_data[0] ^= 1;
            assert!(restore_instruction(&bad, &rule).is_none());
            let mut bad = (*raw).clone();
            bad.accounts[0] = None;
            assert!(restore_instruction(&bad, &rule).is_none());
            let mut bad_rule = rule.clone();
            bad_rule.argument_offsets[0] = usize::MAX;
            assert!(restore_instruction(&raw, &bad_rule).is_none());
            let mut bad = (*raw).clone();
            bad.program_id = Pubkey::new_unique();
            assert!(restore_instruction(&bad, &rule).is_none());
            restored_count += 1;
            if c["signature"] != training["signature"]
                && c["signature"] != recent_training["signature"]
            {
                held_out_count += 1;
            }
        }
        let reports = analyze_wrapped_pumpfun(&tx, &meta);
        if c["signature"].as_str().unwrap().starts_with("5hKon")
            || c["signature"].as_str().unwrap().starts_with("3nMX")
        {
            assert!(!reports.is_empty());
            assert!(reports.iter().all(|r| r.candidate_rule.is_none()));
        }
    }
    assert!(restored_count >= 3);
    assert!(held_out_count >= 1, "must validate an independent signature");
    eprintln!("restored={restored_count}, held_out={held_out_count}");
}
