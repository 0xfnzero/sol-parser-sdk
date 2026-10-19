//! Offline analyzer. With no arguments uses signed mainnet fixtures; otherwise
//! fetches each signature from SOLANA_RPC_URL. Never used in the streaming path.
use sol_parser_sdk::{convert_rpc_to_grpc, wrapped_pumpfun_rpc_analyzer::analyze_wrapped_pumpfun};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signatures: Vec<_> = std::env::args().skip(1).collect();
    let transactions: Vec<EncodedConfirmedTransactionWithStatusMeta> = if signatures.is_empty() {
        let f: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/wrapped_pumpfun_reported.json"))?;
        f["cases"]
            .as_array()
            .ok_or("missing cases")?
            .iter()
            .map(|c| serde_json::from_value(c["wire_rpc"].clone()))
            .collect::<Result<_, _>>()?
    } else {
        let client = RpcClient::new(
            std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into()),
        );
        signatures
            .iter()
            .map(|s| {
                Ok(client.get_transaction_with_config(
                    &s.parse()?,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::Base64),
                        max_supported_transaction_version: Some(1),
                        ..Default::default()
                    },
                )?)
            })
            .collect::<Result<_, Box<dyn std::error::Error>>>()?
    };
    for rpc in transactions {
        let (meta, tx) = convert_rpc_to_grpc(&rpc)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "signature": bs58::encode(&tx.signatures[0]).into_string(),
                "slot": rpc.slot, "analyses": analyze_wrapped_pumpfun(&tx, &meta),
            }))?
        );
    }
    Ok(())
}
