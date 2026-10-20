//! Print admitted pools and their historical fee snapshots, without network access.
//! cargo run --example stock_token_catalog
use sol_parser_sdk::stock_prices::StockTokenCatalog;

fn main() {
    let catalog = StockTokenCatalog::screened_defaults();
    for token in &catalog.tokens {
        for pool in catalog.pools.iter().filter(|p| p.mint == token.mint) {
            println!(
                "{}",
                serde_json::json!({
                    "symbol":token.symbol, "mint":token.mint.to_string(),
                    "pool":pool.pool.to_string(), "dex_type":pool.dex_type,
                    "quote_mint":pool.quote_mint.to_string(),
                    "preferred":catalog.preferred_pool(&token.mint).is_some_and(|p|p.pool==pool.pool),
                    "historical_fee_snapshot":pool.catalog_fee_snapshot,
                })
            );
        }
    }
}
