//! Offline replay of recent signed transactions with instruction-local DEX filters.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    core::events::DexEvent,
    grpc::{EventType, EventTypeFilter},
    shredstream::parse_transaction_dex_events_with_loaded_addresses,
};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../tests/fixtures/shred_routes/raw.json"))?;
    for case in cases {
        let tx: VersionedTransaction =
            wincode::deserialize_exact(&STANDARD.decode(case["wire"].as_str().unwrap())?)?;
        let keys = |name: &str| -> Vec<Pubkey> {
            case[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| k.as_str().unwrap().parse().unwrap())
                .collect()
        };
        for event_type in
            [EventType::PumpFunTrade, EventType::RaydiumCpmmSwap, EventType::RawInstruction]
        {
            let mut events = vec![];
            parse_transaction_dex_events_with_loaded_addresses(
                &tx,
                &keys("loaded_writable"),
                &keys("loaded_readonly"),
                tx.signatures[0],
                case["slot"].as_u64().unwrap(),
                0,
                0,
                Some(&EventTypeFilter::include_only(vec![event_type])),
                &mut events,
            )?;
            let raw: Vec<_> =
                events.iter().filter(|e| matches!(e, DexEvent::RawInstruction(_))).collect();
            println!(
                "{}",
                serde_json::json!({"signature":tx.signatures[0].to_string(), "filter":format!("{event_type:?}"), "raw_instructions":raw})
            );
        }
    }
    Ok(())
}
