//! Offline corpus inspection: cargo run --example analyze_stonkfun_corpus -- <directory>
use sol_parser_sdk::{analyze_rpc_transaction_routes, parse_rpc_transaction};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("supply a captured corpus directory")?;
    let mut paths: Vec<_> = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".base64.json"))
        .collect();
    paths.sort();
    for path in paths {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let result = serde_json::from_value::<EncodedConfirmedTransactionWithStatusMeta>(value)
            .map_err(|error| error.to_string())
            .and_then(|transaction| {
                let events =
                    parse_rpc_transaction(&transaction, None).map_err(|error| error.to_string())?;
                let graduated =
                    [solana_sdk::pubkey!("BUVzsLLLG7GWoyJVoU31pXiBveazA6GXTavZ9VD3CwS9")];
                let route = analyze_rpc_transaction_routes(&transaction, &graduated)
                    .map_err(|error| error.to_string())?;
                Ok((events, route))
            });
        let file = serde_json::to_string(&path.file_name().map(|name| name.to_string_lossy()))?;
        // Serialize directly: converting through Value rejects valid u128 prices.
        match result {
            Ok((events, route)) => println!(
                "{{\"file\":{file},\"events\":{},\"route\":{}}}",
                serde_json::to_string(&events)?,
                serde_json::to_string(&route)?
            ),
            Err(error) => {
                println!("{{\"file\":{file},\"error\":{}}}", serde_json::to_string(&error)?)
            }
        }
    }
    Ok(())
}
