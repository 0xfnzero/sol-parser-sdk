<div align="center">

## v0.7.16 — Pump execution events and current replay

Pump TradeEvent CPI decoding now accepts the official Anchor event prefix as well as the historical encoding. RPC/gRPC compiled instruction budgets and minimum outputs are kept in instruction fields; `sol_amount`, `quote_amount`, and `token_amount` are populated from execution events. If both logs and event CPIs are unavailable these execution fields remain zero (unavailable, not a proven zero fill). Non-SOL quote amounts are not SOL amounts.

Run `cargo run --example pump_execution_replay` from this repository for offline replay of transactions fetched on 2026-10-19. The example prints results with full, absent, and truncated logs; `cargo test --test pump_execution` asserts the execution amounts and identities. Append a recent signature to fetch another transaction; `SOLANA_RPC_URL` selects your RPC. RPC requests support transaction v1.

Recent captured signatures:
- Delta SellV2, slot 458542546: `4ARos3Yq1SMNE3izDpy9YdKrRgMmD7UCDxvnVrBRaGCJHiifPHSmQWc4DPfjUaEUcxS7puGmEN64K7EZdeXLJaj2`.
- Rapid create + buy, slot 458539974: `4A2UJ139R7YRvtDRUuTsbj83EQwkWZehP9mvjc9kgzJEaVLCLRKXPtw6CYwdovduVR8cYcUZ6spxHnKKKfkKhdn2`.
- BuyV3, slot 458540504: `fDy5VD3yMZAvqTjSfvTsbMK3SWBeqHhgoNsZnobFFr13cr5ppqREZsbxjtohzVXSbapP66A7i4Rs9BiVuqeqLCJ`.
- PumpSwap buy, slot 458542555: `TNSxB6uf4Qjp7ZeK7UTkDCZd6XxLbaKf8qkpZqvEhCPWwQtUzMqtTurunF9gwae64M1FKx9xjd63XCSm9XpkXLY`.

Older five-combination multi-hop fixtures remain regression tests, not newly captured mainnet examples. Regressions check per-leg identities and execution amounts, missing/truncated logs, and failed-transaction suppression.

## v0.7.16 — raw Shred instructions

Default Shred parsing emits `RawInstruction` for unsupported third-party programs, including Rapid/Delta, without inferring direction, amounts, or internal buy methods. Infrastructure programs (System, Token, ATA, ComputeBudget, etc.) are omitted by default. Supported official DEX parsers are unchanged; official Pump/PumpSwap multi-hop instructions retain `PumpRouteIntent`.

The payload contains `program_id`, `instruction_index`, `accounts`, `account_indices`, `instruction_data`, and transaction metadata. Account order and duplicates are preserved. Unresolved ALT addresses are null/None, with original indices retained; supply cached loaded addresses or a resolver. No hot-path RPC is performed. Data is copied verbatim without requiring a known layout.

No filter returns unknown third-party instructions. Explicitly including `RawInstruction` returns every outer instruction, including infrastructure. A `PumpFunTrade`-only filter does not include raw events: subscribe to both types when needed. Streamer forwards `DexEvent::RawInstructionEvent` as Common. A Pump address in accounts does not prove a CPI or a particular buy method. Event count is not fill count.

Wrapper adapter functions remain explicit compatibility helpers only; default Shred parsing does not call them. The released-version history below describes previous behavior.

    <h1>⚡ Sol Parser SDK</h1>
    <h3><em>Ultra-low latency Solana DEX event parser with SIMD optimization</em></h3>
</div>

<p align="center">
    <strong>High-performance Rust library for parsing Solana DEX events with microsecond-level latency</strong>
</p>

<p align="center">
    <a href="https://crates.io/crates/sol-parser-sdk">
        <img src="https://img.shields.io/crates/v/sol-parser-sdk.svg" alt="Crates.io">
    </a>
    <a href="https://docs.rs/sol-parser-sdk">
        <img src="https://docs.rs/sol-parser-sdk/badge.svg" alt="Documentation">
    </a>
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License">
    </a>
</p>

<p align="center">
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana">
    <img src="https://img.shields.io/badge/SIMD-FF6B6B?style=for-the-badge&logo=intel&logoColor=white" alt="SIMD">
    <img src="https://img.shields.io/badge/gRPC-4285F4?style=for-the-badge&logo=grpc&logoColor=white" alt="gRPC">
</p>

<p align="center">
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README_CN.md">中文</a> |
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README.md">English</a> |
    <a href="https://fnzero.dev/">Website</a> |
    <a href="https://t.me/fnzero_group">Telegram</a> |
    <a href="https://discord.gg/vuazbGkqQE">Discord</a>
</p>

---

## 📦 SDK Versions and Related SDKs

Parser SDK language versions and related Rust SDKs:

| Language | Repository | Description | Version |
|----------|------------|-------------|---------|
| **Rust** | [sol-parser-sdk](https://github.com/0xfnzero/sol-parser-sdk) | Ultra-low latency with SIMD optimization | `v0.7.16` |
| **Node.js** | [sol-parser-sdk-nodejs](https://github.com/0xfnzero/sol-parser-sdk-nodejs) | TypeScript/JavaScript for Node.js | `v0.5.22` |
| **Python** | [sol-parser-sdk-python](https://github.com/0xfnzero/sol-parser-sdk-python) | Async/await native support | `v0.5.15` |
| **Go** | [sol-parser-sdk-golang](https://github.com/0xfnzero/sol-parser-sdk-golang) | Concurrent-safe with goroutine support | `v0.5.15` |
| **Rust** | [sol-shred-sdk](https://github.com/0xfnzero/sol-shred-sdk) | Raw Solana shred decoding and ShredStream DEX event parsing | `v4.0.7` |
| **Rust** | [sol-trade-sdk](https://github.com/0xfnzero/sol-trade-sdk) | Solana DEX trade construction and transaction execution | `v6.0.2` |

## v0.7.15 — Pump route direction and instruction names

Route intents now expose `is_buy` and `ix_name`, following existing trade field names and official IDL instruction names. RapidBuy reports a buy intent and `buy_exact_quote_in_v2`; BuyPumpDelta reports only a buy intent. Multi-hop direction and unknown methods remain null. Read currencies from `hops[].base_mint` / `quote_mint`. These hints do not confirm execution, and filters do not convert intents into trades.

Older JSON without the fields remains readable as unknown. Rust struct literals must supply the new fields; Go uses pointers and Node.js/Python use null/None for unknown values. Streamer now forwards `DexEvent::PumpRouteIntentEvent` with protocol and event filtering.

## v0.7.14 — Pump routes and offline replay

Adds native Pump route-intent parsing for official multi-hop instructions and the exact RapidBuy/BuyPumpDelta wrapper layouts captured in reported transactions. Adds locally cached ALT resolution and rejects unresolved account identities by default, with explicit best-effort compatibility APIs where supported. Route intents expose instruction constraints and hop identities; confirmed execution amounts require execution metadata.

Includes the latest complete implementation, signed transaction fixtures, native offline-bank regression examples and reproducible local hot-path benchmarks. Unknown wrapper layouts are not inferred from embedded bytes. Local tests do not measure production network latency.

## v0.7.13 — Pump depth-2 compatibility

Adds signed Pump depth-2 create-event replay coverage, including ALT-loaded accounts and executed logs. Existing event layouts and public parsing interfaces remain unchanged.

Validation includes official Pump SDK 4.1.0 instruction comparisons, signed local LiteSVM simulations and SDK regressions. Local simulation state/config is synthetic; no funded transactions were broadcast and production latency was not measured.

## v0.7.12 — PumpFun migration event CPI fix

Fixes PumpFun migration event decoding from event CPI instructions, including captured `migrate_v2` and already-migrated transaction regressions. Retains the signed transaction, loaded-address, ordered stream and parser lifecycle hardening from 0.7.11.

Published together with `solana-streamer-sdk 3.0.10`, which pins this parser release. Validation uses offline transaction fixtures and local tests; no funded mainnet transactions are broadcast.

## v0.7.11 — Signed transaction and hot-path hardening

Hardens signed transaction sanitization, loaded-address boundaries, ordered stream filtering and parser lifecycle. Aligns CLMM and DEX instruction/event layouts, nested CPI route attribution and liquidity/reward accounting. Adds independently signed wire fixtures, ALT failure cases and offline bank regressions.

Validation includes local CPU benchmarks and offline signed-bank scenarios. Measured hot paths use cached inputs without RPC. Benchmarks do not establish production network or transaction-landing latency. No funded mainnet transactions were broadcast.

## What This SDK Is For

`sol-parser-sdk` is the low-level Rust parser core for Solana DEX events. It is designed for trading bots, copy-trading pipelines, sniper bots, indexers, and stream processors that need fast, typed parsing from Yellowstone gRPC transactions, Jito ShredStream entries, RPC transaction payloads, or account subscriptions.

| Area | Coverage |
|------|----------|
| Parser inputs | Yellowstone gRPC, ShredStream, RPC transactions, encoded transactions, protocol account data |
| DEX protocols | PumpFun, PumpSwap, Pump Fees, LaunchLab (including StonkFun), Raydium CPMM, Raydium CLMM, Raydium AMM V4, Meteora DAMM v2, Meteora DLMM, Meteora DBC, Orca Whirlpool |
| Parser backends | Default Borsh parser for maintainability, optional zero-copy parser for latency-sensitive hot paths |
| Related SDK | Use [solana-streamer](https://github.com/0xfnzero/solana-streamer) (`3.0.14`) when you want a higher-level streaming facade over this parser core |

---

## 📊 Performance Highlights

### ⚡ Ultra-Low Latency
- **10-20μs** parsing latency in release mode
- **Zero-copy** parsing with stack-allocated buffers
- **SIMD-accelerated** pattern matching (memchr)
- **Lock-free** ArrayQueue for event delivery

### 🎚️ Flexible Order Modes
| Mode | Latency | Description |
|------|---------|-------------|
| **Unordered** | 10-20μs | Immediate output, ultra-low latency |
| **MicroBatch** | 50-200μs | Micro-batch ordering with time window |
| **StreamingOrdered** | 0.1-5ms | Stream ordering with continuous sequence release |
| **Ordered** | 1-50ms | Buffered slot ordering, flush on newer slot or timeout |

Ordered output retains its last emitted `(slot, tx_index)` watermark. A newer-slot flush closes older slots; timeout flushing still accepts later transactions in the same slot with a higher index. Late data from closed slots or at/before the watermark is dropped with an `Ordered continuity break` warning. Events within one transaction preserve parser order. This mode cannot guarantee upstream completeness.

### 🚀 Optimization Highlights
- ✅ **Zero heap allocation** for hot paths
- ✅ **SIMD pattern matching** for all protocol detection
- ✅ **Static pre-compiled finders** for string search
- ✅ **Inline functions** with aggressive optimization
- ✅ **Event type filtering** for targeted parsing
- ✅ **Conditional Create detection** (only when needed)
- ✅ **Multiple order modes** for latency vs ordering trade-off

---

## 🔥 Quick Start

### Installation

Clone the repository:

```bash
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

Add to your `Cargo.toml`:

```toml
[dependencies]
# Default: Borsh parser
sol-parser-sdk = { path = "../sol-parser-sdk" }

# Or: Zero-copy parser (maximum performance)
sol-parser-sdk = { path = "../sol-parser-sdk", default-features = false, features = ["parse-zero-copy"] }
```

### Use crates.io

```toml
# Add to your Cargo.toml
sol-parser-sdk = "0.7.16"
```

Or with the zero-copy parser (maximum performance):

```toml
sol-parser-sdk = { version = "0.7.16", default-features = false, features = ["parse-zero-copy"] }
```

### Release Notes

#### v0.7.10

- Matches PumpFun create/create_v2 accounts by the actual instruction discriminator and event mint, including CPI calls; ambiguous matches are left unspecified.
- Stops inferring quote mint/vault/token-program fields from arbitrary remaining create_v2 accounts and preserves authoritative decoded fields.
- Restores historical CreateEvent decoding and verifies successful and failed transactions using saved mainnet fixtures.

#### v0.7.9

- Fixes PumpSwap `boost_buy_and_burn` vault and token-program backfills using its own account layout and boost vault authority.
- Preserves strict pool, direction, user and uniqueness matching, plus count-aware ALT enrichment for regular trades.
- Aligns log-derived instruction positions past Ed25519, secp256k1 and secp256r1 precompiles.
- Adds offline mainnet replay fixtures for boost buys and precompile-containing sells across RPC and both Yellowstone parsing paths.

#### v0.7.8

- Updates CPMM creator-fee collection accounts, share PDA and AmmConfig share-rate decoding.
- Fixes failed-transaction event suppression and instruction/account preservation.
- Adds RPC capture/replay examples and 13 mainnet transactions with 17 independently checked operations.
- Run `cargo run --example rpc_corpus_validate -- --list` to view real signatures; run without arguments for offline validation.

#### v0.7.7

- Adds opt-in instruction-level route analysis with CPI positions, observed transfers, native SOL/WSOL lifecycle evidence, and unknown-program coverage.
- Tracks verified StonkFun inner/graduated pool identities and migration clues without assuming every CPMM pool is StonkFun.
- Adds raw gRPC account snapshots with slot/write-version ordering and a cache capture example for RPC-free trading hot paths.

#### v0.7.6

- Anchors Raydium CLMM/CPMM, Orca Whirlpool, Meteora DLMM, and Raydium AMM account fills on the event's own pool so multi-hop routes no longer cross-fill vaults or configs from a sibling swap.
- Exposes instruction account fields on CPMM/CLMM/Whirlpool/DLMM/AMM swap events (config, vaults, mints, observation, tick arrays / bitmap extension where applicable).
- Tightens invoke matching helpers used by account fillers and merger paths.

#### v0.7.5

- Backfills Meteora DAMM v2 swap/swap2 mint, vault, payer, token-program, and remaining instruction accounts from the matching pool's real swap instruction, including optional referral handling.
- Avoids assigning accounts from a different pool or from ambiguous repeated swaps in the same pool.
- Verifies both transactions reported in solana-streamer issue #82 with live mainnet RPC regressions for the default and zero-copy parsers.

#### v0.7.4

- Adds `Protocol::StonkFun` and `Protocol::LaunchLab` as the preferred subscription names; the old `Protocol::RaydiumLaunchlab` remains compatible.
- Identifies StonkFun standard and reward pools from their official LaunchLab platform configuration accounts.
- Opt-in RPC/Yellowstone route inspection preserves CPI positions, mint flows, limits, observed amounts and opaque programs; migration events expose pool provenance. See the [StonkFun route analysis and account subscriptions](docs/ROUTE_ANALYSIS.md).
- Parses the complete current LaunchLab trade event, including reserves, all fee legs, pool status, and the three appended trade accounts.
- Supports the current 18-account LaunchLab trade instruction layout and preserves the appended accounts when merging log and instruction events.
- Adds a real mainnet StonkFun reward-pool transaction regression using signature `4Pb4vgRq6rAFi5NmMZMsfBvuwVVsvBqhySfPS3naMksujvEiGtPjxRLape7V82ZVQvxt7P8YKPCL6RSWTreMUFrY`.
- Adds a real mainnet graduated StonkFun CPMM swap regression using signature `3jiXX1AXnQfve1FCHwqUUXoM2BpS2jZEDNB7S6UXLdHGQa3VmBoWNVw9A2gTLbvEZeSU697s9XKgKDqxaR92Qqcz`.

Run the gated mainnet parser regression with:

```bash
RUN_MAINNET_TESTS=1 cargo test --test current_mainnet_transactions current_stonkfun_reward_trade_preserves_platform_quote_accounts_and_fees -- --nocapture
RUN_MAINNET_TESTS=1 cargo test --test current_mainnet_transactions current_stonkfun_graduated_cpmm_swap_parses_from_mainnet -- --nocapture
```

#### v0.7.3

- Syncs the vendored PumpFun, PumpSwap, and Pump Fees IDLs with the current official `pump-public-docs` definitions.
- Adds current creator-fee, holder-reward, quote, cashback, buyback, and pool configuration fields across instruction, log, account, RPC, gRPC, and ShredStream parsing paths.
- Preserves PumpFun holder-reward values when merging outer instructions with event-CPI data and keeps the multi-language event schema aligned.
- Rejects truncated known PumpSwap trade and CreatePool tails while retaining complete historical layouts and forward-compatible appended fields.

#### v0.7.2

- Syncs vendored Meteora DAMM v2 IDL to **0.2.4** and DBC IDL to **0.2.1**.
- Adds parsers for DAMM v2 Position Delegate and config events: `EvtUpdateDelegatePermission`, `EvtWithdrawDeadLiquidityReward`, `EvtCreateConfig`, and `EvtCreateDynamicConfig` (including the 0.2.4 `permission` field).
- Wires the new events through log, optimized matcher, and inner-instruction paths, plus `EventType` / `DexEvent` filters.
- Identifies Yellowstone V1 messages through `Message.config` and exposes all four transaction-config requests: priority fee, compute-unit limit, loaded-accounts data-size limit, and heap size. V1 ComputeBudget instructions remain ignored as required by Solana.
- Yellowstone V1 ingestion requires `yellowstone-grpc-proto >= 12.6.0` and a server running Yellowstone geyser plugin `>= 15.1.1`; older server plugins silently downgrade V1 to V0 and discard `Message.config` before transmission.
- Accelerates RPC inner-instruction Base58 decoding through the portable safe `base58-turbo` path. The saved mainnet corpus reduces end-to-end RPC parse latency by 17.7% to more than 80%, depending on instruction payload size.
- Borrows RPC log and balance metadata through the parse path to avoid redundant cloning. The final cross-protocol corpus parses PumpFun, PumpSwap, Raydium CPMM, and Meteora/Orca transactions in 7.40-11.80 us.
- Adds offline PumpFun, PumpSwap, Raydium CPMM, and Meteora DLMM/Orca RPC fixtures with exact event regression tests and a shared cross-protocol benchmark.

#### v0.7.1

- Fixes current Meteora DAMM v2 `EvtSwap2` parsing for the 180-byte layout, all three swap modes, transfer fees, reserves, and the mainnet fee-layout upgrade boundary.
- Adds the current unified `EvtLiquidityChange` discriminator and routes its `change_type` to exact AddLiquidity or RemoveLiquidity events across log, optimized, and inner-instruction paths.
- Adds reproducible mainnet RPC regression tests for DAMM v2 Swap and AddLiquidity transactions.

#### v0.7.0

- Upgrades the public Solana types to Solana 4, including Legacy, V0, and V1 transaction messages.
- Uses `wincode 0.5.5` in production ShredStream and RPC transaction decode paths. On the reproducible 21,000-byte Entry benchmark, decode latency dropped from the previous bincode baseline of 39.005 us/op to 8.956 us/op (77.0%, 4.36x faster).
- Removes unused direct dependencies and aligns the Agave dependency family. The normal dependency graph dropped from 658 to 533 package/version entries, while duplicated crate names dropped from 111 to 28.
- Requires Rust 1.91 or newer for the Solana 4 client dependency stack.
- Adds Meteora DLMM user token account context and preserves backward-compatible Serde defaults.

#### v0.6.6

- Adds the post-transaction `token_balance` to PumpFun trade events in raw mint units.
- Adds the post-transaction `sol_balance` in lamports, filled directly from transaction metadata without extra RPC calls.
- Reduces end-to-end PumpFun Yellowstone parsing latency by 34.23% on the captured benchmark fixture, from 6.7349 us to 4.4293 us.
- Adds reproducible Criterion benchmarks, a captured Yellowstone transaction fixture, and live balance-delta validation.

#### v0.6.4

- Adds `token_x_mint` and `token_y_mint` context to current Meteora DLMM swap events.
- Anchors mint enrichment to each event pool so multi-leg DLMM routes cannot reuse another pool's accounts.
- Adds reusable mainnet RPC examples captured on 2026-08-14 for direct `swap` and CPI `swap2` parsing.
- Keeps JSON produced before the new mint fields compatible with Serde deserialization.

#### v0.6.3

- Exposes current LaunchLab quote mint and global configuration context, including USD1 pools.
- Adds opt-in transaction fee, priority fee, compute budget, and SWQoS tip parsing for all providers supported by sol-trade-sdk.
- Identifies each recognized tip provider and recipient while keeping the disabled transaction-cost path allocation-free and effectively zero-cost.
- Adds reusable mainnet transaction fixtures captured on 2026-08-13 for LaunchLab USD1 and transaction-cost parsing.

#### v0.6.2

- Parses the current Meteora DLMM Anchor event-CPI prefix layout while retaining legacy suffix compatibility.
- Aligns Meteora DLMM, Raydium CPMM/AMM V4, and Orca Whirlpool event and account layouts with current official sources.
- Preserves repeated swaps and liquidity events with occurrence-aware log/instruction deduplication and stack-aware CPI merging.
- Adds reusable mainnet transaction fixtures captured on 2026-08-13 for Meteora DLMM, PumpFun, PumpSwap, Raydium, and Orca.

#### v0.6.1

- Decodes the complete current PumpSwap Buy/Sell event tail across log and CPI/inner-instruction paths: cashback, buyback fees, signed virtual quote reserves, boost eligibility, and base supply.
- Uses one validated PumpSwap trade decoder for the default and zero-copy feature configurations while preserving historical event layouts.
- Rejects truncated tails, malformed UTF-8, invalid Borsh booleans, and overflowing string bounds instead of emitting partially decoded events.
- Keeps older serialized PumpSwap events compatible by defaulting fields introduced by the buyback and boost upgrades.

#### v0.5.15

- Fixes Pump.fun `create_v2` quote mint parsing for both 16-account and 19-account layouts.
- Uses the appended quote mint only when the 19-account quote-pool tail is present; 16-account `create_v2` keeps the SOL sentinel.
- Selects the matching create/create_v2 instruction when filling accounts so later buy/sell instructions cannot overwrite create quote fields.

#### v0.5.13

- Preserves real Pump.fun WSOL quote mints (`So11111111111111111111111111111111111111112`) in gRPC and ShredStream create/trade outputs.
- Keeps the Solscan SOL sentinel (`So11111111111111111111111111111111111111111`) only for legacy or missing Pump.fun quote mint fields.
- Adds merge coverage so a placeholder SOL quote mint can be replaced by a later real WSOL quote mint from logs or instruction/account context.

#### v0.5.11

- Parses PumpSwap `create_pool` instruction args, including `index`, deposit amounts, `coin_creator`, `is_mayhem_mode`, and `is_cashback_coin`.
- Fixes PumpSwap `create_pool` instruction account mapping to match the IDL (`pool`, `creator`, `base_mint`, `quote_mint`, LP/user token accounts).
- Preserves `is_cashback_coin` when instruction-derived CreatePool data is merged with log-derived CreatePool data.
- Clarifies source semantics: the PumpSwap log `CreatePoolEvent` IDL does not carry `is_cashback_coin`; ShredStream/outer-instruction parsing can read it from instruction data, and account subscriptions can read the authoritative `Pool` account field.

#### v0.5.10

- Aligns PumpSwap `CreatePoolEvent` with the on-chain IDL: the event exposes `is_mayhem_mode` but not `is_cashback_coin`.
- Keeps PumpSwap `is_cashback_coin` on the `AccountPumpSwapPool` account event, where the on-chain `Pool` account stores it.
- Fixes the PumpSwap CreatePool log payload length check to include the final `is_mayhem_mode` byte.
- Documents that ShredStream CreatePool events cannot recover `is_cashback_coin` because Shred entries do not include account bodies.

#### v0.5.9

- Implements real Yellowstone gRPC `stop()` behavior by signaling, aborting, and awaiting the active subscription task.
- Serializes gRPC subscription lifecycle transitions so concurrent stop/re-subscribe calls cannot orphan retry loops.
- Uses a per-subscription stop signal so a new subscription cannot accidentally reset the stop state for an older task.
- Labels stream failures as `Grpc Stream error` to distinguish them from ShredStream logs.
- Makes the warmup test independent from global test execution order.

#### v0.5.8

- Adds configurable ShredStream event filter examples, including Pump.fun trade, create-trade, buy, sell, and buy-exact-sol-in presets.
- Clarifies Pump.fun IDL instruction names in `ix_name`: `buy`, `buy_v2`, `buy_exact_sol_in`, `buy_exact_quote_in_v2`, `sell`, and `sell_v2`.
- Keeps Pump.fun filter families aligned with IDL semantics: `PumpFunBuy` covers all buy instructions, `PumpFunSell` covers all sell instructions, and `PumpFunTrade` covers all buy and sell instructions.
- Ensures subscribing only to `PumpFunTrade` emits unified `DexEvent::PumpFunTrade` events on the ShredStream hot path.

#### v0.5.5

- Aligns ShredStream static-account parsing across Rust, Node.js, Python, and Go.
- Keeps V0 ALT-loaded instruction accounts on the hot path with default pubkey placeholders instead of dropping the whole instruction.
- Adds discriminator fallback when a ShredStream outer program id is ALT-loaded and not present in the static account table.
- Improves Pump.fun ShredStream parity for create/create_v2, v2 short-account trades, and event-type filtering.
- Refreshes multi-protocol routing and account-fill documentation for Pump.fun, PumpSwap, Pump Fees, Raydium, Orca, and Meteora paths.

#### v0.5.4

- Emits Pump.fun `create` and `create_v2` as one canonical `PumpFunCreate` event.
- Treats `PumpFunCreate` and `PumpFunCreateV2` filters as the same create-family subscription.
- Keeps create_v2 account fields on the canonical create event so bots do not need to handle two event variants.
- Prevents duplicate new-mint callbacks from gRPC log + instruction parsing.

#### v0.5.3

- Preserves real Pump.fun v2 `ix_name` values, including `buy_v2`, `sell_v2`, and `buy_exact_quote_in_v2`.
- Improves ShredStream Pump.fun v2 parsing with best-effort short-account handling for buy, sell, and exact-quote instructions.
- Treats Pump.fun buy-family filters symmetrically, so `PumpFunBuy` and `PumpFunBuyExactSolIn` subscriptions both match the compatible buy variants.
- Keeps ShredStream ALT/default-account best-effort parsing for outer instructions while preserving the CPI/inner-only limitation.

### Performance Testing

Transaction-cost parsing is opt-in and does not run on the default DEX event path:

```rust
use sol_parser_sdk::{parse_yellowstone_transaction_cost, SwqosProvider};

let cost = parse_yellowstone_transaction_cost(&transaction, &meta).unwrap();
let jito_tip = cost.tip_lamports_for(SwqosProvider::Jito);
```

Yellowstone and RPC results use status metadata for the authoritative transaction fee and
confirmed tip status. `transaction_fee_lamports` already includes the priority fee; relay tips
are separate transfers and are added only in `total_fee_and_tip_lamports`. Tip recipients for every
SWQoS provider supported by `sol-trade-sdk` are recognized automatically, and each `TipPayment`
identifies its provider. ShredStream can expose requested compute-budget values and outer tip
transfers, but cannot confirm fees, execution, inner instructions, or ALT-loaded tip recipients.

Test parsing latency with the optimized examples:

```bash
# PumpFun with detailed metrics (per-event + 10s stats)
cargo run --example pumpfun_with_metrics --release

# PumpSwap with detailed metrics (per-event + 10s stats)
cargo run --example pumpswap_with_metrics --release

# PumpSwap ultra-low latency test
cargo run --example pumpswap_low_latency --release

# PumpSwap with MicroBatch ordering
cargo run --example pumpswap_ordered --release

# Expected output:
# gRPC receive time: 1234567890 μs
# Event receive time: 1234567900 μs
# Latency: 10 μs  <-- Ultra-low latency!
```

### Examples

| Description | Run Command | Source Code |
|-------------|-------------|-------------|
| **PumpFun** | | |
| PumpFun event parsing with metrics | `cargo run --example pumpfun_with_metrics --release` | [examples/pumpfun_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_with_metrics.rs) | `v0.7.16` |
| PumpFun trade type filtering | `cargo run --example pumpfun_trade_filter --release` | [examples/pumpfun_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter.rs) | `v0.7.16` |
| PumpFun trade with ordered mode | `cargo run --example pumpfun_trade_filter_ordered --release` | [examples/pumpfun_trade_filter_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter_ordered.rs) | `v0.7.16` |
| Quick PumpFun connection test | `cargo run --example pumpfun_quick_test --release` | [examples/pumpfun_quick_test.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_quick_test.rs) | `v0.7.16` |
| Parse PumpFun tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` | [examples/parse_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pump_tx.rs) | `v0.7.16` |
| Parse PumpFun quote-mint cases | `TX_SIGNATURES=<sig1,sig2> cargo run --example parse_pumpfun_quote_cases --release` | [examples/parse_pumpfun_quote_cases.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpfun_quote_cases.rs) | `v0.7.16` |
| Debug PumpFun transaction | `cargo run --example debug_pump_tx --release` | [examples/debug_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pump_tx.rs) | `v0.7.16` |
| **PumpSwap** | | |
| PumpSwap events with metrics | `cargo run --example pumpswap_with_metrics --release` | [examples/pumpswap_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_with_metrics.rs) | `v0.7.16` |
| PumpSwap ultra-low latency | `cargo run --example pumpswap_low_latency --release` | [examples/pumpswap_low_latency.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_low_latency.rs) | `v0.7.16` |
| PumpSwap with MicroBatch ordering | `cargo run --example pumpswap_ordered --release` | [examples/pumpswap_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_ordered.rs) | `v0.7.16` |
| Parse PumpSwap tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` | [examples/parse_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpswap_tx.rs) | `v0.7.16` |
| Debug PumpSwap transaction | `cargo run --example debug_pumpswap_tx --release` | [examples/debug_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pumpswap_tx.rs) | `v0.7.16` |
| **Meteora DAMM** | | |
| Meteora DAMM V2 events | `cargo run --example meteora_damm_grpc --release` | [examples/meteora_damm_grpc.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_grpc.rs) | `v0.7.16` |
| Parse Meteora DAMM tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` | [examples/parse_meteora_damm_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_meteora_damm_tx.rs) | `v0.7.16` |
| **Non-Pump DEX dry-run scenarios** | | |
| LaunchLab migration filter | `cargo run --example raydium_launchlab_migration` | [examples/raydium_launchlab_migration.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_launchlab_migration.rs) | `v0.7.16` |
| Raydium CPMM new pool filter | `cargo run --example raydium_cpmm_new_pool` | [examples/raydium_cpmm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_cpmm_new_pool.rs) | `v0.7.16` |
| Raydium CLMM token price math | `cargo run --example raydium_clmm_token_price` | [examples/raydium_clmm_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_clmm_token_price.rs) | `v0.7.16` |
| Orca Whirlpool token price math | `cargo run --example orca_whirlpool_token_price` | [examples/orca_whirlpool_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/orca_whirlpool_token_price.rs) | `v0.7.16` |
| Meteora DAMM new pool baseline | `cargo run --example meteora_damm_new_pool` | [examples/meteora_damm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_new_pool.rs) | `v0.7.16` |
| Meteora DBC token price math | `cargo run --example meteora_dbc_token_price` | [examples/meteora_dbc_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_dbc_token_price.rs) | `v0.7.16` |
| Wallet trade filter | `cargo run --example wallet_trade_filter` | [examples/wallet_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/wallet_trade_filter.rs) | `v0.7.16` |
| gRPC latency slot compare config | `cargo run --example grpc_latency_slot_compare` | [examples/grpc_latency_slot_compare.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/grpc_latency_slot_compare.rs) | `v0.7.16` |
| **Account subscription** | | |
| Token account balance updates | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` | [examples/token_balance_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_balance_listen.rs) | `v0.7.16` |
| Nonce account state changes | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` | [examples/nonce_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/nonce_listen.rs) | `v0.7.16` |
| Mint account info | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` | [examples/token_decimals_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_decimals_listen.rs) | `v0.7.16` |
| PumpSwap pool accounts via memcmp | `cargo run --example pumpswap_pool_account_listen --release` | [examples/pumpswap_pool_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_pool_account_listen.rs) | `v0.7.16` |
| All ATAs for mints | `cargo run --example mint_all_ata_account_listen --release` | [examples/mint_all_ata_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/mint_all_ata_account_listen.rs) | `v0.7.16` |
| **ShredStream** | | |
| Jito ShredStream subscription | `cargo run --example shredstream_example --release` | [examples/shredstream_example.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/shredstream_example.rs) | `v0.7.16` |
| **Utility** | | |
| Dynamic subscription filters | `cargo run --example dynamic_subscription --release` | [examples/dynamic_subscription.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/dynamic_subscription.rs) | `v0.7.16` |
| Debug PumpSwap account filling | `cargo run --example test_account_filling --release` | [examples/test_account_filling.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/test_account_filling.rs) | `v0.7.16` |

### Basic Usage

```rust
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol,
    TransactionFilter, YellowstoneGrpc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create gRPC client with default config (Unordered mode)
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;
    
    // Or with custom config for ordered events
    let config = ClientConfig {
        order_mode: OrderMode::MicroBatch,  // Low latency + ordering
        micro_batch_us: 100,                // 100μs batch window
        ..ClientConfig::default()
    };
    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    let protocols = vec![Protocol::PumpFun, Protocol::PumpSwap, Protocol::RaydiumCpmm];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    // Filter before parsing for the lowest-latency path.
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpSwapBuy,
        EventType::PumpSwapSell,
        EventType::RaydiumCpmmSwap,
    ]);

    // Subscribe and get lock-free queue
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    ).await?;

    // Consume events with minimal latency
    tokio::spawn(async move {
        let mut spin_count = 0;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                // Process event (10-20μs latency!)
                println!("{:?}", event);
            } else {
                // Hybrid spin-wait strategy
                spin_count += 1;
                if spin_count < 1000 {
                    std::hint::spin_loop();
                } else {
                    tokio::task::yield_now().await;
                    spin_count = 0;
                }
            }
        }
    });

    Ok(())
}
```

### ShredStream Usage (Jito)

ShredStream provides ultra-low latency (~50-100ms faster than gRPC) by directly subscribing to Jito's ShredStream service:

```rust
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::shredstream::{ShredStreamClient, ShredStreamConfig};
use sol_parser_sdk::DexEvent;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create ShredStream client
    let client = ShredStreamClient::new("http://127.0.0.1:10800").await?;

    // Or with custom config
    let config = ShredStreamConfig {
        connection_timeout_ms: 5000,
        request_timeout_ms: 30000,
        max_decoding_message_size: 1024 * 1024 * 1024,
        reconnect_delay_ms: 1000,
        max_reconnect_attempts: 0, // 0 = infinite reconnect
    };
    let client = ShredStreamClient::new_with_config("http://127.0.0.1:10800", config).await?;

    // Subscribe with SDK-side filtering before event conversion.
    // Use `client.subscribe().await?` to receive every supported event.
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpSwapBuy,
        EventType::RaydiumCpmmSwap,
    ]);
    let queue = client.subscribe_with_filter(Some(event_filter)).await?;

    // Consume events
    loop {
        if let Some(event) = queue.pop() {
            match &event {
                DexEvent::PumpFunTrade(e) => {
                    println!("PumpFun Trade: mint={}, is_buy={}", e.mint, e.is_buy);
                }
                DexEvent::PumpSwapBuy(e) => {
                    println!("PumpSwap Buy: pool={}", e.pool);
                }
                _ => {}
            }
        } else {
            std::hint::spin_loop();
        }
    }
}
```

**ShredStream Limitations:**
- The default parser requires resolved instruction keys. Supply cached ALT addresses with `parse_transaction_dex_events_with_loaded_addresses` or `with_address_lookup_resolver`; unresolved instructions are skipped. An explicit `best_effort` API retains provisional legacy behavior.
- No execution metadata: supported official routes and known wrappers emit explicit route intents, while arbitrary CPI results and confirmed fills require the execution-metadata path.
- No block_time - always 0
- tx_index is entry-level, not slot-level

---

## 🏗️ Supported Protocols

### DEX Protocols
- ✅ **PumpFun** - Meme coin trading (ultra-fast zero-copy path, incl. v2 instructions)
- ✅ **Pump Fees** - Pump fee-sharing configuration events
- ✅ **PumpSwap** - PumpFun swap protocol
- ✅ **LaunchLab** - Token launch platform, including StonkFun attribution
- ✅ **Raydium AMM V4** - Automated Market Maker
- ✅ **Raydium CLMM** - Concentrated Liquidity
- ✅ **Raydium CPMM** - Concentrated Pool
- ✅ **Orca Whirlpool** - Concentrated liquidity AMM
- ✅ **Meteora Pools** - Dynamic AMM
- ✅ **Meteora DAMM v2** - Dynamic AMM V2
- ✅ **Meteora DLMM** - Dynamic Liquidity Market Maker
- 🚧 **Meteora DBC** - Program id and filters added; transaction/account parser pass pending

### Event Types
Each protocol supports:
- 📈 **Trade/Swap Events** - Buy/sell transactions
- 💧 **Liquidity Events** - Deposits/withdrawals
- 🏊 **Pool Events** - Pool creation/initialization
- 🎯 **Position Events** - Open/close positions (CLMM)

### Non-Pump DEX Support Matrix

| Protocol | Events | Accounts | Examples | Language constants |
|----------|--------|----------|----------|--------------------|
| LaunchLab | Trade, pool create, migrate | Pool/config snapshots | Migration, buy/sell oracle planned | Rust, Node, Python, Go |
| Raydium CPMM | Swap, deposit, withdraw, initialize | AmmConfig, PoolState | New pool, token price | Rust, Node, Python, Go |
| Raydium CLMM | Swap, pool, position, liquidity | AmmConfig, PoolState, TickArray | Token price | Rust, Node, Python, Go |
| Raydium AMM V4 | Swap, deposit, withdraw, initialize2 | Pending | Token price oracle planned | Rust, Node, Python, Go |
| Orca Whirlpool | Swap, liquidity, pool init | Whirlpool, Position, TickArray, FeeTier, Config | Token price | Rust, Node, Python, Go |
| Meteora Pools | Swap, liquidity, pool create, fees | Pending | Token price oracle planned | Rust, Node, Python, Go |
| Meteora DAMM V2 | Swap, liquidity, position | Pending | New pool, token price oracle planned | Rust, Node, Python, Go |
| Meteora DLMM | Swap, liquidity, bin/position | Pool/bin array/bitmap snapshots | Token price oracle planned | Rust, Node, Python, Go |
| Meteora DBC | Swap, initialize pool, curve complete (Rust log parser) | Pending | Token price, migration oracle planned | Rust, Node, Python, Go |

The canonical cross-language baseline is tracked in
[`protocols/canonical.json`](protocols/canonical.json). Raw account updates are opt-in via `AccountRawSnapshot`; specialized snapshots cover LaunchLab pools/configs, DLMM pools/bin arrays/bitmap extensions, Whirlpool dynamic arrays/adaptive Oracles and CLMM bitmap extensions.

---

## ⚡ Performance Features

### Zero-Copy Parsing
```rust
// Stack-allocated 512-byte buffer for PumpFun Trade
const MAX_DECODE_SIZE: usize = 512;
let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

