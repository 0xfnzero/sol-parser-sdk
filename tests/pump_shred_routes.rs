use sol_parser_sdk as sdk;
use solana_sdk::{
    hash::Hash,
    message::Message as WireMessage,
    signature::{Keypair, Signer},
};
use solana_sdk::{
    message::{compiled_instruction::CompiledInstruction, VersionedMessage},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use sdk::core::events::{DexEvent, PumpRouteKind};
use sdk::grpc::{EventType, EventTypeFilter};
use sdk::shredstream::{
    parse_transaction_dex_events_with_filter, parse_transaction_dex_events_with_loaded_addresses,
};
use serde_json::Value;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/shred_routes/reported.json")).unwrap()
}
fn transaction(f: &Value) -> VersionedTransaction {
    let tx: VersionedTransaction =
        wincode::deserialize_exact(&STANDARD.decode(f["wire"].as_str().unwrap()).unwrap()).unwrap();
    let required = tx.message.header().num_required_signatures as usize;
    assert!(required > 0);
    assert_eq!(tx.signatures.len(), required);
    assert_eq!(tx.signatures[0].to_string(), f["signature"].as_str().unwrap());
    for (signature, key) in tx.signatures.iter().zip(tx.message.static_account_keys()) {
        assert!(signature.verify(key.as_ref(), &tx.message.serialize()));
    }
    tx
}
fn addresses(f: &Value, name: &str) -> Vec<Pubkey> {
    f[name].as_array().unwrap().iter().map(|s| s.as_str().unwrap().parse().unwrap()).collect()
}
fn resolved(
    tx: &VersionedTransaction,
    f: &Value,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();
    parse_transaction_dex_events_with_loaded_addresses(
        tx,
        &addresses(f, "loaded_writable"),
        &addresses(f, "loaded_readonly"),
        tx.signatures[0],
        f["slot"].as_u64().unwrap(),
        888,
        999,
        filter,
        &mut events,
    )
    .unwrap();
    events
}
fn keys(tx: &VersionedTransaction, f: &Value) -> Vec<Pubkey> {
    let mut keys = tx.message.static_account_keys().to_vec();
    keys.extend(addresses(f, "loaded_writable"));
    keys.extend(addresses(f, "loaded_readonly"));
    keys
}
fn instructions_mut(tx: &mut VersionedTransaction) -> &mut Vec<CompiledInstruction> {
    match &mut tx.message {
        VersionedMessage::Legacy(m) => &mut m.instructions,
        VersionedMessage::V0(m) => &mut m.instructions,
        VersionedMessage::V1(m) => &mut m.instructions,
    }
}

#[test]
fn route_trade_hints_are_optional_and_survive_serialization() {
    let mut seen = std::collections::HashSet::new();
    for f in fixtures() {
        for event in resolved(&transaction(&f), &f, None) {
            let DexEvent::PumpRouteIntent(intent) = event else {
                panic!("expected route intent");
            };
            seen.insert(format!("{:?}", intent.kind));
            match intent.kind {
                PumpRouteKind::RapidBuy => {
                    assert_eq!(intent.is_buy, Some(true));
                    assert_eq!(intent.ix_name.as_deref(), Some("buy_exact_quote_in_v2"));
                    assert_eq!(intent.hops.len(), 1);
                    assert_eq!(
                        &intent.instruction_data[17..25],
                        &sdk::instr::pump::discriminators::BUY_EXACT_QUOTE_IN_V2
                    );
                }
                PumpRouteKind::BuyPumpDelta => {
                    assert_eq!(intent.is_buy, Some(true));
                    assert_eq!(intent.ix_name, None);
                    assert_eq!(intent.amount_in, None);
                    assert_eq!(intent.hops.len(), 1);
                }
                PumpRouteKind::MultiHopSwap | PumpRouteKind::MultiHopCurveSwap => {
                    assert_eq!(intent.is_buy, None);
                    assert_eq!(intent.ix_name, None);
                }
            }
            let mut json = serde_json::to_value(&intent).unwrap();
            let restored: sdk::core::events::PumpRouteIntentEvent =
                serde_json::from_value(json.clone()).unwrap();
            assert_eq!(restored.is_buy, intent.is_buy);
            assert_eq!(restored.ix_name, intent.ix_name);
            // Previously persisted events have no hints: unknown must stay unknown.
            json.as_object_mut().unwrap().remove("is_buy");
            json.as_object_mut().unwrap().remove("ix_name");
            let legacy: sdk::core::events::PumpRouteIntentEvent =
                serde_json::from_value(json).unwrap();
            assert_eq!(legacy.is_buy, None);
            assert_eq!(legacy.ix_name, None);
        }
    }
    assert_eq!(
        seen,
        ["RapidBuy", "BuyPumpDelta", "MultiHopSwap"].into_iter().map(str::to_owned).collect()
    );
}

