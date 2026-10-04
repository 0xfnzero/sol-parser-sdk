//! PublicNode raw-account + Clock/blockhash subscription for downstream caches.
//! GRPC_TOKEN=<provider token> ACCOUNTS=<comma-separated pubkeys> \
//! cargo run --example grpc_raw_cache -- snapshots.jsonl
//! No RPC bootstrap: unchanged accounts might never emit an update.
use sol_parser_sdk::{
    grpc::{AccountFilter, ClientConfig, EventType, EventTypeFilter, YellowstoneGrpc},
    DexEvent,
};
use std::{
    io::Write,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "PublicNode gRPC raw accounts + Clock + block metadata (no RPC)
Usage: ACCOUNTS=<comma-separated pubkeys> GRPC_TOKEN=<provider token> \
       cargo run --example grpc_raw_cache -- [snapshots.jsonl]
Environment: GRPC_URL/GRPC_ENDPOINT, GRPC_TOKEN/GRPC_AUTH_TOKEN, DURATION_SECONDS (default 30)
Writes account snapshots to the supplied path and block metadata to <path>.blocks.jsonl.
Unchanged accounts may emit no updates; this is an incremental stream, not a startup snapshot."
        );
        return Ok(());
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let path = std::env::args().nth(1).unwrap_or_else(|| "snapshots.jsonl".into());
    let mut keys: Vec<String> = std::env::var("ACCOUNTS")?
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(|key| key.parse::<solana_sdk::pubkey::Pubkey>().map(|key| key.to_string()))
        .collect::<Result<_, _>>()?;
    if keys.is_empty() {
        return Err("ACCOUNTS must include at least one valid pubkey".into());
    }
    let mut file = std::fs::File::create(&path)?;
    let mut blocks = std::fs::File::create(format!("{path}.blocks.jsonl"))?;
    keys.push(solana_sdk::sysvar::clock::ID.to_string());
    keys.sort();
    keys.dedup();
    let grpc = YellowstoneGrpc::new_with_config(
        std::env::var("GRPC_URL")
            .or_else(|_| std::env::var("GRPC_ENDPOINT"))
            .unwrap_or_else(|_| "https://solana-yellowstone-grpc.publicnode.com:443".into()),
        std::env::var("GRPC_TOKEN").or_else(|_| std::env::var("GRPC_AUTH_TOKEN")).ok(),
        ClientConfig::default(),
    )?;
    let queue = grpc
        .subscribe_dex_events(
            vec![],
            vec![AccountFilter { account: keys, owner: vec![], filters: vec![] }],
            Some(EventTypeFilter::include_only(vec![
                EventType::AccountRawSnapshot,
                EventType::BlockMeta,
            ])),
        )
        .await?;
    let seconds = std::env::var("DURATION_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut count = 0;
    while Instant::now() < deadline {
        while let Some(event) = queue.pop() {
            match event {
                DexEvent::RawAccountSnapshot(snapshot) => {
                    writeln!(file, "{}", serde_json::to_string(&snapshot)?)?;
                    count += 1;
                    println!(
                        "account={} slot={} version={} closed={}",
                        snapshot.account.pubkey,
                        snapshot.metadata.slot,
                        snapshot.write_version,
                        snapshot.account.lamports == 0
                    );
                }
                DexEvent::BlockMeta(block) => {
                    writeln!(blocks, "{}", serde_json::to_string(&block)?)?;
                    println!(
                        "block={} hash={}",
                        block.metadata.slot,
                        block.metadata.recent_blockhash.as_deref().unwrap_or("unknown")
                    );
                }
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    grpc.stop().await;
    if count == 0 {
        return Err(
            "No raw accounts received: check token, address filters and endpoint availability"
                .into(),
        );
    }
    println!("Saved {count} raw updates; this is an incremental stream, not a complete startup snapshot.");
    Ok(())
}
