# StonkFun transaction audit — 2026-10-01

## Reproducible sample

Captured the most recent 50 signatures for each of the Standard platform
`4E876qZTE9FJMrBzgVtBrSrzz2TLivB5Y5QXPjB4gZL7`, Reward platform
`6BwHHDg3u1854jC8PDLXvR4spTcLNaoBxLJNGC4nTESt`, and verified graduated
KNOTS CPMM `BUVzsLLLG7GWoyJVoU31pXiBveazA6GXTavZ9VD3CwS9`.
148 of 150 full transactions were captured and parsed: 111 succeeded, 37 failed.
Raw JSON/base64 and the signature manifest remain in
`/tmp/stonkfun-corpus-20261001`; five selected fixtures are in `tests/fixtures`.
This is a recent bounded sample, with only one graduated pool. It cannot establish
market-wide coverage percentages or validate the reported 1,316-trade statistics.

Successful decoded swap legs (not transaction counts):

| Protocol | Legs |
| --- | ---: |
| LaunchLab | 68 (38 Standard, 30 Reward) |
| CPMM | 38 |
| DLMM | 35 |
| CLMM | 28 |
| Whirlpool | 15 |
| PumpSwap | 6 |
| AMM v4 | 2 |

105 successful transactions contain an attributed StonkFun leg. Of these,
35 have no additional decoded swap, 57 have one, and 13 have multiple. These
counts describe observed instructions, not a guarantee of direct funding or
serial topology: unknown programs and unrelated swaps can coexist. All decoded
successful route legs resolve input/output mints with the new route API.

## Observed forms and corrected examples

Users trade with pre-held quote tokens, convert SOL to quote, sell quote back to
SOL, use intermediate USDC, split funds across pools, and interact through
aggregators or opaque programs. A program wrapping known swaps is a router;
an undecoded invocation carrying token transfers without known swap descendants
is an unresolved economic operation, not proof of a private pool.

The five supplied historical signatures were fetched separately:

| Signature | Observed funding path |
| --- | --- |
| `Sd12BssQUC1tq2DBFg7zwXRNcqPB3JfguPpkFY83J4bHLV1SSjmhGAcRRQBCX88MJMVfcwBJrEY4eXQMj6zDymU` | Whirlpool WSOL→quote→LaunchLab |
| `64VqBwhyGbwVdrtP2Vsf9dd975saVs2GsqTTvJBA8VGKzepJydKasE16RzLVvYp8ZVFBJsub9UuHMyZngSF9LnSE` | Custom program WSOL→USDC, then DLMM USDC→quote→LaunchLab |
| `3sVu14kV8fydzwwWxBWZ4GVgHLP6Qnox1Eh8tXLKbL1k1MBynsrDmtdkubVfyxE1b6tT16X1B6gVPbHxx5kTjZzF` | CLMM USDC→quote→LaunchLab; no same-transaction SOL conversion |
| `5266gqa7XF1Zz6nc921CLJZWJN8tHipZLwUpegxXxByFFqFixGz8bPZhNu3hz6YAWb1dGq6J11FtEFegpsyNPvkF` | Split paths including Whirlpool, CLMM and a USDC branch; not a simple serial two-hop |
| `4ftgdPj6RsGRuN5HmkYqa8eBLq6hsscZuv52oHAvGW7DvCrK5c3LjEwFJFB4gU7Tftq1NExiCDpVunH8iwjbqtXp` | CLMM WSOL→USDC, then DLMM USDC→quote→LaunchLab |

## Parser improvements

```rust,ignore
use sol_parser_sdk::analyze_rpc_transaction_routes;
let route = analyze_rpc_transaction_routes(&transaction, &verified_graduated_pools)?;
for leg in &route.legs {
    // position, pool, mint pair, instruction limits and observed actual amounts
}
```

The opt-in RPC and Yellowstone APIs preserve execution order, outer/inner indices,
CPI depth, normalized swaps, token transfers and unknown invocations. Mint
resolution combines pre/post balances, checked transfers, explicit protocol
accounts and propagation across plain transfers, including ephemeral accounts.
Instruction amounts/thresholds stay separate from executed amounts. Failed
transactions retain intent but never expose successful actual swap amounts.
Unknown Token-2022 net credits remain unknown when the fee is unavailable.
`trader` is an invocation authority and may be a router PDA, not the fee payer.

Migration parsing now recognizes LaunchLab `migrate_to_cpswap` and legacy
`migrate_to_amm`, exposes old/new pool, base/quote mints, platform and destination
program. `liquidity_amount_known` is false when the instruction does not provide
an actual LP amount. Graduated StonkFun attribution requires a caller-maintained
verified pool set, ideally populated from migrations; a stock quote alone is
insufficient attribution.

### Migration registry (2026-10-02)

