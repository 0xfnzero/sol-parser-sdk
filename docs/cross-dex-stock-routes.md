# Cross-DEX stock conversion legs

The target token's launch venue does not determine the stock conversion venue.
Stonk LaunchLab can follow a Meteora stock swap; Pump.fun/PumpSwap can precede
or follow Raydium, Orca or Meteora. Inspect each leg's program, pool, mint pair
and instruction position. Do not relabel all legs using the outer router or
the launch platform, and do not infer a linear path from co-occurrence alone.

`analyze_rpc_transaction_routes` and `analyze_yellowstone_transaction_routes`
preserve all supported DEX legs independently. They now also recognize Pump.fun
v2 quote-token buys/sells and the IDL's 15-account LaunchLab trade layout.
`StockTokenCatalog::conversion_legs(&route)` selects successful, catalog-approved
conversion legs by exact pool/program/mint pair. The target pool is not treated
as a conversion pool merely because one of its tokens is a stock.

```sh
cargo run --example stock_route_replay
cargo test --test cross_dex_stock_routes
```

The bundled six signed mainnet captures cover Stonk/Meteora, PumpSwap/Raydium,
PumpSwap/Orca, and Pump.fun v2 buy/sell and v3 with Raydium. Every leg is printed
with base58 addresses, its own DEX, instruction position and optional executed
amounts. `catalog_stock_conversion` is pool membership, not proof that a leg
funded another leg. Unknown invocations remain visible.

For another transaction, fetch outside the hot path and replay offline:

```sh
cargo run --example rpc_transaction_replay -- --fetch SIGNATURE /tmp/route.json
cargo run --example stock_route_replay -- --fixture /tmp/route.json
```

No RPC is performed by route analysis or either replay example's default mode.
gRPC callers must provide the complete transaction and metadata, including CPI
instructions and ALT keys. Filtering an event stream to PumpSwap events will
intentionally hide its Raydium/Orca/Meteora events; it cannot show the whole route.
Shred alone cannot reconstruct hidden CPI legs. Never attach fetched execution
metadata to Shred and present the result as Shred-only decoding.

An unavailable executed amount stays `None`. In particular, a Token-2022 gross
transfer without known withheld fees does not establish net output. Historical
amounts, limits and catalog fees are not live quotes or suggested order sizes.