#[test]
fn six_signed_reported_transactions_replay_without_rpc() {
    for f in fixtures() {
        let tx = transaction(&f);
        let expected = f["expected"].as_array().unwrap();
        let events = resolved(&tx, &f, None);
        assert_eq!(events.len(), expected.len(), "{}", f["signature"]);
        for (event, expected) in events.iter().zip(expected) {
            let DexEvent::PumpRouteIntent(intent) = event else {
                panic!("expected explicit intent, not a fill")
            };
            assert_eq!(format!("{:?}", intent.kind), expected["kind"].as_str().unwrap());
            assert_eq!(
                intent.instruction_index,
                expected["instruction_index"].as_u64().unwrap() as usize
            );
            assert_eq!(intent.user.to_string(), expected["user"].as_str().unwrap());
            assert_eq!(intent.input_account.to_string(), expected["input"].as_str().unwrap());
            assert_eq!(intent.output_account.to_string(), expected["output"].as_str().unwrap());
            assert_eq!(intent.amount_in, expected["amount_in"].as_u64());
            assert_eq!(intent.min_amount_out, expected["min_amount_out"].as_u64());
            assert_eq!(intent.hops.len(), expected["hops"].as_array().unwrap().len());
            for (hop, expected) in intent.hops.iter().zip(expected["hops"].as_array().unwrap()) {
                for (key, value) in
                    [hop.base_mint, hop.quote_mint, hop.venue, hop.base_vault, hop.quote_vault]
                        .iter()
                        .zip(expected.as_array().unwrap())
                {
                    assert_eq!(key.to_string(), value.as_str().unwrap());
                    assert_ne!(*key, Pubkey::default());
                }
            }
            assert_eq!(intent.metadata.slot, f["slot"].as_u64().unwrap());
            assert_eq!(intent.metadata.tx_index, 888);
            assert_eq!(intent.metadata.grpc_recv_us, 999);
            assert_eq!(
                intent.metadata.recent_blockhash.as_deref(),
                Some(tx.message.recent_blockhash().to_string()).as_deref()
            );
            if intent.kind == PumpRouteKind::BuyPumpDelta {
                assert!(expected["executed_input_for_comparison"].as_u64().unwrap() > 0);
                assert_eq!(intent.amount_in, None, "runtime balance must not become a wire amount");
            }
            // The new intent variant and metadata must round-trip for callbacks/storage.
            let roundtrip: DexEvent =
                serde_json::from_value(serde_json::to_value(event).unwrap()).unwrap();
            assert_eq!(roundtrip.metadata().signature, tx.signatures[0]);
        }
        let mut raw = Vec::new();
        parse_transaction_dex_events_with_filter(
            &tx,
            tx.signatures[0],
            42,
            888,
            999,
            None,
            &mut raw,
        );
        assert_eq!(
            raw.len(),
            if addresses(&f, "loaded_writable").is_empty()
                && addresses(&f, "loaded_readonly").is_empty()
            {
                expected.len()
            } else {
                0
            }
        );
        let user_filter =
            EventTypeFilter::include_only(vec![EventType::PumpFunCreate, EventType::PumpFunTrade]);
        assert_eq!(resolved(&tx, &f, Some(&user_filter)).len(), expected.len());
        let route_filter = EventTypeFilter::include_only(vec![EventType::PumpRouteIntent]);
        assert_eq!(resolved(&tx, &f, Some(&route_filter)).len(), expected.len());
        let swap_filter = EventTypeFilter::include_only(vec![EventType::PumpSwapTrade]);
        assert_eq!(
            resolved(&tx, &f, Some(&swap_filter)).len(),
            if expected[0]["kind"] == "MultiHopSwap" { 1 } else { 0 }
        );
        for filter in [
            EventTypeFilter::include_only(vec![]),
            EventTypeFilter::include_only(vec![EventType::PumpFunCreate]),
            EventTypeFilter::include_only(vec![EventType::PumpFunSell]),
            EventTypeFilter::exclude_types(vec![EventType::PumpRouteIntent]),
        ] {
            assert!(resolved(&tx, &f, Some(&filter)).is_empty());
        }
        let combined = EventTypeFilter {
            include_only: Some(vec![EventType::PumpRouteIntent, EventType::PumpFunTrade]),
            exclude_types: Some(vec![EventType::PumpRouteIntent]),
        };
        assert!(resolved(&tx, &f, Some(&combined)).is_empty());
    }
}