// Decode directly to stack, no heap allocation
general_purpose::STANDARD
    .decode_slice(data_part.as_bytes(), &mut decode_buf)
    .ok()?;
```

### SIMD Pattern Matching
```rust
// Pre-compiled SIMD finders (initialized once)
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));

// 3-10x faster than .contains()
if PUMPFUN_FINDER.find(log_bytes).is_some() {
    return LogType::PumpFun;
}
```

### Event Type Filtering
```rust
// Ultra-fast path for single event type
if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
    if log_type == LogType::PumpFun {
        return parse_pumpfun_trade(  // Zero-copy path
            log, signature, slot, block_time, grpc_recv_us, is_created_buy
        );
    }
}
```

### Lock-Free Queue
```rust
// ArrayQueue with 100,000 capacity
let queue = Arc::new(ArrayQueue::<DexEvent>::new(100_000));

// Non-blocking push/pop (no mutex overhead)
let _ = queue.push(event);
if let Some(event) = queue.pop() {
    // Process event
}
```

---

## 🎯 Event Filtering

Reduce processing overhead by filtering specific events:

### Example: Trading Bot
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunBuy,
    EventType::PumpFunSell,
    EventType::PumpFunBuyExactSolIn,
    EventType::PumpSwapBuy,
    EventType::PumpSwapSell,
    EventType::RaydiumLaunchlabTrade,
    EventType::RaydiumCpmmSwap,
    EventType::RaydiumAmmV4Swap,
    EventType::RaydiumClmmSwap,
    EventType::OrcaWhirlpoolSwap,
    EventType::MeteoraPoolsSwap,
    EventType::MeteoraDammV2Swap,
    EventType::MeteoraDlmmSwap,
]);
```

