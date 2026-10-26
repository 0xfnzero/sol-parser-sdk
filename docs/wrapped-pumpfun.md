# Wrapped Pump.fun transactions

## Callback contract

Third-party input is opaque. The SDK does not infer wrapper ABIs, reorder wrapper
accounts or reconstruct a Pump buy/sell instruction. Consumers interpret the
original input in the trade event's `third_program`.

On Shred, an unknown outer program referencing Pump.fun in **that instruction's**
accounts produces a `PumpFunTrade` candidate with `parse_status=Unresolved`.
This does not establish a trade, direction, mint, pool or amount: the program may
only prepare accounts. Missing trade fields retain defaults, not proven zero fills.
Broad PumpFunTrade subscriptions receive candidates; buy/sell-only subscriptions
do not. Other unknown instructions are skipped, without scanning all DEX families.

On gRPC/RPC, supplied execution metadata and actual Pump CPI instructions determine
the trade. Its exact originating outer instruction fills `third_program`.
Direct calls have no third-party payload. A successful invocation can be
`ExecutionConfirmed` even when some execution amounts are unavailable.

One outer instruction can invoke zero, one or several trades. Compare signature
and outer instruction index rather than expecting equal Shred/gRPC event counts.

## Original input and metadata

`third_program: Option<RawInstructionPayload>` contains:

- `program_id`: original outer program.
- `instruction_index`: outer instruction position.
- `accounts`: original ordered addresses, including duplicates and unresolved `None` entries.
- `account_indices`: original indices into the transaction account table.
- `instruction_data`: unchanged instruction bytes.

Transaction metadata stays on the enclosing event, including `tx_index`,
`grpc_recv_us` and `recent_blockhash`. A local Shred index is not confirmed
slot-wide execution order. Cached loaded addresses may be supplied for ALT
resolution; the parsing hot path never calls RPC.

## Pump.fun program ID stored in ALT

`pump_ix::parse_transaction_dex_events_with_filter` reads static account keys only.
If a wrapper's Pump.fun program reference lives in an address lookup table, this
entry point cannot identify the referenced DEX and may return no events. The
wrapper's name or discriminator is insufficient evidence. `third_program` retains
an identified wrapper's input; it does not resolve missing addresses.

For a manually supplied transaction, resolve its lookups from a prewarmed local
cache, then call the existing loaded-addresses entry point:

```rust,ignore
// `loaded` is a slot-valid local-cache result for this transaction's lookups.
// Both vectors follow message lookup order; never perform RPC in this call path.
sol_parser_sdk::shredstream::pump_ix::parse_transaction_dex_events_with_loaded_addresses(
    &created_tx,
    &loaded.writable,
    &loaded.readonly,
    sign,
    observer_info.create_slot,
    888,
    observer_info.grpc_recv_us,
    event_filter,
    &mut events,
)?;
```

For SDK-managed subscriptions, configure `ShredStreamClient::with_address_lookup_resolver`
with the same local cache. Preload/update tables outside the receive task. Supplying
another transaction's loaded addresses or guessing missing keys is unsafe; a cache
miss needs to be tracked separately from a successfully parsed empty result.

The signed reports `2EUPDVEH…` and `38NiGzWa…` use GMGN's
`GMgnVFR8Jb39LoXsEVzb3DvBy3ywCmdmJquHUy1Lrkqb` program and lookup table
`58tUNfyBtwkMeU8gpyZYKEo4Hyb4E25ui12w5vKYQx3P`. Each contains **three**
successful Pump.fun `buy` CPIs, from outer instructions 3, 6 and 9 (zero-based).
With static keys alone, both yield zero Pump callbacks; with the saved ALT snapshot,
each yields three `Unresolved` Shred candidates with the original `third_program`.
gRPC yields three `ExecutionConfirmed` buys with actual nonzero amounts.
The Shred candidates still carry unknown/default trade fields, not inferred buys.

Reproduce only these two signed captures without a network:

```sh
cargo run --offline --example pump_execution_replay -- --reported-alt
```

Each row prints `event_count`: `shred_static_pump` is 0, `shred_resolved` is 3,
and each gRPC mode is 3. The full signatures are
`2EUPDVEHJjJjE2GUMFonmeas1oGN7dpvx5Ni8W5tpxC4iekAVWE9UvmZvEMMNh8S4bEcFv8r1NyGFRsZEaWW5ySo`
and `38NiGzWamkJGr3HME3FgVQQw9dSG9sBkKgVYPb3EESBqYbbJbiQ9P7Jv4Zdci9SrbnjZGJoGecAti9CmSkUusLvi`.
This replays signed inputs and saved ALT addresses; it is not live Shred reception.

For future live transactions, the existing subscription example now preloads
real ALT accounts at startup and supplies its in-memory resolver to the client:

```sh
SOLANA_RPC_URL=https://YOUR_RPC cargo run --example shredstream_example -- \
  --endpoint http://127.0.0.1:10800 --filter trade \
  --alt-table 58tUNfyBtwkMeU8gpyZYKEo4Hyb4E25ui12w5vKYQx3P
```

