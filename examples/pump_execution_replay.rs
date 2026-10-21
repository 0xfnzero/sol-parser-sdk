//! Offline by default; pass a current signature to fetch via SOLANA_RPC_URL.
use base64::{engine::general_purpose::STANDARD, Engine};
use sol_parser_sdk::{convert_rpc_to_grpc, grpc::parse_subscribe_update_transaction_low_latency};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use yellowstone_grpc_proto::prelude::{SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transactions: Vec<EncodedConfirmedTransactionWithStatusMeta> = if let Some(sig) =
        std::env::args().nth(1)
    {
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
        // Offline Shred replay: transaction bytes + supplied ALT snapshot only.
        // Fetching the snapshot above is tooling, never part of the parser hot path.
        let signed = rpc.transaction.transaction.decode().ok_or("invalid transaction")?;
        let addresses = |values: &[Vec<u8>]| -> Result<Vec<Pubkey>, _> {
            values.iter().map(|bytes| Pubkey::try_from(bytes.as_slice())).collect()
        };
        let mut shred_events = Vec::new();
        sol_parser_sdk::shredstream::parse_transaction_dex_events_with_loaded_addresses(
            &signed,
            &addresses(&meta.loaded_writable_addresses)?,
            &addresses(&meta.loaded_readonly_addresses)?,
            signed.signatures[0],
            rpc.slot,
            0,
            0,
            None,
            &mut shred_events,
        )?;
        println!(
            "{}",
            serde_json::json!({"mode":"shred", "signature":signature_text, "slot":rpc.slot, "events":shred_events})
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
                    ..Default::default()
                }),
            };
            let events = parse_subscribe_update_transaction_low_latency(
                &update,
                0,
                rpc.block_time.map(|s| s * 1_000_000),
                None,
            );
            println!(
                "{}",
                serde_json::json!({"mode":mode,"signature":signature_text,"slot":rpc.slot,"events":events})
            );
        }
    }
    Ok(())
}