#[test]
fn malformed_routes_and_unknown_programs_do_not_emit_or_panic() {
    for f in fixtures() {
        let tx = transaction(&f);
        let index = f["expected"][0]["instruction_index"].as_u64().unwrap() as usize;
        let original = tx.message.instructions()[index].clone();
        // Isolate the candidate so rejection cannot be hidden by sibling routes.
        for length in 0..original.data.len() {
            let mut malformed = tx.clone();
            let mut ix = original.clone();
            ix.data.truncate(length);
            *instructions_mut(&mut malformed) = vec![ix];
            assert!(resolved(&malformed, &f, None).is_empty(), "truncation {length}");
        }
        for length in 0..original.accounts.len() {
            let mut malformed = tx.clone();
            let mut ix = original.clone();
            ix.accounts.truncate(length);
            *instructions_mut(&mut malformed) = vec![ix];
            // A shorter official route can still be well-formed; wrappers are exact layouts.
            if f["expected"][0]["kind"] != "MultiHopSwap" || length < 21 || (length - 16) % 5 != 0 {
                assert!(resolved(&malformed, &f, None).is_empty(), "account truncation {length}");
            }
        }
        for mutate in [0, 1, 2] {
            let mut malformed = tx.clone();
            let mut ix = original.clone();
            if mutate == 0 {
                ix.data[0] ^= 0xff;
            }
            if mutate == 1 {
                ix.program_id_index = 0;
            } // user program != allowlisted program
            if mutate == 2 {
                ix.data.push(0);
            }
            *instructions_mut(&mut malformed) = vec![ix];
            assert!(resolved(&malformed, &f, None).is_empty());
        }
        // Resolved-key validation is transactional: errors leave the output unchanged.
        let mut events = resolved(&tx, &f, None);
        let before = serde_json::to_value(&events).unwrap();
        let mut malformed = tx.clone();
        instructions_mut(&mut malformed)[index].program_id_index = 255;
        assert!(parse_transaction_dex_events_with_loaded_addresses(
            &malformed,
            &addresses(&f, "loaded_writable"),
            &addresses(&f, "loaded_readonly"),
            tx.signatures[0],
            42,
            0,
            0,
            None,
            &mut events
        )
        .is_err());
        assert_eq!(serde_json::to_value(&events).unwrap(), before);
        let mut writable = addresses(&f, "loaded_writable");
        writable.push(Pubkey::new_from_array([123; 32]));
        assert!(parse_transaction_dex_events_with_loaded_addresses(
            &tx,
            &writable,
            &addresses(&f, "loaded_readonly"),
            tx.signatures[0],
            42,
            0,
            0,
            None,
            &mut events
        )
        .is_err());
        assert_eq!(serde_json::to_value(&events).unwrap(), before);
        // Sanity check fixture ALT order contains every signed instruction reference.
        let account_keys = keys(&tx, &f);
        assert!(tx
            .message
            .instructions()
            .iter()
            .all(|ix| (ix.program_id_index as usize) < account_keys.len()
                && ix.accounts.iter().all(|&i| (i as usize) < account_keys.len())));
    }
}

fn signed_instruction(
    program: Pubkey,
    mut accounts: Vec<Pubkey>,
    data: Vec<u8>,
    user_index: usize,
) -> VersionedTransaction {
    use solana_instruction::{AccountMeta, Instruction};
    let payer = Keypair::new_from_array([83; 32]);
    accounts[user_index] = payer.pubkey();
    let ix = Instruction {
        program_id: program,
        data,
        accounts: accounts
            .iter()
            .enumerate()
            .map(|(i, k)| AccountMeta::new(*k, i == user_index))
            .collect(),
    };
    let message = VersionedMessage::Legacy(WireMessage::new_with_blockhash(
        &[ix],
        Some(&payer.pubkey()),
        &Hash::new_from_array([61; 32]),
    ));
    let tx = VersionedTransaction::try_new(message, &[payer]).unwrap();
    for (signature, key) in tx.signatures.iter().zip(tx.message.static_account_keys()) {
        assert!(signature.verify(key.as_ref(), &tx.message.serialize()));
    }
    wincode::deserialize_exact(&wincode::serialize(&tx).unwrap()).unwrap()
}

