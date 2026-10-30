//! Offline by default. --reported-alt replays only the two reported GMGN trades.
//! Pass a current signature to fetch via SOLANA_RPC_URL, outside the parser.
//! Creation JSON includes alt_addresses: transaction ALT table keys, without resolving them.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{
    convert_rpc_to_grpc,
    grpc::{parse_subscribe_update_transaction_low_latency, EventType, EventTypeFilter},
};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("Usage: cargo run --example pump_execution_replay -- [--reported-alt | SIGNATURE]\nNo arguments: all bundled captures, offline.\n--reported-alt: only 2EUPDVEH… and 38NiGzWa…, offline.\nSIGNATURE: fetch via SOLANA_RPC_URL before parsing.\nShred replay uses saved transaction inputs and ALT addresses; gRPC additionally uses saved execution metadata.");
        return Ok(());
    }
    if args.len() > 1 {
        return Err("expected --reported-alt or one signature; see --help".into());
    }
    let transactions: Vec<EncodedConfirmedTransactionWithStatusMeta> = if args
        .first()
        .is_some_and(|s| s == "--reported-alt")
    {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/wrapped_pumpfun_reported.json"))?;
        fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["resolved_pump_trade_count"].is_number())
            .map(|case| serde_json::from_value(case["wire_rpc"].clone()))
            .collect::<Result<_, _>>()?
    } else if let Some(sig) = args.first() {
        let url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
        vec![RpcClient::new(url).get_transaction_with_config(
            &sig.parse::<Signature>()?,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                max_supported_transaction_version: Some(1),
                ..Default::default()
            },
        )?]
    } else {
        let mut fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/pump_execution_20261019.json"))?;
        let reported: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/wrapped_pumpfun_reported.json"))?;
        fixture["cases"]
            .as_array_mut()
            .unwrap()
            .extend(reported["cases"].as_array().unwrap().iter().cloned());
        let multi_hop: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/pump_multi_hop_20261025.json"))?;
        fixture["cases"]
            .as_array_mut()
            .unwrap()
            .extend(multi_hop["cases"].as_array().unwrap().iter().cloned());
        let create_alt: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/pump_create_alt_20261026.json"))?;
        fixture["cases"]
            .as_array_mut()
            .unwrap()
            .extend(create_alt["cases"].as_array().unwrap().iter().cloned());
        fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| serde_json::from_value(c["wire_rpc"].clone()))
            .collect::<Result<_, _>>()?
    };
    for rpc in transactions {
        let (meta, tx) = convert_rpc_to_grpc(&rpc)?;
        let signature = tx.signatures[0].clone();
        let signature_text = Signature::try_from(signature.as_slice())?.to_string();
        let signed = rpc.transaction.transaction.decode().ok_or("invalid transaction")?;
        // Exactly the static-only entry point used on the Shred hot path. No ALT,
        // logs, inner instructions or RPC calls are available to this parser.
        let filter =
            EventTypeFilter::include_only(vec![EventType::PumpFunCreate, EventType::PumpFunTrade]);
        let mut shred_events = Vec::new();
        sol_parser_sdk::shredstream::pump_ix::parse_transaction_dex_events_with_filter(
            &signed,
            signed.signatures[0],
            rpc.slot,
            888,
            123456,
            Some(&filter),
            &mut shred_events,
        );
        println!(
            "{}",
            serde_json::json!({"mode":"shred_static_pump", "signature":signature_text, "slot":rpc.slot, "event_count":shred_events.len(), "events":shred_events})
        );
        // Compare with caller-supplied ALT addresses; fetching them above is
        // tooling, never part of the parser hot path.
        let addresses = |values: &[Vec<u8>]| -> Result<Vec<Pubkey>, _> {
            values.iter().map(|bytes| Pubkey::try_from(bytes.as_slice())).collect()
        };
        shred_events.clear();
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &signed,
            &addresses(&meta.loaded_writable_addresses)?,
            &addresses(&meta.loaded_readonly_addresses)?,
            signed.signatures[0],
            rpc.slot,
            888,
            123456,
            Some(&filter),
            &mut shred_events,
        )?;
        println!(
            "{}",
            serde_json::json!({"mode":"shred_resolved", "signature":signature_text, "slot":rpc.slot, "event_count":shred_events.len(), "events":shred_events})
        );
        for mode in ["full", "no_logs", "truncated_logs", "missing_create_logs"] {
            let mut meta = meta.clone();
            if mode == "no_logs" {
                meta.log_messages.clear();
            }
            if mode == "truncated_logs" {
                meta.log_messages.truncate(meta.log_messages.len() / 2);
            }
            if mode == "missing_create_logs" {
                meta.log_messages.retain(|log| {
                    !log.strip_prefix("Program data: ")
                        .and_then(|s| STANDARD.decode(s).ok())
                        .is_some_and(|data| {
                            data.starts_with(&[27, 114, 169, 77, 222, 235, 99, 118])
                        })
                });
            }
            let update = SubscribeUpdateTransaction {
                slot: rpc.slot,
                transaction: Some(SubscribeUpdateTransactionInfo {
                    signature: signature.clone(),
                    transaction: Some(tx.clone()),
                    meta: Some(meta),
                    index: 888,
                    ..Default::default()
                }),
            };
            let events = parse_subscribe_update_transaction_low_latency(
                &update,
                123456,
                rpc.block_time.map(|s| s * 1_000_000),
                Some(&filter),
            );
            println!(
                "{}",
                serde_json::json!({"mode":mode,"signature":signature_text,"slot":rpc.slot,"event_count":events.len(),"events":events})
            );
        }
    }
    Ok(())
}
