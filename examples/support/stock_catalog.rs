//! Offline catalog maintenance shared by the Rust examples and regression tests.
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn read_json(path: impl AsRef<Path>) -> Result<Value> {
    let path = path.as_ref();
    serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("parse {}", path.display()))
}

pub fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

/// JSON number spelling (25 vs 25.0) and float round-trip noise are not drift.
/// Integer identifiers remain exact; arrays, fields and preference order must match.
pub fn verify_catalog(actual: &Value, expected: &Value) -> Result<()> {
    fn compare(a: &Value, b: &Value, path: &str) -> Result<()> {
        match (a, b) {
            (Value::Object(a), Value::Object(b)) => {
                ensure!(a.len() == b.len(), "catalog fields differ at {path}");
                for (key, value) in a {
                    compare(
                        value,
                        b.get(key).with_context(|| format!("missing {path}/{key}"))?,
                        &format!("{path}/{key}"),
                    )?;
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                ensure!(a.len() == b.len(), "catalog length differs at {path}");
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    compare(a, b, &format!("{path}/{i}"))?;
                }
            }
            (Value::Number(a), Value::Number(b)) if a.is_f64() || b.is_f64() => {
                let a = a.as_f64().context("invalid number")?;
                let b = b.as_f64().context("invalid number")?;
                ensure!(
                    (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs()),
                    "catalog number differs at {path}: {a} != {b}"
                );
            }
            _ => ensure!(a == b, "catalog value differs at {path}: {a} != {b}"),
        }
        Ok(())
    }
    compare(actual, expected, "")
}

pub fn dex_type(program: &str) -> Option<&'static str> {
    match program {
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some("raydium_clmm"),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some("orca_whirlpool"),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => Some("meteora_dlmm"),
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => Some("raydium_cpmm"),
        _ => None,
    }
}

pub fn nominal_fee_score(fees: &Value, rate: &Value) -> Option<f64> {
    let mut retained = 1.0;
    for tax in ["stock_transfer_tax", "quote_transfer_tax"] {
        // Missing hook information is unknown, not proof of a free transfer.
        if fees[tax]["transfer_hook"].as_bool() != Some(false) {
            return None;
        }
    }
    for value in [
        rate,
        &fees["stock_transfer_tax"]["basis_points"],
        &fees["quote_transfer_tax"]["basis_points"],
    ] {
        let rate = value.as_f64()?;
        if !rate.is_finite() || !(0.0..10000.0).contains(&rate) {
            return None;
        }
        retained *= 1.0 - rate / 10000.0;
    }
    Some(10000.0 * (1.0 - retained))
}

pub fn depth_passes(audit: &Value, minimum: f64) -> bool {
    audit["liquid_side_reserve_usd"].as_f64().is_some_and(|r| r.is_finite() && r >= minimum)
        && ["buy_output", "sell_output"]
            .iter()
            .all(|key| audit[key].as_f64().is_some_and(|v| v > 0.0))
}

pub fn pool_order(a: &Value, b: &Value) -> Ordering {
    a["mint"]
        .as_str()
        .cmp(&b["mint"].as_str())
        .then_with(|| {
            a["nominal_snapshot_fee_bps"]
                .as_f64()
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b["nominal_snapshot_fee_bps"].as_f64().unwrap_or(f64::INFINITY))
        })
        .then_with(|| {
            b["liquid_side_reserve_usd"]
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(&a["liquid_side_reserve_usd"].as_f64().unwrap_or(0.0))
        })
        .then_with(|| a["pool"].as_str().cmp(&b["pool"].as_str()))
}

