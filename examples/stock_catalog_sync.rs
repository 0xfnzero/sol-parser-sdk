//! Generate/check SDK catalogs from saved evidence; no network access.
#[allow(dead_code)]
#[path = "support/stock_catalog.rs"]
mod stock_catalog;
use anyhow::{ensure, Result};
use stock_catalog::*;

fn main() -> Result<()> {
    let args = arguments(
        &["report", "fee-audit", "parser-root", "trade-root", "workspace-root"],
        &["check"],
    )?;
    let catalog = build_catalog(
        &read_json(required(&args, "report")?)?,
        &read_json(required(&args, "fee-audit")?)?,
    )?;
    let mut paths: Vec<_> = ["parser-root", "trade-root"]
        .iter()
        .map(|key| {
            Ok(std::path::Path::new(required(&args, key)?).join("src/stock_token_catalog.json"))
        })
        .collect::<Result<_>>()?;
    if let Some(root) = args.get("workspace-root") {
        for relative in [
            "sol-parser-sdk-nodejs/src/stock_token_catalog.json",
            "sol-parser-sdk-python/sol_parser/stock_token_catalog.json",
            "sol-parser-sdk-golang/solparser/stock_token_catalog.json",
            "sol-trade-sdk-nodejs/src/stock_token_catalog.json",
            "sol-trade-sdk-python/src/stock_token_catalog.json",
            "sol-trade-sdk-golang/pkg/stockcatalog/stock_token_catalog.json",
            "sol-shred-sdk/src/stock_token_catalog.json",
        ] {
            paths.push(std::path::Path::new(root).join(relative));
        }
    }
    if args.contains_key("check") {
        for path in &paths {
            verify_catalog(&read_json(path)?, &catalog)?;
        }
        for path in &paths[1..] {
            ensure!(
                std::fs::read(&paths[0])? == std::fs::read(path)?,
                "SDK catalog files differ: {}",
                path.display()
            );
        }
    } else {
        for path in paths {
            write_json(path, &catalog)?;
        }
    }
    println!(
        "{} mints, {} pools; identical SDK catalogs",
        catalog["tokens"].as_array().unwrap().len(),
        catalog["pools"].as_array().unwrap().len()
    );
    Ok(())
}
