# Release history

## v0.9.0 — 2026-10-22

Synchronize SDK code, examples, version navigation and AI release rules. Read the [release and migration notes](RELEASE_NOTES.md) for callback removals, V3 defaults and the Go /v9 migration.

These notes preserve behavior at release time, not the current API contract. See the [README](README.md) and [current documentation](docs/README.md) for usage.

## v0.7.19 — Wrapped Pump.fun rules and gRPC execution context

Adds opt-in offline rule analysis and Shred instruction reconstruction with `third_program` provenance. The parsing hot path performs no RPC; unmatched instructions remain raw and executed amounts are not inferred. Fixes mint-scoped create-buy context and preserves versioned Pump.fun IDL method names. Includes signed mainnet transaction replays.

[Wrapped Pump.fun: offline rules, hot-path boundaries, and real-transaction replays](docs/wrapped-pumpfun.md)

## v0.7.18 — Shred metadata passthrough and ordering scope

Preserve and verify transaction position and receive time through raw instruction callbacks. Entries indexes are message-local, unlike Yellowstone node indexes; use signatures for cross-message deduplication. This release clarifies API documentation and adds regression protection without changing metadata fields or numbering behavior.


## v0.7.18 — DEX-scoped RawInstruction and typed DEX identifiers

Raw instructions now respect DEX subscriptions using programs referenced by each instruction. `RawInstructionDexType` provides typed identifiers with unchanged JSON strings. Unknown router layouts do not establish pool addresses, trade direction, or method versions.


## v0.7.18 — Pump execution events and current replay

Pump TradeEvent CPI decoding now accepts the official Anchor event prefix as well as the historical encoding. RPC/gRPC compiled instruction budgets and minimum outputs are kept in instruction fields; `sol_amount`, `quote_amount`, and `token_amount` are populated from execution events. If both logs and event CPIs are unavailable these execution fields remain zero (unavailable, not a proven zero fill). Non-SOL quote amounts are not SOL amounts.

Run `cargo run --example pump_execution_replay` from this repository for offline replay of transactions fetched on 2026-10-19. The example prints results with full, absent, and truncated logs; `cargo test --test pump_execution` asserts the execution amounts and identities. Append a recent signature to fetch another transaction; `SOLANA_RPC_URL` selects your RPC. RPC requests support transaction v1.

Recent captured signatures:
- Delta SellV2, slot 458542546: `4ARos3Yq1SMNE3izDpy9YdKrRgMmD7UCDxvnVrBRaGCJHiifPHSmQWc4DPfjUaEUcxS7puGmEN64K7EZdeXLJaj2`.
- Rapid create + buy, slot 458539974: `4A2UJ139R7YRvtDRUuTsbj83EQwkWZehP9mvjc9kgzJEaVLCLRKXPtw6CYwdovduVR8cYcUZ6spxHnKKKfkKhdn2`.
- BuyV3, slot 458540504: `fDy5VD3yMZAvqTjSfvTsbMK3SWBeqHhgoNsZnobFFr13cr5ppqREZsbxjtohzVXSbapP66A7i4Rs9BiVuqeqLCJ`.
- PumpSwap buy, slot 458542555: `TNSxB6uf4Qjp7ZeK7UTkDCZd6XxLbaKf8qkpZqvEhCPWwQtUzMqtTurunF9gwae64M1FKx9xjd63XCSm9XpkXLY`.

Older five-combination multi-hop fixtures remain regression tests, not newly captured mainnet examples. Regressions check per-leg identities and execution amounts, missing/truncated logs, and failed-transaction suppression.

## v0.7.18 — raw Shred instructions

Default Shred parsing emits `RawInstruction` for unsupported third-party programs, including Rapid/Delta, without inferring direction, amounts, or internal buy methods. Infrastructure programs (System, Token, ATA, ComputeBudget, etc.) are omitted by default. Supported official DEX parsers are unchanged; official Pump/PumpSwap multi-hop instructions retain `PumpRouteIntent`.

The payload contains `program_id`, `instruction_index`, `accounts`, `account_indices`, `instruction_data`, and transaction metadata. Account order and duplicates are preserved. Unresolved ALT addresses are null/None, with original indices retained; supply cached loaded addresses or a resolver. No hot-path RPC is performed. Data is copied verbatim without requiring a known layout.

Run `cargo run --example raw_instruction_filter` to compare PumpFun-only, Raydium CPMM-only, and all-raw subscriptions against signed offline transactions; the output includes `dexes`.

The raw event retains `metadata.tx_index` and `metadata.grpc_recv_us` from the parser arguments (streamer names the receive field `metadata.recv_us`). The default Entries client index is local to one received Entries message and restarts at zero for the next message; it is not a confirmed slot-wide transaction index. `instruction_index` orders outer instructions within a transaction. Receive time is a local microsecond timestamp, not an execution-order guarantee. Standalone replay APIs may default these values to zero; pass the upstream values when available.

Without a filter, unknown third-party instructions are returned. Broad trade/swap filters (for example `PumpFunTrade` or `RaydiumCpmmSwap`) also return raw instructions associated with that DEX; direction-specific filters do not infer a buy or sell. Explicit `RawInstruction` selects all outer instructions only when no protocol restriction is set; streamer protocol restrictions always apply. Excluding `RawInstruction` wins. `dex_type` uses the public `RawInstructionDexType` enum (typed string constants in Go); JSON values remain unchanged, such as `"pumpfun"` and `"raydium_cpmm"`. `dexes` contains `{ dex_type, program_id, pool }` for each recognized DEX program targeted or referenced by this instruction, not other instructions in the transaction. Multi-DEX wrappers retain every association. Unknown or unresolved ALT references cannot satisfy a DEX subscription. `pool` is currently null: an unknown wrapper layout cannot establish the pool account. A program reference is not proof of an executed CPI. Raw callbacks are not fills.

Wrapper adapter functions remain explicit compatibility helpers only; default Shred parsing does not call them. The released-version history below describes previous behavior.

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

## Historical README integration notes

## Pump routes in ShredStream (offline, before execution)

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
