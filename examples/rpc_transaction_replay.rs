//! Fetch a real transaction once, then replay the saved RPC payload offline.
//! cargo run --example rpc_transaction_replay -- --fetch SIGNATURE OUTPUT.json
//! cargo run --example rpc_transaction_replay -- --fixture OUTPUT.json
//! SOLANA_RPC_URL selects the RPC endpoint; its URL is never printed.
use anyhow::{bail, ensure, Context, Result};
use sol_parser_sdk::parse_rpc_transaction_with_cost;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::{fs, str::FromStr, time::Duration};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (tx, requested): (EncodedConfirmedTransactionWithStatusMeta, Option<Signature>) = match args
        .as_slice()
    {
        [mode, path] if mode == "--fixture" => (serde_json::from_slice(&fs::read(path)?)?, None),
        [mode, signature, path] if mode == "--fetch" => {
            let signature = Signature::from_str(signature)?;
            let url = std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
            let client = RpcClient::new_with_timeout(url, Duration::from_secs(30));
            let tx = client
                .get_transaction_with_config(
                    &signature,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::Base64),
                        max_supported_transaction_version: Some(1),
                        ..Default::default()
                    },
                )
                .context("getTransaction failed; retry or use an archive RPC")?;
            // Preserve the original payload even if parsing exposes a defect.
            fs::write(path, serde_json::to_vec_pretty(&tx)?)?;
            (tx, Some(signature))
        }
        _ => bail!("usage: --fetch SIGNATURE OUTPUT.json | --fixture INPUT.json"),
    };
    let parsed = parse_rpc_transaction_with_cost(&tx, None)?;
    if let Some(signature) = requested {
        ensure!(parsed.signature == signature, "RPC returned a different signature");
    }
    // Serialize directly: converting events through serde_json::Value rejects
    // legitimate u128 liquidity/price fields above u64::MAX.
    println!(
        "{{\"signature\":{},\"slot\":{},\"execution_error\":{},\"events\":{},\"cost\":{}}}",
        serde_json::to_string(&parsed.signature.to_string())?,
        tx.slot,
        serde_json::to_string(&tx.transaction.meta.as_ref().and_then(|meta| meta.err.as_ref()))?,
        serde_json::to_string_pretty(&parsed.events)?,
        serde_json::to_string(&parsed.cost)?
    );
    Ok(())
}