pub fn build_catalog(report: &Value, audit: &Value) -> Result<Value> {
    ensure!(
        report["minimum_liquid_side_usd"].as_f64() == Some(500.0)
            && report["volume_filter"].as_bool() == Some(false),
        "Expected a $500 reserve threshold without a volume filter"
    );
    let audit = audit
        .as_array()
        .or_else(|| audit["candidate_results"].as_array())
        .context("fee audit must contain candidate results")?;
    let fee_rows: BTreeMap<_, _> =
        audit.iter().filter_map(|r| r["pool"].as_str().map(|p| (p, &r["fees"]))).collect();
    let mut tokens = BTreeMap::new();
    let mut pools = Vec::new();
    for row in report["candidates"].as_array().context("missing candidates")? {
        if row["accepted"].as_bool() != Some(true) {
            continue;
        }
        let Some(dex) = row["program_id"].as_str().and_then(dex_type) else {
            continue;
        };
        for key in ["pool", "mint", "quote_mint", "symbol", "catalog_source"] {
            ensure!(row[key].as_str().is_some_and(|s| !s.is_empty()), "missing {key}");
        }
        ensure!(row["decimals"].as_u64().is_some_and(|d| d <= 255), "invalid decimals");
        ensure!(
            row["liquid_side_reserve_usd"].as_f64().is_some_and(|r| r.is_finite() && r >= 500.0),
            "accepted pool below reserve threshold"
        );
        let mut fees = row["fees"].clone();
        let mut score = None;
        if !fees.is_null() {
            ensure!(fees.is_object(), "fees must be an object");
            let mut rate = fees["swap_fee_bps"].clone();
            if let Some(captured) = fee_rows.get(row["pool"].as_str().unwrap()) {
                if captured["slot"].as_u64().is_some() && captured["slot"] == fees["snapshot_slot"]
                {
                    ensure!(
                        captured["unix_timestamp"].as_i64().is_some()
                            && captured["unix_timestamp"] == fees["unix_timestamp"],
                        "fee timestamp mismatch: {}",
                        row["pool"]
                    );
                    let base =
                        captured["base_fee_bps"].as_f64().context("invalid audited base fee")?;
                    let expected =
                        fees["base_swap_fee_bps"].as_f64().context("invalid snapshot base fee")?;
                    ensure!(
                        base.is_finite() && expected.is_finite() && (base - expected).abs() <= 1e-7,
                        "base fee mismatch: {}",
                        row["pool"]
                    );
                    if let Some(captured_rate) = captured.get("snapshot_start_fee_bps") {
                        rate = captured_rate.clone();
                    }
                }
            }
            score = nominal_fee_score(&fees, &rate);
            fees["snapshot_start_swap_fee_bps"] = rate;
        }
        tokens.insert(row["mint"].as_str().unwrap().to_owned(), json!({"mint":row["mint"], "symbol":row["symbol"], "decimals":row["decimals"], "catalog_source":row["catalog_source"]}));
        pools.push(json!({"pool":row["pool"], "mint":row["mint"], "quote_mint":row["quote_mint"], "dex_type":dex,
            "catalog_fee_snapshot":fees, "nominal_snapshot_fee_bps":score, "liquid_side_reserve_usd":row["liquid_side_reserve_usd"]}));
    }
    pools.sort_by(pool_order);
    let mut seen = BTreeSet::new();
    for pool in &mut pools {
        pool["preferred"] = json!(seen.insert(pool["mint"].as_str().unwrap().to_owned()));
    }
    Ok(json!({"schema_version":1, "policy":{"minimum_liquid_side_usd":500, "volume_filter":false,
        "fee_preference":"Known nominal snapshot-start swap plus transfer rates, then deeper reserve; unknown last. Not a live or whole-order quote."},
        "tokens":tokens.into_values().collect::<Vec<_>>(), "pools":pools}))
}

/// Strict named arguments; rejects typos and duplicate flags.
pub fn arguments(names: &[&str], switches: &[&str]) -> Result<BTreeMap<String, String>> {
    let mut args = std::env::args().skip(1);
    let mut result = BTreeMap::new();
    while let Some(flag) = args.next() {
        let key = flag.strip_prefix("--").context("expected --option")?;
        let value = if switches.contains(&key) {
            String::new()
        } else if names.contains(&key) {
            args.next().filter(|v| !v.starts_with("--")).context("missing option value")?
        } else {
            bail!("unknown option: {flag}")
        };
        ensure!(result.insert(key.to_owned(), value).is_none(), "duplicate option: {flag}");
    }
    Ok(result)
}

pub fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    args.get(key).map(String::as_str).with_context(|| format!("missing --{key}"))
}
