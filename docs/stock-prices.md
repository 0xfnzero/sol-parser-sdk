# Stock and ETF price subscriptions

`stock_prices::StockPriceSubscription` is an explicit opt-in subscription. It
emits `StockTokenPriceEvent` rather than changing the existing `DexEvent` stream.
Use a dedicated `YellowstoneGrpc` client: starting any subscription on a client
replaces that client's previous subscription. `StockPriceConfig::default()` has
an empty catalog; call `StockTokenCatalog::screened_defaults()` to enable defaults.

```rust,no_run
use sol_parser_sdk::stock_prices::{StockPriceConfig, StockTokenCatalog};
let mut config = StockPriceConfig {
    catalog: StockTokenCatalog::screened_defaults(),
    ..Default::default()
};
// Add/replace/remove config.catalog.tokens and config.catalog.pools before subscribing.
// Duplicate mints/pools, unsupported DEXes and unsupported quote mints are rejected.
```

The catalog contains **33 distinct mints / 58 pools**, covering xStocks and the
Backpack stock/ETF directory. Directory source is not a claim about legal issuer
or collateral backing. The same stock from different providers remains a
different mint. ETFs and other listed products retain their original symbols;
they are not represented as ordinary company shares.

## Screening evidence

[Captured evidence](research/stock-catalog-20261020.json) records exact official
mint identities, mint decimals from chain snapshots, pool owners, slots,
transaction signatures, bidirectional depth tests and on-chain fee snapshots.

The [2026-10-22 review](research/stock-catalog-20261022.json) is the current
catalog-generation input. It removes five pools after large-withdrawal history
and a fresh $500 liquid-side reserve check; see the [exact exclusions](STOCK_BRIDGE_SYNC.md#pool-removals-on-2026-10-22).
The old account/transaction captures remain historical evidence. In particular,
the bundled ABNB price replay now yields no default stock-price event for its
removed pool, while its underlying DEX trade still parses normally.

Admission now requires **at least $500 of verified USDC/WSOL-side pool reserve**,
with no volume or TVL filter. USDC is valued at $1; WSOL is valued through the
snapshot's cached SOL/USDC quote. This is liquid-side reserve, not a claim that
$500 can execute within a particular slippage bound. Successful $10 buy/sell
quote probes and their impact versus $1 probes are recorded separately. Impact
above 100 bps no longer independently excludes a pool: the depth tool's legacy
`eligible` flag is retained only as raw evidence, and admission is recomputed.

Initially, all 69 previously discovered candidates were reconsidered, including those
excluded by the former $10k threshold. Fifty-five have sufficient reserve and usable
two-way quote evidence. Missing quotes/accounts remain unverified, not silently
admitted. SPHR.US is now included regardless of its low volume; BABA.US, INTCx
and QQQx were also added. Snapshot eligibility is historical, not a current quote.

The subsequent [seven-source scan](research/stock-discovery-20261020.json) captured
2,080 successful transactions without fetch gaps, from 1,000 signatures per source
(Stonk platform accounts, Pump.fun/PumpSwap, and Raydium AMM v4/CPMM/CLMM).
Its source windows span 2026-10-20 05:34:02–05:37:19 UTC+8; some high-frequency
program samples cover only seconds. This is not a full-day or exhaustive scan.
Of 33 candidate accounts, eight new pools passed identity, $500 liquid-side
reserve and bidirectional quote validation: seven Raydium CLMM pools and one
Orca pool. HIMS.US, AMD.US, RACE.US and RDDT.US add four mints; AAPLx, ABNB.US
and MSFTx gain alternatives. That scan produced 34 mints / 63 pools; the later
2026-10-22 review reduces the current defaults to 33 mints / 58 pools.
One additional RDDT.US pool failed admission. Non-stock route legs and invalid
candidate account positions were rejected rather than admitted by association.

Candidate discovery reused bounded recent mainnet transaction sampling and
official identity records from the sibling trade SDK research. It is not an
exhaustive ecosystem ranking. Unknown tokens or tickers alone are never admitted.
The offline refresh tool accepts that evidence schema and coherent pool snapshots:

```sh
cargo run --example stock_catalog_screen -- --evidence VERIFIED_EVIDENCE.json \
  --snapshots SNAPSHOT_DIRECTORY --scan BROAD_CANDIDATE_SCAN.json \
  --identities OFFICIAL_MINT_RECORDS.json --minimum 500 \
  --depth-tool /path/to/bridge_pool_depth --fees-tool target/debug/examples/stock_pool_fees \
  --output REPORT.json --catalog-output CATALOG.json
```

Build the local fee tool with `cargo build --example stock_pool_fees`. The depth
tool is the sibling sol-trade-sdk's `bridge_pool_depth` example (offline quote
replay); it is not a runtime parser dependency. The screener performs no network
requests and **does not refresh input snapshots**. Refresh them independently,
review the generated catalog, then update `src/stock_token_catalog.json`.

