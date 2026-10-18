use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use sol_parser_sdk::{
    core::events::DexEvent,
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
                default.iter().filter(|e| matches!(e, DexEvent::RawInstruction(_))).count(),
                count as usize
            );
            assert!(!default.iter().any(|e| matches!(
                e,
                DexEvent::PumpFunTrade(_)
                    | DexEvent::PumpFunBuy(_)
                    | DexEvent::PumpFunSell(_)
                    | DexEvent::PumpFunBuyExactSolIn(_)
            )));
        }
        if f["signature"].as_str().unwrap().starts_with("3nMX")
            || f["signature"].as_str().unwrap().starts_with("2r4u")
        {
            let default = parse(&tx, &f, None);
            assert_eq!(
                default.len(),
                if f["signature"].as_str().unwrap().starts_with("3nMX") { 2 } else { 3 }
            );
            assert!(default.iter().all(|e| matches!(e, DexEvent::RawInstruction(_))));
            assert!(parse(
                &tx,
                &f,
                Some(&EventTypeFilter::include_only(vec![EventType::PumpFunTrade]))
            )
            .is_empty());
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
        None,
        &mut events,
    );
    let [DexEvent::RawInstruction(raw)] = &events[..] else { panic!() };
    assert_eq!(raw.instruction_data, ix.data);
    assert_eq!(raw.account_indices, ix.accounts);
    assert_eq!(raw.accounts.last(), Some(&None));
    assert_eq!(raw.accounts[0], raw.accounts[raw.accounts.len() - 2]);
    assert_eq!(raw.accounts.len(), ix.accounts.len());
}