`StonkFunPoolRegistry` now maintains a caller-owned index of successful StonkFun
CPMM migrations. It maps curve pool, base mint and quote mint to the graduated
pool, retaining original signature and slot. Non-StonkFun platforms and failed
transactions are ignored. Duplicate replay is idempotent; conflicting provenance
rejects the entire batch without partially updating the index.

```rust,ignore
use sol_parser_sdk::{StonkFunPoolRegistry, analyze_rpc_transaction_routes};
let mut registry = StonkFunPoolRegistry::default();
registry.observe_rpc_transaction(&migration_transaction)?;
let candidates = registry.pools_for_base_mint(&meme_mint);
let identities = registry.verified_cpmm_pools();
let route = analyze_rpc_transaction_routes(&trade_transaction, &identities)?;
let saved = serde_json::to_string(&registry)?;
```

Persist/restore with serde JSON. Load trusted finalized transaction history or
handle rollback in your application. This index does not backfill missed history
or discover SOL↔quote funding pools, and it does not verify present-day liquidity.
The new registry is covered by synthetic migration/replay/conflict tests; the
trade route fixtures remain real mainnet transactions.

Mint resolution also reads token-account initialization for ephemeral accounts.
Truncated checked transfers and SPL Token instructions masquerading as
Token-2022 fee instructions are rejected instead of fabricating data.

The route analyzer intentionally allocates and is separate from the existing
event hot path. The legacy event mint-autofill change was not applied: a safety
hook incorrectly classified that ordinary local patch as a remote repository
transfer and blocked it. Use route-leg mints when legacy events lack them.

Remaining gaps: opaque/private programs, complete aggregator-specific semantics,
automatic history backfill and rollback handling, transfer-hook adapters and
market-wide sampling. The SDK exposes positions and transfers for topology
analysis; it does not claim its flat leg list is a reconstructed serial route.

## Re-run

```sh
python3 scripts/capture_stonkfun.py --limit 50 --output /tmp/stonkfun-corpus
cargo run --example analyze_stonkfun_corpus -- /tmp/stonkfun-corpus
cargo test --lib --tests
cargo test --no-default-features --features parse-zero-copy --lib --test stonkfun_routes
```

The capture tool reads `RPC_URL` optionally, excludes endpoint credentials from
the manifest, and never submits transactions. RPC/Yellowstone parity, graduated
attribution, failed execution, complex paths and migration mapping have fixture
regressions. See [trade route API](../../sol-trade-sdk/docs/STONKFUN_ROUTES.md)
for the newly supported explicit funding routes and their quote requirements.

## SOL/WSOL evidence and offline trade preparation

`TransactionRoute.native_token_actions` retains System transfers into accounts
identified as WSOL, SyncNative instructions, and WSOL CloseAccount destinations
and authorities. It also retains attempted instructions for failed transactions;
check `succeeded` before treating them as executed. This is lifecycle evidence,
not a decision to spend native SOL versus an existing WSOL balance. It is not a
net output amount (closing returns rent as well), and transactions without mint
identity may have no lifecycle evidence even if they use native token accounts.

Trade SDK's optional `parser-adapter` converts a LaunchLab/CPMM route leg into
pool/mint identity clues. `SubscriptionAccountCache` then validates current
pool/config/mint/vault bytes and builds complete meme parameters without RPC.
Transaction parsing alone does not expose complete current fee configuration,
liquidity state or a fresh quote. Subscribe to the missing accounts; do not copy
historical instruction thresholds or trade reserves as a new trade's quote.
See trade SDK's `docs/STONKFUN_ROUTES.md` for freshness, epoch and snapshot rules.

## Subscription state bridge (2026-10-02)

`AccountLiquiditySnapshot` adds validated raw snapshots for LaunchLab pools,
GlobalConfig/PlatformConfig, DLMM pools/bin arrays/bitmap extensions, Whirlpool
dynamic tick arrays/adaptive Oracles and CLMM bitmap extensions. Existing account
parsers continue handling other supported normalized account types.

For a complete trade cache, explicitly opt in to `AccountRawSnapshot` through
`EventTypeFilter::include_only`. This event preserves the original `AccountData`,
slot, write_version and startup flag, including zero-lamport/empty account closure
updates and account types without dedicated decoders (mints, Clock, ALTs, configs).
It is not emitted by default. Address filters must subscribe to every dependency;
a transaction event cannot supply unchanged config or current liquidity bytes.

The trade SDK's `update_from_parser_snapshot` imports these events with version
ordering and tombstones. Its new `prepare_stonkfun_trade` quotes explicit paths
from subscribed state and resolves the current inner/graduated meme parameters,
then applies the caller's wallet, amount, direction and asset choice. See the
[compile-checked integration example](../../sol-trade-sdk/examples/stonkfun_cached_prepare.rs).
SOL/WSOL selection remains caller intent; native lifecycle actions are evidence.
Publish consistent batches, handle rollback externally and reprepare on dependent
updates/epoch changes. Neither the parser nor adapter claims automatic best-route
selection or full opaque-program replay.