### Example: Pool Monitor
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunCreate,
    EventType::PumpFeesUpdateFeeShares,
    EventType::PumpSwapCreatePool,
    EventType::AccountPumpSwapPool,
    EventType::RaydiumCpmmInitialize,
    EventType::RaydiumClmmCreatePool,
    EventType::OrcaWhirlpoolPoolInitialized,
    EventType::MeteoraPoolsPoolCreated,
    EventType::MeteoraDammV2CreatePosition,
    EventType::MeteoraDlmmInitializePool,
]);
```

`PumpSwapCreatePool` includes `is_mayhem_mode`. For `is_cashback_coin`,
ShredStream/outer-instruction parsing reads the flag from the `create_pool`
instruction args, while log-only `CreatePoolEvent` payloads keep the default
`false` because the log event IDL does not carry this field. The authoritative
account value is also available from
`PumpSwapPoolAccountEvent.pool.is_cashback_coin`.

**Performance Impact:**
- 60-80% reduction in processing
- Lower memory usage
- Reduced network bandwidth

---

## 🔧 Advanced Features

### Create+Buy Detection
Automatically detects when a token is created and immediately bought in the same transaction:

```rust
// Detects "Program data: GB7IKAUcB3c..." pattern
let has_create = detect_pumpfun_create(logs);

