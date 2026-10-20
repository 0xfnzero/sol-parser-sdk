use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use sol_parser_sdk::{
    core::events::{DexEvent, PumpFunParseStatus},
    grpc::{EventType, EventTypeFilter},
    shredstream::parse_transaction_dex_events_with_loaded_addresses,
};
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction};

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/shred_routes/raw.json")).unwrap()
}
fn keys(f: &Value, name: &str) -> Vec<Pubkey> {
    f[name].as_array().unwrap().iter().map(|k| k.as_str().unwrap().parse().unwrap()).collect()
}
fn parse(tx: &VersionedTransaction, f: &Value, filter: Option<&EventTypeFilter>) -> Vec<DexEvent> {
    let mut events = vec![];
    parse_transaction_dex_events_with_loaded_addresses(
        tx,
        &keys(f, "loaded_writable"),
        &keys(f, "loaded_readonly"),
        tx.signatures[0],
        f["slot"].as_u64().unwrap(),
        1,
        2,
        filter,
        &mut events,
    )
    .unwrap();
    events
}
#[test]
fn signed_wires_preserve_every_account_and_byte_without_execution_metadata() {
    for f in fixtures() {
        let tx: VersionedTransaction =
            wincode::deserialize_exact(&STANDARD.decode(f["wire"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(tx.signatures[0].to_string(), f["signature"].as_str().unwrap());
        for (sig, key) in tx.signatures.iter().zip(tx.message.static_account_keys()) {
            assert!(sig.verify(key.as_ref(), &tx.message.serialize()));
        }
        let mut all = tx.message.static_account_keys().to_vec();
        all.extend(keys(&f, "loaded_writable"));
        all.extend(keys(&f, "loaded_readonly"));
        let filter = EventTypeFilter::include_only(vec![EventType::RawInstruction]);
        let events = parse(&tx, &f, Some(&filter));
        assert_eq!(events.len(), tx.message.instructions().len());
        for (event, ix) in events.iter().zip(tx.message.instructions()) {
            let DexEvent::RawInstruction(raw) = event else { panic!() };
            assert_eq!(raw.metadata.tx_index, 1);
            assert_eq!(raw.metadata.grpc_recv_us, 2);
            assert_eq!(raw.program_id, all[ix.program_id_index as usize]);
            assert_eq!(
                raw.accounts,
                ix.accounts.iter().map(|i| Some(all[*i as usize])).collect::<Vec<_>>()
            );
            assert_eq!(raw.account_indices, ix.accounts);
            assert_eq!(raw.instruction_data, ix.data);
            let json = serde_json::to_value(event).unwrap();
            let restored: DexEvent = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(serde_json::to_value(restored).unwrap(), json);
            assert!(json["RawInstruction"].get("is_buy").is_none());
            assert!(json["RawInstruction"].get("ix_name").is_none());
        }
        if let Some(count) = f["raw_instruction_count"].as_u64() {
            let default = parse(&tx, &f, None);
            assert_eq!(
                default
                    .iter()
                    .filter(|e| matches!(
                        e,
                        DexEvent::RawInstruction(_) | DexEvent::PumpFunTrade(_)
                    ))
                    .count(),
                count as usize
            );
            for e in &default {
                if let DexEvent::PumpFunTrade(t) = e {
                    assert_eq!(t.parse_status, PumpFunParseStatus::Unresolved);
                    assert_eq!((t.sol_amount, t.token_amount), (0, 0));
                    assert!(t.ix_name.is_empty());
                }
            }
        }
        if f["signature"].as_str().unwrap().starts_with("3nMX")
            || f["signature"].as_str().unwrap().starts_with("2r4u")
        {
            let default = parse(&tx, &f, None);
            assert_eq!(
                default.len(),
                if f["signature"].as_str().unwrap().starts_with("3nMX") { 2 } else { 3 }
            );
            assert!(default
                .iter()
                .all(|e| matches!(e, DexEvent::RawInstruction(_) | DexEvent::PumpFunTrade(_))));
            let selected =
                parse(&tx, &f, Some(&EventTypeFilter::include_only(vec![EventType::PumpFunTrade])));
            assert!(!selected.is_empty());
            assert!(selected.iter().all(|e| matches!(e, DexEvent::PumpFunTrade(t) if t.parse_status == PumpFunParseStatus::Unresolved && t.third_program.is_some())));
            for direction in
                [EventType::PumpFunBuy, EventType::PumpFunSell, EventType::PumpFunBuyExactSolIn]
            {
                assert!(parse(&tx, &f, Some(&EventTypeFilter::include_only(vec![direction])))
                    .is_empty());
            }
        }
    }
}

#[test]
fn unknown_programs_changed_layouts_and_unresolved_accounts_stay_raw() {
    let f = &fixtures()[0];
    let mut tx: VersionedTransaction =
        wincode::deserialize_exact(&STANDARD.decode(f["wire"].as_str().unwrap()).unwrap()).unwrap();
    let instructions = match &mut tx.message {
        VersionedMessage::Legacy(m) => &mut m.instructions,
        VersionedMessage::V0(m) => &mut m.instructions,
        VersionedMessage::V1(m) => &mut m.instructions,
    };
    let mut ix = instructions[1].clone();
    ix.program_id_index = 0; // An arbitrary program, not the known wrapper.
    ix.accounts.reverse();
    ix.accounts.push(ix.accounts[0]); // Duplicate remains in place.
    ix.accounts.push(250); // Unresolved: preserve index and None.
    ix.data = vec![255, 0, 1, 2, 3]; // No recognized discriminator or fixed length.
    *instructions = vec![ix.clone()];
    let mut events = vec![];
    sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter(
        &tx,
        tx.signatures[0],
        1,
        2,
        3,
        Some(&EventTypeFilter::include_only(vec![EventType::RawInstruction])),
        &mut events,
    );
    let [DexEvent::RawInstruction(raw)] = &events[..] else { panic!() };
    assert_eq!(raw.instruction_data, ix.data);
    assert_eq!(raw.account_indices, ix.accounts);
    assert_eq!(raw.accounts.last(), Some(&None));
    assert_eq!(raw.accounts[0], raw.accounts[raw.accounts.len() - 2]);
    assert_eq!(raw.accounts.len(), ix.accounts.len());
}

#[test]
fn raw_dex_subscriptions_are_instruction_local_and_never_infer_pool_layout() {
    use sol_parser_sdk::grpc::program_ids::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        message::Message,
        signature::Signature,
    };
    let wrapper = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let pairs = [
        (PUMPFUN_PROGRAM, EventType::PumpFunTrade, "pumpfun"),
        (PUMPSWAP_PROGRAM, EventType::PumpSwapTrade, "pumpswap"),
        (RAYDIUM_LAUNCHLAB_PROGRAM, EventType::RaydiumLaunchlabTrade, "raydium_launchlab"),
        (RAYDIUM_CPMM_PROGRAM, EventType::RaydiumCpmmSwap, "raydium_cpmm"),
        (RAYDIUM_CLMM_PROGRAM, EventType::RaydiumClmmSwap, "raydium_clmm"),
        (RAYDIUM_AMM_V4_PROGRAM, EventType::RaydiumAmmV4Swap, "raydium_amm_v4"),
        (ORCA_WHIRLPOOL_PROGRAM, EventType::OrcaWhirlpoolSwap, "orca_whirlpool"),
        (METEORA_POOLS_PROGRAM, EventType::MeteoraPoolsSwap, "meteora_pools"),
        (METEORA_DAMM_V2_PROGRAM, EventType::MeteoraDammV2Swap, "meteora_damm_v2"),
        (METEORA_DLMM_PROGRAM, EventType::MeteoraDlmmSwap, "meteora_dlmm"),
        (METEORA_DBC_PROGRAM, EventType::MeteoraDbcSwap, "meteora_dbc"),
    ];
    for (program, event_type, name) in pairs {
        let tx = VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::new(
                &[
                    Instruction {
                        program_id: wrapper,
                        accounts: vec![
                            AccountMeta::new_readonly(program, false),
                            AccountMeta::new_readonly(program, false),
                        ],
                        data: vec![255],
                    },
                    // This instruction can see none of the DEX accounts, although the transaction can.
                    Instruction { program_id: wrapper, accounts: vec![], data: vec![254] },
                ],
                Some(&payer),
            )),
        };
        let mut events = vec![];
        let filter = EventTypeFilter::include_only(vec![event_type]);
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter(
            &tx,
            tx.signatures[0],
            1,
            0,
            0,
            Some(&filter),
            &mut events,
        );
        if name == "pumpfun" {
            let [DexEvent::PumpFunTrade(t)] = &events[..] else { panic!("{events:?}") };
            assert_eq!(t.parse_status, PumpFunParseStatus::Unresolved);
            let raw = t.third_program.as_ref().unwrap();
            assert_eq!(raw.instruction_index, 0);
            assert_eq!(raw.accounts, vec![Some(program), Some(program)]);
            assert!(filter.should_include_dex_event(&events[0]));
            assert!(!EventTypeFilter::include_only(vec![EventType::PumpFunSell])
                .should_include_dex_event(&events[0]));
            continue;
        }
        let [DexEvent::RawInstruction(raw)] = &events[..] else { panic!("{name}: {events:?}") };
        assert_eq!(raw.instruction_index, 0);
        assert_eq!(raw.dexes.len(), 1);
        assert_eq!(raw.dexes[0].dex_type.as_str(), name);
        assert_eq!(raw.dexes[0].pool, None);
        assert_eq!(raw.accounts.len(), 2);
        assert!(filter.should_include_dex_event(&events[0]));
        let unrelated = EventTypeFilter::include_only(vec![if name == "pumpfun" {
            EventType::RaydiumCpmmSwap
        } else {
            EventType::PumpFunTrade
        }]);
        assert!(!unrelated.should_include_dex_event(&events[0]));
        let exclude = EventTypeFilter {
            include_only: Some(vec![event_type]),
            exclude_types: Some(vec![EventType::RawInstruction]),
        };
        assert!(!exclude.should_include_dex_event(&events[0]));
    }
}