## Fees and pool preference

Each `StockPool.catalog_fee_snapshot` provides slot, epoch, timestamp, base swap
fee, known total swap fee (or `None` for dynamic fees), and transfer tax for both
mints. One basis point (bps) is 0.01%. Protocol fee shares are already part of
swap fees and are not added again. Transfer taxes include raw-unit caps and the
active transfer-hook program, if any; a disabled hook extension is not active.

At these snapshots all 34 admitted stock/ETF mints have **0 bps configured token
transfer tax**, and no active transfer hook. This does not make swaps free or
guarantee future rates. Known fixed pool fees range from **1 to 100 bps**:
preferred SPYx 0.01%, AAPLx 0.08%, TSLAx/NVDAx/GLDx 0.10%, TTWO.US 0.20%,
ABNB.US/GOOGLx/MSFTx 0.25%. Other pools use dynamic fees; for example AVGOx's
0.05% base fee is **not** a verified total fee. Exact per-pool values are retained
in the bundled catalog and evidence report.

Both SDKs now package the same `src/stock_token_catalog.json`. Within each mint,
pools are ordered by known nominal snapshot-start swap/transfer rates, lowest
first; deeper reserve breaks ties. Unknown rates follow known rates. Dynamic
`snapshot_start_swap_fee_bps` is the initial active-bin/tick rate at the recorded
Clock, not a fixed or whole-order fee. `catalog.preferred_pool(&mint)` returns
the shared historical preference for that mint. sol-trade-sdk installs all
58 qualified stock pools, including alternatives, plus its WSOL/USDC bridge.
All qualified alternatives remain subscribed, so this preference does not hide
trades. It is an offline catalog preference, not a live routing decision.
Transfer caps, transaction size, dynamic fees and hook behavior can change actual
cost or ranking; priority excludes unpriced hook behavior, network fees, tips
and price impact. Catalog fees are never presented as live fee updates.

Run `cargo run --example stock_token_catalog` to print every pool's base58 mint
and pool addresses, preference flag, and full fee snapshot without RPC access.

Regenerate both packaged catalogs together after screening; add `--check` to
verify generated JSON content and byte-for-byte parity between SDK copies without writing:

```sh
cargo run --example stock_catalog_sync -- --report docs/research/stock-catalog-20261022.json \
  --fee-audit ../sol-trade-sdk/docs/research/bridge-defaults-20261020-expanded-scan.json \
  --parser-root . --trade-root ../sol-trade-sdk
```

Run `cargo test --test stock_catalog_tools` for offline screening-policy,
fee-ordering and recorded-catalog regressions. Both maintenance tools and their
tests are Rust; they do not require Python. `--catalog-output` uses the same
$500 policy and schema as the sync tool; other screening thresholds are report-only.
Active transfer hooks and missing fee information remain unknown, not zero-cost.