// Sets is_created_buy flag on Trade events
if has_create {
    trade_event.is_created_buy = true;
}
```

### Pump.fun Bonding Curve v2 (buy_v2 / sell_v2 / buy_exact_quote_in_v2)

The SDK recognizes Pump.fun's new v2 trading instructions introduced in the Bonding Curve upgrade. Event logs from `buy_v2`, `sell_v2`, and `buy_exact_quote_in_v2` are parsed with the same zero-copy path and mapped to the existing event types:

| ix_name in TradeEvent | DexEvent Variant |
|----------------------|-----------------|
| `"buy"` / `"buy_v2"` / `"buy_exact_quote_in"` / `"buy_exact_quote_in_v2"` | `DexEvent::PumpFunBuy` |
| `"sell"` / `"sell_v2"` | `DexEvent::PumpFunSell` |
| `"buy_exact_sol_in"` | `DexEvent::PumpFunBuyExactSolIn` |

No changes are required in your event handling code — v2 events arrive through the same `PumpFunTradeEvent` struct with the correct `ix_name` field populated. Instruction discriminators for `buy_v2` (`[184, 23, 238, 97, 103, 197, 211, 61]`), `sell_v2` (`[93, 246, 130, 60, 231, 233, 64, 178]`), and `buy_exact_quote_in_v2` (`[194, 171, 28, 70, 104, 77, 91, 47]`) are recognized at the instruction parser level.

`CreateEvent` also exposes `quote_mint` and `virtual_quote_reserves`, so USDC quote pools can be distinguished from native SOL pools and initialized with the correct quote-side reserve.

### Dynamic Subscription
Update filters without reconnecting:

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### Order Modes
Choose the right balance between latency and ordering:

```rust
use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

