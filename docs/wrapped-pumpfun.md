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
analyzer API. It replays seven saved signed transactions, including the reported
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