The optional research collector `scripts/discover_stock_pools.py` scans bounded recent signatures from Stonk
platform accounts, Pump.fun, PumpSwap and Raydium AMM v4/CPMM/CLMM. Pass complete
official directories with `--xstocks` and `--backpack`, not a ticker allowlist.
It records fetch gaps and candidate signatures; `--replay` reuses that signature
window. Candidate instruction-account positions are discovery hints only:
validate pool owner/layout, mint pair, depth and fees before admission. Neither
discovery nor refresh runs on the parsing/trading hot path.

## Event semantics

Events include mint, symbol, pool, typed DEX identifier, quote mint, raw stock
and quote amounts, SOL/USDC prices, full transaction metadata, optional reference
price provenance, and optional liquidity/depth snapshot.

- `basis = ReportedSwapAmounts`: the ratio of protocol-reported executed swap
  amounts with token decimals. It is not NAV, pool spot price, executable quote,
  or a fee-adjusted user receipt. Transfer-fee and protocol-fee conventions differ.
- Supports Raydium CLMM/CPMM, Orca Whirlpool and Meteora DLMM, including confirmed
  CPI trades inside routers. These cover all current default stock pools.
- Only `ExecutionConfirmed` events with nonzero amounts and matching configured
  pool/mint/DEX identities qualify. Shred instruction-only/unresolved callbacks
  cannot establish an executed price and are ignored.
  Canonical execution fields must also be present: CPMM reserve-before amounts,
  CLMM/Whirlpool ending sqrt price, or DLMM binary event version 1/2. Successful
  DLMM exact-out instructions with only maximum-input limits are not priced.
- SOL denotes WSOL units (1:1); USDT and other quotes are not silently treated as USDC.
- The directly traded quote price is always available. The other denomination
  uses only a fresh configured SOL/USDC pool execution (default: Orca
  `Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE`). Missing/stale/future reference
  means `None`. Defaults: 30 seconds receive age and 75 slots maximum lag.
  References from the same transaction are excluded because event metadata
  does not supply a universal CPI order for all supported protocols.
- Pool prices remain separate. There is no misleading cross-pool aggregate or
  whole-market "latest price". Older slot/transaction positions are rejected;
  multiple swaps within one transaction remain separate notifications.
- `liquidity` is **None until supplied** through `update_liquidity()`. Publish
  coherent, timestamped TVL/depth from your background account/quote collector.
  Depth includes a price-impact bound from 0 to 9999 bps and a nonblank source.
  Invalid updates do not replace the last accepted snapshot. Concentrated liquidity's raw `liquidity`
  number is not TVL. Historic catalog TVL is never replayed as live liquidity.

The processor performs no network access. All RPC/HTTP collection belongs in
offline discovery or a separate background collector. It resets conversion and
liquidity caches after reconnects/queue loss. Raw `StockPriceTracker` users must
call `reset()` after discontinuities or detected fork rollbacks. ExecutionConfirmed
does not mean finalized; stream commitment remains the client's policy.

## Examples and verification

```sh
GRPC_ENDPOINT=https://YOUR_ENDPOINT GRPC_AUTH_TOKEN=TOKEN \
  cargo run --example stock_price_subscribe
cargo run --example stock_price_replay
cargo test --test stock_prices
```

The replay includes actual AAPLx/Orca, ABNB.US/Raydium CLMM and MU.US/Meteora DLMM
transactions fetched as signed wire data with execution metadata. It prints
historical execution prices, not live prices. Without a matching fresh reference
or external liquidity snapshot it intentionally leaves those values unknown.

## Trading integration

The sibling trade SDK exposes `PoolTradeHint`, `SubscriptionAccountCache`,
`PumpSwapRoutingFeed` and configurable per-hop bridge execution/impact limits
(default 100 bps each). Map each catalog pool's `quote_mint`→`mint` to a hint
and merge with `PumpSwapRoutingConfig::extend_pools`. Feed raw account snapshots
into the background cache; a `StockTokenPriceEvent` is an execution-price signal,
not enough state for quoting. Missing state must remain unavailable. See
`sol-trade-sdk/docs/STREAMING_QUOTES.md` for parser/streamer × gRPC/shred examples.
