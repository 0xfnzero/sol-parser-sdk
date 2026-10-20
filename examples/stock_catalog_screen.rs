//! Screen saved pool snapshots with the Rust depth/fee tools; no RPC or HTTP.
#[allow(dead_code)]
#[path = "support/stock_catalog.rs"]
mod stock_catalog;
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::{stream, StreamExt};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use stock_catalog::*;

async fn run_tool(tool: &Path, snapshot: &Path, require_success: bool) -> Result<Value> {
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new(tool).arg(snapshot).kill_on_drop(true).output(),
    )
    .await
    .context("tool timed out after 60 seconds")??;
    // A depth tool may emit valid amounts while rejecting its own stricter policy.
    ensure!(
        !require_success || output.status.success(),
        "tool failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).context("tool did not emit a JSON report")
}

struct ScreenConfig {
    snapshots: PathBuf,
    depth_tool: PathBuf,
    fees_tool: PathBuf,
    minimum: f64,
}

async fn collect(pool: &Value, records: &[Value], config: &ScreenConfig) -> Result<Value> {
    let mint = pool["output_mint"].as_str().context("missing output mint")?;
    let pool_address = pool["pool"].as_str().context("missing pool")?;
    // Prevent untrusted evidence from escaping the snapshot directory.
    let _: solana_sdk::pubkey::Pubkey = pool_address.parse().context("invalid pool address")?;
    let identity = records
        .iter()
        .find(|r| {
            r["record"]["addresses"]["solana"] == mint
                || r["record"]["tokens"].as_array().is_some_and(|tokens| {
                    tokens
                        .iter()
                        .any(|t| t["contractAddress"] == mint && t["blockchain"] == "Solana")
                })
        })
        .context("no official identity")?;
    let path = config.snapshots.join(format!("{pool_address}-snapshot.json"));
    if !path.exists()
        && pool["preliminary_liquid_reserve_usd"].as_f64().is_some_and(|r| r < config.minimum)
    {
        return Ok(json!({"pool":pool_address, "symbol":pool["symbol"], "accepted":false,
            "reason":"preliminary reserve below threshold; no full depth snapshot",
            "preliminary_liquid_reserve_usd":pool["preliminary_liquid_reserve_usd"]}));
    }
    let snapshot = read_json(&path)?;
    ensure!(snapshot["slot"].as_u64().is_some(), "missing snapshot slot");
    ensure!(
        snapshot["hints"].as_array().is_some_and(|hints| hints
            .iter()
            .any(|h| h["pool"] == pool["pool"]
                && h["input_mint"] == pool["input_mint"]
                && h["output_mint"] == mint)),
        "snapshot pair mismatch"
    );
    let account = &snapshot["accounts"][mint];
    ensure!(
        matches!(
            account["owner"].as_str(),
            Some(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                    | "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
            )
        ),
        "invalid mint owner"
    );
    ensure!(account["data"][1] == "base64", "expected base64 mint data");
    let data = STANDARD.decode(account["data"][0].as_str().context("missing mint data")?)?;
    ensure!(data.len() >= 82 && data[45] == 1, "uninitialized mint");
    let depth = run_tool(&config.depth_tool, &path, false).await?;
    ensure!(depth["slot"] == snapshot["slot"], "depth slot mismatch");
    let audit = &depth["reports"]
        .as_array()
        .context("missing depth reports")?
        .iter()
        .find(|r| r["pool"] == pool["pool"])
        .context("missing pool depth")?["audit"];
    let accepted = depth_passes(audit, config.minimum);
    let fee_result: Result<Value> = async {
        let reports = run_tool(&config.fees_tool, &path, true).await?;
        let fees = reports
            .as_array()
            .context("missing fee reports")?
            .iter()
            .find(|r| r["pool"] == pool["pool"])
            .context("missing pool fees")?
            .clone();
        ensure!(fees["snapshot_slot"] == snapshot["slot"], "fee slot mismatch");
        Ok(fees)
    }
    .await;
    let (fees, fee_error) = match fee_result {
        Ok(fees) => (fees, None),
        Err(error) => (Value::Null, Some(format!("{error:#}"))),
    };
    Ok(json!({"symbol":pool["symbol"], "mint":mint, "decimals":data[44],
        "catalog_source":identity["source"], "official_record":identity["record"],
        "pool":pool_address, "quote_mint":pool["input_mint"],
        "program_id":snapshot["accounts"][pool_address]["owner"], "snapshot_slot":snapshot["slot"],
        "observed_transaction_signatures":pool.get("observed_transaction_signatures").cloned().unwrap_or(json!([])),
        "liquid_side_reserve_usd":audit["liquid_side_reserve_usd"], "depth":depth,
        "nominal_combined_fee_bps":nominal_fee_score(&fees, &fees["swap_fee_bps"]),
        "fees":fees, "fee_error":fee_error, "accepted":accepted}))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = arguments(
        &[
            "evidence",
            "snapshots",
            "scan",
            "identities",
            "depth-tool",
            "fees-tool",
            "minimum",
            "output",
            "catalog-output",
        ],
        &[],
    )?;
    let minimum = args.get("minimum").map(|v| v.parse::<f64>()).transpose()?.unwrap_or(500.0);
    ensure!(minimum.is_finite() && minimum > 0.0, "minimum must be finite and positive");
    if args.contains_key("catalog-output") {
        ensure!(minimum == 500.0, "packaged catalog policy requires --minimum 500");
    }
    let config = ScreenConfig {
        snapshots: required(&args, "snapshots")?.into(),
        depth_tool: std::fs::canonicalize(required(&args, "depth-tool")?)?,
        fees_tool: std::fs::canonicalize(required(&args, "fees-tool")?)?,
        minimum,
    };
    let output = required(&args, "output")?;
    let evidence = read_json(required(&args, "evidence")?)?;
    let mut pools = BTreeMap::new();
    for pool in evidence["pools"].as_array().context("missing pools")? {
        if pool["symbol"] != "WSOL/USDC" {
            pools.insert(pool["pool"].as_str().context("missing pool")?.to_owned(), pool.clone());
        }
    }
    let mut records =
        evidence["issuer_records"].as_array().context("missing issuer records")?.clone();
    if let Some(path) = args.get("identities") {
        for record in
            read_json(path)?["stock_mints"].as_object().context("missing stock_mints")?.values()
        {
            records.push(json!({"source":record.get("issuer_source").cloned().unwrap_or(json!("https://api.backpack.exchange/api/v1/assets")),
                "record":record.get("issuer_record").unwrap_or(record)}));
        }
    }
    if let Some(path) = args.get("scan") {
        for pool in
            read_json(path)?["candidate_results"].as_array().context("missing candidate_results")?
        {
            pools.entry(pool["pool"].as_str().context("missing pool")?.to_owned()).or_insert_with(|| json!({
                "pool":pool["pool"], "symbol":pool["symbol"], "output_mint":pool["mint"], "input_mint":pool["quote_mint"],
                "preliminary_liquid_reserve_usd":pool["preliminary_liquid_reserve_usd"]}));
        }
    }
    // Bound subprocess fanout and retain stable pool order for reviewable reports.
    let results: Vec<Value> = stream::iter(pools.values().map(|pool| async {
        collect(pool, &records, &config).await.unwrap_or_else(|error| json!({
            "pool":pool["pool"], "symbol":pool["symbol"], "accepted":false, "error":format!("{error:#}")}))
    })).buffered(4).collect().await;
    let report = json!({"screened_at":chrono::Utc::now().to_rfc3339(), "minimum_liquid_side_usd":minimum, "volume_filter":false,
        "valuation_note":"Verified USDC/WSOL-side vault reserve; USDC assumed $1, SOL valued by cached USDC quote. Not TVL or an executable order-size guarantee.",
        "priority_note":"Known nominal swap plus configured transfer rates first; unknown rates and active hooks are not priced. Caps and order size can change ranking.",
        "coverage_note":"Bounded chain sample, not exhaustive; snapshots are historical.", "candidates":results});
    write_json(output, &report)?;
    if let Some(path) = args.get("catalog-output") {
        write_json(path, &build_catalog(&report, &json!([]))?)?;
    }
    println!(
        "{}",
        json!({"candidates":results.len(), "accepted":results.iter().filter(|r| r["accepted"] == true).count(),
        "errors":results.iter().filter(|r| r.get("error").is_some()).count()})
    );
    Ok(())
}