// Ultra-low latency (no ordering guarantee)
let config = ClientConfig {
    order_mode: OrderMode::Unordered,
    ..ClientConfig::default()
};

// Low latency with micro-batch ordering (50-200μs)
let config = ClientConfig {
    order_mode: OrderMode::MicroBatch,
    micro_batch_us: 100,  // 100μs batch window
    ..ClientConfig::default()
};

// Stream ordering with continuous sequence release (0.1-5ms)
let config = ClientConfig {
    order_mode: OrderMode::StreamingOrdered,
    order_timeout_ms: 50,  // Timeout for incomplete sequences
    ..ClientConfig::default()
};

// Buffered slot ordering (flush on newer slot or timeout)
let config = ClientConfig {
    order_mode: OrderMode::Ordered,
    order_timeout_ms: 100,
    ..ClientConfig::default()
};
```

### Performance Metrics
```rust
let config = ClientConfig {
    enable_metrics: true,
    ..ClientConfig::default()
};

let grpc = YellowstoneGrpc::new_with_config(endpoint, token, config)?;
```

---

## 📁 Project Structure

```
src/
├── core/
│   └── events.rs          # Event definitions
├── grpc/
│   ├── client.rs          # Yellowstone gRPC client
│   ├── buffers.rs         # SlotBuffer & MicroBatchBuffer
│   └── types.rs           # OrderMode, ClientConfig, filters
├── shredstream/
│   ├── client.rs          # Jito ShredStream client
│   ├── config.rs          # ShredStreamConfig
│   └── proto/             # Protobuf definitions
├── logs/
│   ├── optimized_matcher.rs  # SIMD log detection
│   ├── zero_copy_parser.rs   # Zero-copy parsing
│   ├── pumpfun.rs         # PumpFun parser
│   ├── raydium_*.rs       # Raydium parsers
│   ├── orca_*.rs          # Orca parsers
│   └── meteora_*.rs       # Meteora parsers
├── instr/
│   └── *.rs               # Instruction parsers
├── warmup/
│   └── mod.rs             # Parser warmup (auto-called)
└── lib.rs
```

---

## 🚀 Optimization Techniques

### 1. **SIMD String Matching**
- Replaced all `.contains()` with `memmem::Finder`
- 3-10x performance improvement
- Pre-compiled static finders

### 2. **Zero-Copy Parsing**
- Stack-allocated buffers (512 bytes)
- No heap allocation in hot path
- Inline helper functions

### 3. **Event Type Filtering**
- Early filtering at protocol level
- Conditional Create detection
- Single-type ultra-fast path

### 4. **Lock-Free Queue**
- ArrayQueue (100K capacity)
- Spin-wait hybrid strategy
- No mutex overhead

### 5. **Aggressive Inlining**
```rust
#[inline(always)]
fn read_u64_le_inline(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[offset..offset + 8]);
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}
```

---

## 📊 Benchmarks

### Parsing Latency (Release Mode)
| Protocol | Avg Latency | Min | Max |
|----------|-------------|-----|-----|
| PumpFun Trade (zero-copy) | 10-15μs | 8μs | 20μs |
| Raydium AMM V4 Swap | 15-20μs | 12μs | 25μs |
| Orca Whirlpool Swap | 15-20μs | 12μs | 25μs |

### SIMD Pattern Matching
| Operation | Before (contains) | After (SIMD) | Speedup |
|-----------|------------------|--------------|---------|
| Protocol detection | 50-100ns | 10-20ns | 3-10x |
| Create event detection | 150ns | 30ns | 5x |

---

## 📄 License

MIT License

## 📞 Contact

- **Repository**: https://github.com/0xfnzero/sol-parser-sdk
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE

---

## ⚠️ Performance Tips

1. **Use Event Filtering** - Filter at the source for 60-80% performance gain
2. **Run in Release Mode** - `cargo build --release` for full optimization
3. **Test with sudo** - `sudo cargo run --example basic --release` for accurate timing
4. **Monitor Latency** - Check `grpc_recv_us` and queue latency in production
5. **Tune Queue Size** - Adjust ArrayQueue capacity based on your throughput
6. **Spin-Wait Strategy** - Tune spin count (default: 1000) for your use case

## 🔬 Development

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Generate docs
cargo doc --open
```