Add `--preload-only` to verify startup fetching without a Shred endpoint.
Repeat `--alt-table` for additional tables. `examples/support/alt_cache.rs` shows
the concrete `preload` and `resolve` implementation for both SDK-managed and
manual parsing. It is example code, not a new SDK API. It rejects deactivating
tables rather than guessing their cooldown and rejects newer-than-transaction
snapshots. Startup snapshots do not auto-refresh; production cache updates belong
outside the receive task. Resolver failures are counted independently of events.
Use `--filter trade`; an opaque wrapper cannot establish a buy-only match.

中文：这两笔不是过滤器漏掉买单，而是 Pump.fun 的 program ID 在 ALT 中，静态账户入口
无法确认它们引用了哪个 DEX。把本地预热缓存解析出的地址传入上述入口，或在订阅客户端
配置缓存 resolver，即可收到 `third_program != None` 的回调。每笔实际有 3 次买入；
Shred 仍不推断第三方合约的内部买卖方法和金额。热路径不请求 RPC。

## Removed interfaces

`DexEvent::PumpRouteIntent`, `DexEvent::RawInstruction`, their subscription
variants and their route/raw parsers have been removed. Remove those subscriptions
and match arms. Subscribe to PumpFunTrade and inspect `third_program` and
`parse_status` instead. There is no separate raw callback or built-in wrapper-rule
engine. Official multi-hop instructions no longer emit route-intent callbacks;
their executed trade legs remain available through gRPC/RPC.

Other existing native DEX parsers and gRPC trade provenance remain unchanged.
The stock-price configuration's `RawInstructionDexType` identifiers remain for
compatibility; they do not cause raw-instruction parsing or hot-path DEX scans.

## Focused offline verification

Run from this repository:

```sh
cargo test --offline --lib wrapped_pumpfun_rpc_analyzer
```

`src/wrapped_pumpfun_rpc_analyzer.rs` is now a test-only module, not an RPC
analyzer API. It replays nine saved signed transactions, including the reported
`5hKon…`, `3nMX…` and `2r4u…` signatures. It verifies original programs,
indices, account order, bytes, metadata and subscription boundaries.

The Shred side receives signed transaction bytes and an already-loaded ALT
snapshot only; execution metadata is used by the gRPC side. This tests the parser
offline, not live UDP reception, shred reassembly or current mainnet behavior.

## Creation order, V3 and execution amounts

When a creation event is recovered from CPI because its log is missing, it is
delivered before trades of that mint. Trade and multi-hop leg order is retained.
Native `buy_v3`, `buy_exact_quote_in_v3` and `sell_v3` accept additional accounts
after the 17 IDL accounts. This does not decode opaque third-party instructions.

Both parser features classify V3 CPI events consistently, so buy/sell filters
retain their executed amounts even without logs. Amounts come from execution
events; missing execution evidence still leaves amounts unknown/default, never
replaced with an input budget. Non-SOL quote amounts remain in `quote_amount`.

```sh
cargo test --offline --test pump_execution
cargo test --offline --test pump_execution --no-default-features --features parse-zero-copy
cargo test --offline --lib shred_v3_accepts
cargo run --offline --example pump_execution_replay
```

The example prints Shred input decoding and gRPC replay with full, missing,
truncated and missing-create logs. Pass a signature and set `SOLANA_RPC_URL` to
fetch another transaction outside the parsing hot path. The V3 name/remaining-
account boundary tests are explicitly constructed cases, alongside saved
transaction replay; they are not additional mainnet captures.

## Official PumpSwap multi-hop

PumpSwap's `multi_hop_swap` can include both AMM and Pump.fun bonding-curve hops.
Shred parsing emits a `PumpFunTrade` for each curve whose canonical PDA can be
identified from the documented five-account hop layout. All-AMM routes do not
produce PumpFun callbacks merely because the fixed accounts contain the Pump program.
The callback retains `third_program`, mint, quote mint, curve accounts and
`ix_name = "multi_hop_swap"`. A continuous mint path establishes direction and
`InstructionDecoded`; unresolved direction remains `Unresolved`. Neither status
establishes execution. Per-hop executed amounts remain zero on Shred; the raw
outer data contains the route's input amount and final minimum, not per-hop fills.

gRPC preserves each on-chain TradeEvent's `sol_amount`, `quote_amount` and
`token_amount`. For a token-quoted curve, `sol_amount = 0` is valid; read
`quote_mint` and `quote_amount` instead. The official per-hop event name is
`multi_hop_swap`, even though the internal Pump CPI is `multi_hop_curve_swap`.

中文：官方多跳现在按内盘各跳回调，并保留原始路由输入。Shred 中成交量为零表示尚无执行数据；
gRPC 的非 SOL 报价交易应读取 `quote_amount`，不能把整个路由花费的 SOL 当作内盘这一跳的 SOL 金额。

`cargo run --offline --example pump_execution_replay` includes the signed report
`g1fC9wYKdndDZAyMCWkfFAqUNBCzj1brvNb3aAnRjxkrVhaSzsvELXFXz11U3DdmbM2AeFbwcaSgG86BqxApvFZ`.
It replays transaction inputs and saved execution metadata; it does not test live shred reception.
See the [official instruction documentation](https://github.com/pump-fun/pump-public-docs/blob/main/docs/instructions/MULTI_HOP_SWAP.md).
