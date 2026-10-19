//! Offline by default. --fixture FILE accepts captures with {cases:[{wire_rpc}]}.
//! --rules FILE explicitly selects reviewed wrapper rules. The bundled rules are
//! examples of observed layouts, not a promise about future program upgrades.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    core::events::DexEvent,
    grpc::{EventType, EventTypeFilter},
    wrapped_pumpfun_parser::{parse_wrapped_pumpfun_instruction, WrappedPumpfunRule},
};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = include_str!("../tests/fixtures/wrapped_pumpfun_reported.json").to_owned();
    let mut rules = include_str!("fixtures/wrapped_pumpfun_rules.json").to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let path = args.next().ok_or("expected file after --fixture or --rules")?;
        match arg.as_str() {
            "--fixture" => fixture = std::fs::read_to_string(path)?,
            "--rules" => rules = std::fs::read_to_string(path)?,
            _ => return Err("unknown argument".into()),
        }
    }
    let f: serde_json::Value = serde_json::from_str(&fixture)?;
    let rules: Vec<WrappedPumpfunRule> = serde_json::from_str(&rules)?;
    let filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
    for c in f["cases"].as_array().ok_or("missing cases")? {
        let rpc = serde_json::from_value(c["wire_rpc"].clone())?;
        let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc)?;
        let bytes = STANDARD
            .decode(c["wire_rpc"]["transaction"][0].as_str().ok_or("expected base64 wire")?)?;
        let signed: VersionedTransaction = wincode::deserialize_exact(&bytes)?;
        let loaded = |v: &[Vec<u8>]| -> Result<Vec<Pubkey>, _> {
            v.iter().map(|k| Pubkey::try_from(k.as_slice())).collect()
        };
        let mut shred = vec![];
        // In production these addresses must come from an existing local ALT
        // cache. This offline comparison obtains them from the saved capture.
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &signed,
            &loaded(&meta.loaded_writable_addresses)?,
            &loaded(&meta.loaded_readonly_addresses)?,
            signed.signatures[0],
            rpc.slot,
            77,
            123456,
            Some(&filter),
            &mut shred,
        )?;
        let wrapped: Vec<_> = shred
            .iter()
            .filter_map(|e| {
                let DexEvent::RawInstruction(raw) = e else { return None };
                rules
                    .iter()
                    .find_map(|rule| parse_wrapped_pumpfun_instruction(raw, rule, Some(&filter)))
            })
            .collect();
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx),
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
        println!(
            "{}",
            serde_json::json!({"signature": signed.signatures[0].to_string(),
            "grpc": grpc, "shred": shred, "opt_in_wrapped": wrapped})
        );
    }
    Ok(())
}