#[test]
fn raw_multi_dex_and_alt_boundaries() {
    use sol_parser_sdk::core::events::{raw_instruction_dexes, raw_instruction_matches_filter};
    use sol_parser_sdk::grpc::program_ids::*;
    let dexes = raw_instruction_dexes(
        Pubkey::new_unique(),
        &[Some(PUMPFUN_PROGRAM), None, Some(RAYDIUM_CPMM_PROGRAM), Some(PUMPFUN_PROGRAM)],
    );
    assert_eq!(dexes.len(), 2);
    for event in [EventType::PumpFunTrade, EventType::RaydiumCpmmSwap] {
        assert!(raw_instruction_matches_filter(
            &dexes,
            &EventTypeFilter::include_only(vec![event])
        ));
    }
    assert!(!raw_instruction_matches_filter(
        &dexes,
        &EventTypeFilter::include_only(vec![EventType::PumpFunBuy])
    ));
    assert!(!raw_instruction_matches_filter(
        &[],
        &EventTypeFilter::include_only(vec![EventType::PumpFunTrade])
    ));
    assert!(raw_instruction_matches_filter(
        &[],
        &EventTypeFilter::include_only(vec![EventType::RawInstruction])
    ));
}

#[test]
fn unresolved_candidates_preserve_missing_keys_and_respect_exclusion() {
    use sol_parser_sdk::core::events::PumpFunParseStatus;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        message::Message,
        signature::Signature,
    };
    let wrapper = Pubkey::new_unique();
    let pump = sol_parser_sdk::grpc::program_ids::PUMPFUN_PROGRAM;
    let mut message = Message::new(
        &[Instruction {
            program_id: wrapper,
            accounts: vec![AccountMeta::new_readonly(pump, false)],
            data: vec![7, 9, 11],
        }],
        Some(&Pubkey::new_unique()),
    );
    message.instructions[0].accounts.push(255);
    let tx = VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::Legacy(message),
    };
    let parse = |filter: Option<&EventTypeFilter>| {
        let mut out = vec![];
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_filter(
            &tx,
            tx.signatures[0],
            12,
            34,
            56,
            filter,
            &mut out,
        );
        out
    };
    let events = parse(None);
    let [DexEvent::PumpFunTrade(t)] = &events[..] else { panic!("{events:?}") };
    assert_eq!(t.parse_status, PumpFunParseStatus::Unresolved);
    assert_eq!((t.metadata.slot, t.metadata.tx_index, t.metadata.grpc_recv_us), (12, 34, 56));
    let raw = t.third_program.as_ref().unwrap();
    assert_eq!(raw.accounts, vec![Some(pump), None]);
    assert_eq!(raw.instruction_data, vec![7, 9, 11]);
    let json = serde_json::to_value(t).unwrap();
    assert!(json["third_program"].get("metadata").is_none());
    assert!(json["third_program"].get("dexes").is_none());
    assert_eq!(
        serde_json::from_value::<sol_parser_sdk::core::events::PumpFunTradeEvent>(json)
            .unwrap()
            .third_program
            .as_ref(),
        Some(raw)
    );
    for filter in [
        EventTypeFilter::include_only(vec![EventType::PumpFunBuy]),
        EventTypeFilter::include_only(vec![EventType::PumpFunSell]),
        EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]),
        EventTypeFilter::exclude_types(vec![EventType::PumpFunTrade]),
        EventTypeFilter {
            include_only: Some(vec![EventType::PumpFunTrade]),
            exclude_types: Some(vec![EventType::PumpFunTrade]),
        },
    ] {
        assert!(parse(Some(&filter)).is_empty(), "{filter:?}");
    }
}