#[test]
fn signed_static_official_routes_and_curve_bounds() {
    let curve: Value =
        serde_json::from_str(include_str!("fixtures/shred_routes/curve_instruction.json")).unwrap();
    let data: Vec<u8> = serde_json::from_value(curve["data"].clone()).unwrap();
    let accounts = addresses(&curve, "accounts");
    let tx =
        signed_instruction(curve["program"].as_str().unwrap().parse().unwrap(), accounts, data, 3);
    let f = serde_json::json!({"slot":42,"loaded_writable":[],"loaded_readonly":[]});
    let events = resolved(&tx, &f, None);
    assert_eq!(events.len(), 1);
    let DexEvent::PumpRouteIntent(e) = &events[0] else { panic!() };
    assert_eq!(e.kind, PumpRouteKind::MultiHopCurveSwap);
    assert_eq!(e.is_buy, None);
    assert_eq!(e.ix_name, None);
    assert_eq!(e.hops.len(), 1);
    assert_eq!(e.hops[0].base_mint.to_string(), curve["accounts"][13].as_str().unwrap());
    assert_eq!(e.min_amount_out, None);
    for length in 0..tx.message.instructions()[0].data.len() {
        let mut malformed = tx.clone();
        instructions_mut(&mut malformed)[0].data.truncate(length);
        assert!(resolved(&malformed, &f, None).is_empty());
    }
    for n in [0u32, 2, u32::MAX] {
        let mut malformed = tx.clone();
        instructions_mut(&mut malformed)[0].data[16..20].copy_from_slice(&n.to_le_bytes());
        assert!(resolved(&malformed, &f, None).is_empty());
    }
    let original =
        fixtures().into_iter().find(|f| f["expected"][0]["kind"] == "MultiHopSwap").unwrap();
    let captured = transaction(&original);
    let all_keys = keys(&captured, &original);
    let ix = &captured.message.instructions()
        [original["expected"][0]["instruction_index"].as_u64().unwrap() as usize];
    let accounts: Vec<_> = ix.accounts.iter().map(|&i| all_keys[i as usize]).collect();
    // Zero minimum output is a valid constraint, not a missing route.
    let mut data = ix.data.clone();
    data[16..24].copy_from_slice(&0u64.to_le_bytes());
    let tx = signed_instruction(all_keys[ix.program_id_index as usize], accounts, data, 0);
    let mut raw = Vec::new();
    parse_transaction_dex_events_with_filter(&tx, tx.signatures[0], 42, 0, 999, None, &mut raw);
    assert_eq!(raw.len(), 1);
    let DexEvent::PumpRouteIntent(e) = &raw[0] else { panic!() };
    assert_eq!(e.hops.len(), 2);
    assert_eq!(e.min_amount_out, Some(0));
    let mut zero_input = tx.clone();
    instructions_mut(&mut zero_input)[0].data[8..16].copy_from_slice(&0u64.to_le_bytes());
    assert!(resolved(&zero_input, &f, None).is_empty());
}

// Only this test thread is counted; concurrent tests cannot contaminate results.
struct CountingAllocator;
thread_local! {
    static COUNT_ALLOCATIONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        COUNT_ALLOCATIONS.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        COUNT_ALLOCATIONS.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        std::alloc::System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn excluded_routes_do_not_allocate_in_the_parser() {
    for f in fixtures() {
        let tx = transaction(&f);
        let writable = addresses(&f, "loaded_writable");
        let readonly = addresses(&f, "loaded_readonly");
        let filter = EventTypeFilter::exclude_types(vec![EventType::PumpRouteIntent]);
        let mut events = Vec::new();
        ALLOCATIONS.with(|n| n.set(0));
        COUNT_ALLOCATIONS.with(|active| active.set(true));
        let result = parse_transaction_dex_events_with_loaded_addresses(
            &tx,
            &writable,
            &readonly,
            tx.signatures[0],
            42,
            0,
            999,
            Some(&filter),
            &mut events,
        );
        COUNT_ALLOCATIONS.with(|active| active.set(false));
        result.unwrap();
        assert!(events.is_empty());
        assert_eq!(
            ALLOCATIONS.with(|n| n.get()),
            0,
            "excluded route should reject before building an event"
        );
    }
}