Shared PumpFun create/create_v2 mainnet fixtures, replay instructions and verification limits are documented in the [validation guide](https://github.com/0xfnzero/sol-parser-sdk-golang/tree/main/validation/pumpfun_create_20261007).

## Usage documentation

- [Account subscriptions](docs/ACCOUNT_SUBSCRIPTIONS.md)
- [Route analysis](docs/ROUTE_ANALYSIS.md)
- [CPMM creator-fee](docs/cpmm-creator-fee-share.md)
- [Pump/PumpSwap account layouts](docs/PUMP_PUMP_AMM_PARSER_ACCOUNTS.md)
- [gRPC versus RPC](docs/grpc-vs-rpc.md)
- [Latency troubleshooting](docs/troubleshooting-latency.md)


### Pump routes in ShredStream (offline, before execution)

The DEX shred parser recognizes official `multi_hop_swap` and
`multi_hop_curve_swap`, plus the captured RapidBuy (`rapidV5DA…`, 41-byte layout)
and BuyPumpDelta (`J7pourVw…`, 28-byte layout) wrapper versions. It emits
`DexEvent::PumpRouteIntent`, with the original instruction index, user, input and
output accounts, route hops and original instruction data. A two-hop route is
**one route event with two hops**; three separate wrapper calls produce three
route events. Unknown programs or changed layouts are not guessed by scanning
for embedded Pump bytes.

**These events are intents, not confirmed fills.** They contain no execution
status, actual fill amounts, reserves or post balances. `amount_in` and
`min_amount_out` are explicit instruction constraints where decoded; BuyPumpDelta
chooses its input at runtime, so these fields are `None`. Failed or conditional
execution cannot be determined from raw shreds. Use execution metadata for fills.

Subscribe with `EventType::PumpRouteIntent`. Broad `PumpFunTrade` subscriptions
also receive route intents; official `MultiHopSwap` routes can contain both Pump
families and also match `PumpSwapTrade`. Direction-specific buy/sell and create
filters do not match route intents. `exclude_types([PumpRouteIntent])` suppresses
all intents. Consumers must handle the new `DexEvent::PumpRouteIntent` variant
rather than assuming every callback is a trade variant.

`filter: None` already includes route intents; changing the filter does not
convert them into `PumpFunTrade`. For RapidBuy and BuyPumpDelta, `is_buy` is
`Some(true)`, relative to `hops[0].base_mint`; the payment mint is
`hops[0].quote_mint`. RapidBuy also exposes
`ix_name = Some("buy_exact_quote_in_v2")` from its validated embedded
instruction. BuyPumpDelta's expected method is unknown (`None`). Both hints
are `None` for multi-hop routes. These are pre-execution hints, not proof of
a successful CPI. Older serialized events deserialize with unknown hints.

```rust,ignore
if let DexEvent::PumpRouteIntent(intent) = event {
    println!("buy intent: {:?}, expected method: {:?}",
        intent.is_buy, intent.ix_name);
    for hop in &intent.hops {
        println!("base: {}, quote: {}", hop.base_mint, hop.quote_mint);
    }
}
```

The new optional fields preserve JSON read compatibility, but downstream Rust
code constructing `PumpRouteIntentEvent` literals must supply the two fields.

ALT addresses must come from a prewarmed, slot-aware local cache. Both SDKs expose
`shredstream::parse_transaction_dex_events_with_loaded_addresses`: pass all loaded
writable addresses, then all loaded readonly addresses, in the message lookup
order. Count/index errors return before appending events. The caller must ensure
address identity and lookup-table activation at the transaction slot. The default
parser skips unresolved instruction accounts instead of emitting placeholder
pubkeys. `parse_transaction_dex_events_best_effort` retains the older provisional
behavior for direct instructions; route adapters always require resolved keys.

High-level clients accept `with_address_lookup_resolver(|transaction, slot| ...)`,
returning `LoadedAddresses` from your cache. Cache errors skip the transaction;
without a resolver, fully static instructions can still parse. Do not perform RPC
or blocking I/O in this synchronous callback. The SDK parsing path performs no
RPC, simulation, or transaction submission.

Regression coverage includes the six original signed mainnet transaction wires,
ALT/filter/error cases, signed static official routes, and malformed/truncated
instructions. Run `cargo test --test pump_shred_routes`. The tests are offline;
synthetic signing validates extraction and does not simulate program execution.

### Repeatable offline Pump simulations

Run the native Rust example and tests:

```sh
cargo run --offline --example pump_offline_bank
cargo test --offline --test pump_offline_bank --test pump_shred_routes
```

The example signs and executes 20 route scenarios against frozen deployed programs
in LiteSVM twice, checks rollback/signature/ALT behavior and validates SDK parsing.
The integration test reuses the example code. No Python, RPC calls or broadcasts are
required. See [configuration, snapshot provenance and scope](examples/offline_bank/README.md).


## Reproducible local hot-path latency

Run the native Rust example with a configuration file (no RPC):

```sh
cargo run --release --example hotpath_latency -- examples/hotpath_latency.json
```

The JSON configuration controls `warmup`, `samples`, and `batch`. Output includes
p50/p95/p99 nanoseconds per operation and the build profile. Use `--release` for
performance comparisons and run on an otherwise idle machine. Re-run with the
same configuration and fixtures before comparing changes.

Six captured signed transactions are decoded and signature-verified before the
measurement. Strict parsing uses cached loaded addresses and validates expected
event counts. Two unsigned synthetic cases measure irrelevant-program and
unknown-discriminator rejection. Output-vector storage is reused. Measurements
exclude signature verification, fixture loading, network reception, and bank
execution; they do not establish production ShredStream latency.
