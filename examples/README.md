# Parser examples

Run from the `sol-parser-sdk` repository root with `cargo run --example NAME`.
Start with an offline replay; configure endpoints only when using online tools.

## Start here: offline

- `stock_route_replay`: each DEX leg, pool, mint pair and stock-conversion identity.
- `stock_price_replay`: stock execution-price notifications from saved transactions.
- `stock_token_catalog`: bundled stock identities and historical fee snapshots.
- `wrapped_trade_replay`: router provenance across DEX event families.
- `pump_execution_replay`: Shred input replay and gRPC execution replay with full, missing, truncated or missing-create logs; accepts a transaction signature via `SOLANA_RPC_URL`.
- `cpmm_creator_fee_replay`: CPMM execution and creator-fee regression.

```sh
cargo run --example stock_route_replay
cargo run --example stock_price_replay
cargo run --example wrapped_trade_replay
```

The first, second and fourth entries share `--fixture FILE` and `--help`.
FILE may be a saved RPC transaction or a `{"cases":[{"rpc":...}]}` capture;
`wire_rpc` cases are also accepted. Malformed or empty captures are errors.
Defaults use bundled captures and never fall back to network access.

## Maintain the shared stock catalog: offline Rust tools

`stock_catalog_screen` validates saved snapshots with the Rust `stock_pool_fees`
and sibling SDK's `bridge_pool_depth` tools. `stock_catalog_sync` builds/checks
the identical packaged catalogs. See [commands and evidence requirements](../docs/stock-prices.md).
Run `cargo test --test stock_catalog_tools` for policy, fee and catalog regressions.
Optional Python research collectors are not part of the Rust test workflow.

## Validate examples and SDK behavior

```sh
cargo test --lib --tests
cargo check --examples
cargo test --example stock_route_replay --example stock_pool_fees
```

Regression tests belong in Rust under `tests/` or `#[cfg(test)]`; do not add Python
test runners to the Rust workflow. Historical fixture provenance may mention
another language, but reading those fixtures does not run that language.
Online subscription examples process bounded batches before checking cancellation.

Wrapped Pump.fun provenance is verified by the single offline unit-test module
`src/wrapped_pumpfun_rpc_analyzer.rs`:

```sh
cargo test --offline --lib wrapped_pumpfun_rpc_analyzer
```

It checks the reported signed transactions through gRPC and outer-only Shred parsing.
It does not infer wrapper rules or fetch RPC data. See [the callback contract](../docs/wrapped-pumpfun.md).

## Capture and inspect real transactions: RPC

Use one generic capture/replay entry point for new signatures:

```sh
SOLANA_RPC_URL=https://YOUR_RPC cargo run --example rpc_transaction_replay -- \
  --fetch SIGNATURE /tmp/transaction.json
cargo run --example stock_route_replay -- --fixture /tmp/transaction.json
```

- `rpc_discover_replay` discovers and verifies wire captures; `rpc_corpus_validate`
  checks saved execution evidence. See each file's usage before fetching.
- `analyze_stonkfun_corpus` inspects captured Stonk routes; see
  [route analysis](../docs/ROUTE_ANALYSIS.md).
- Existing `parse_*`, `debug_*` and `test_account_filling` are focused diagnostics,
  not automated test-suite entry points. Some have historical signatures and
  require archival RPC. Prefer saved replays for repeatable verification.

## Online subscriptions

Provide `GRPC_ENDPOINT` and, if required, `GRPC_AUTH_TOKEN`.
The shared configuration also accepts `GRPC_URL` / `GRPC_TOKEN` for compatibility.
Examples using `support/grpc_config.rs` have no built-in authentication token.
Individual older diagnostics document any additional configuration in source.
Account listeners, `grpc_raw_cache` and `pumpswap_low_latency` use the same
configuration helper: set an explicit endpoint instead of relying on a provider
fallback. `GRPC_ENDPOINT` / `GRPC_AUTH_TOKEN` take precedence over their aliases.

- Stocks: `stock_price_subscribe`; [price semantics and liquidity](../docs/stock-prices.md).
- Pump.fun: `pumpfun_trade_filter`, `pumpfun_trade_filter_ordered`,
  `pumpfun_with_metrics`, `pumpfun_quick_test` (bounded subscription smoke check).
  The smoke check stops after ten events, sixty seconds, or Ctrl-C and closes
  its subscription; it is not a deterministic regression test.
- PumpSwap: `pumpswap_low_latency`, `pumpswap_ordered`, `pumpswap_with_metrics`.
- Other protocols: `meteora_damm_grpc`, `meteora_damm_new_pool`,
  `raydium_cpmm_new_pool`, `raydium_launchlab_migration`, and the `*_token_price` examples.
- Filters/state: `dynamic_subscription`, `wallet_trade_filter`,
  `grpc_raw_cache`, `*_listen`;
  [account subscription configuration](../docs/ACCOUNT_SUBSCRIPTIONS.md).
- Shred: `shredstream_example`, `shredstream_pump_snipe`.
  Shred does not expose executed CPI routes or confirmed prices.

## Specialist tools

- `stock_pool_fees`: decode historical account-snapshot fees; no live quote.
- `grpc_latency_slot_compare`: transport diagnostics.
- `fixtures/` contains inputs; `support/` contains helpers, not separate example binaries.

## Naming and contribution rules

Use `snake_case`, with the protocol/domain first and purpose last:
`stock_route_replay`, `stock_price_subscribe`, `pumpswap_ordered`.
Keep established public example names unless consolidating the same workflow.
Use official protocol/IDL terminology inside examples; do not rename instruction
methods such as `buy_exact_quote_in_v2`. Keep SDK implementation and strategy
logic out of duplicated wrappers; share only configuration, input and formatting.
Execution replays share `support/grpc_replay.rs`; input normalization moves large
JSON values rather than cloning captures. Do not feed executed CPI metadata to Shred.
Protocol diagnostics and independent regression cases are not interchangeable.

`cross_dex_stock_routes` was renamed to `stock_route_replay`. Test targets and
capture filenames retain their descriptive `cross_dex_*` names.
See the [documentation index](../docs/README.md) for the authoritative topic guides.
