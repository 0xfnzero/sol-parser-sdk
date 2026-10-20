//! Generate/check the two SDK catalogs from saved evidence; no network access.
#[allow(dead_code)]
#[path = "support/stock_catalog.rs"]
mod stock_catalog;
use anyhow::{ensure, Result};
use stock_catalog::*;

fn main() -> Result<()> {
    let args = arguments(&["report", "fee-audit", "parser-root", "trade-root"], &["check"])?;
    let catalog = build_catalog(
        &read_json(required(&args, "report")?)?,
        &read_json(required(&args, "fee-audit")?)?,
    )?;
    let paths: Vec<_> = ["parser-root", "trade-root"]
        .iter()
        .map(|key| {
            Ok(std::path::Path::new(required(&args, key)?).join("src/stock_token_catalog.json"))
        })
        .collect::<Result<_>>()?;
    if args.contains_key("check") {
        for path in &paths {
            verify_catalog(&read_json(path)?, &catalog)?;
        }
        ensure!(std::fs::read(&paths[0])? == std::fs::read(&paths[1])?, "SDK catalog files differ");
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
