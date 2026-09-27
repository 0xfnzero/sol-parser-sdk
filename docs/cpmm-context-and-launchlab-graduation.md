# CPMM context and LaunchLab graduation events

This parser surface supplies **observed instruction context**, not a trading authorization or wallet-net fill. It uses checked-in `idls/raydium_cpmm.json` and `idls/raydium_launchpad.json`. No RPC enrichment or additional subscription is introduced.

## CPMM swap API

`DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent)` retains its original amount/pool fields and adds serde-defaulted fields:

- `context: Option<RaydiumCpmmSwapContext>`: payer (the swap instruction actor, not the transaction fee payer), authority, AMM config, input/output token accounts, vaults, token programs and mints, observation state. Requires complete nondefault accounts and supported classic SPL/Token-2022 program IDs. This does **not** validate account ownership, extensions, balances, fees or pool state.
- `instruction_index: Option<InstructionIndex>`: zero-based `outer` message instruction index and optional zero-based `inner` index within that outer instruction's entire inner-instruction group. The inner index is not the occurrence number of a pool/program. Use `(signature, instruction_index)` for leg identity; a signature alone cannot identify repeated swaps.
- `amounts_source: CpmmSwapAmountsSource`: `InstructionOnly` keeps both executed amount fields zero; instruction budgets/slippage limits are not fills. `SwapEvent` identifies the checked-in IDL event or its stable legacy prefix. `LegacyLog` labels older SDK decoder layouts; `Unknown` is the safe serde/default state and labels incomplete modern tails.
- `log_mints: Option<CpmmSwapMints>`: input/output mint tail from the current SwapEvent. Absent on the supported old prefix.
- `transaction_success: Option<bool>`: metadata `err` status, **not commitment**. Standalone instruction/log decoders leave it unknown.

`base_input` means **exact-input versus exact-output**, never semantic buy/sell. Determine direction from the input/output mint pair against independently validated asset/quote identity. CPMM token0/token1 ordering is not asset/quote ordering.

### Binding and merge policy

RPC parsing and both Yellowstone transaction entry points validate the complete log invocation sequence against the outer and inner compiled instructions, resolving static plus loaded writable/readonly keys. Program IDs, stack depths, invocation completions, pool, mode and available logged mints must agree. Caught CPI failures and failures of enclosing invocations cannot supply log actor context. Multiple CPMM data records in one invocation are ambiguous and not bound. Logs must be complete; missing stack heights or invocation records fail closed.

CPMM log/instruction dedup uses the validated locator, not a pool occurrence counter. Ordinary outer/inner CPMM swap instructions are never generically merged into another leg. A missing event log for the first repeated pool leg does not assign the second leg's logged amounts to the first actor.

A log with unknown/ambiguous/mismatched context remains parseable without actor context; a separate instruction-only fallback may also be emitted. Consumers must not combine those independently by pool or amount, or treat both as executed trades. The old 81-byte SwapEvent body prefix is still accepted; complete current tails supply mints. Intermediate tail lengths retain legacy numerical parsing with `Unknown` provenance. Older SDK log/CPI decoders remain available but do not gain inferred actor context or authoritative execution provenance.

**Minimum ingest gate for an attributed executed CPMM observation:** `amounts_source == SwapEvent`, `context.is_some()`, `instruction_index.is_some()`, and `transaction_success == Some(true)`. Also require the appropriate commitment from the surrounding stream. This is still not enough to book a wallet-net fill or classify StonkFun.

## LaunchLab → CPMM event API

`DexEvent::RaydiumLaunchlabMigrateCpmm(RaydiumLaunchlabMigrateCpmmEvent)` and `EventType::RaydiumLaunchlabMigrateCpmm` represent the current IDL's ordinary `migrate_to_cpswap` instruction, whether outer or invoked by another program.

Fields expose payer, semantic base/quote mints, platform config, old LaunchLab pool, global config and vaults, explicit new CPMM pool/program/config/authority/vaults/observation state, and base/quote token programs. No liquidity amount is invented. `stonkfun_mode()` checks only the exact known Standard/Reward platform config identities; unknown platforms are emitted generically and return `None`.

`instruction_index` identifies the actual migration instruction. `transaction_success` reports transaction metadata. **`invocation_success: Option<bool>` is separate:** a complete matched trace must show the migration and its ancestors completed successfully. Routers may catch failed CPIs, so transaction success alone is insufficient. With missing/truncated logs the instruction linkage is still emitted, but invocation success stays `None`. A failed or unknown invocation must not be registered as a completed graduation.

The current 28-account layout is validated against the checked-in IDL, including fixed program identities and current quote-token program constraint. Unknown/truncated layouts are rejected; they are not reinterpreted using current indices. `migrate_to_amm` remains unchanged and is not classified as CPMM. There is no fabricated migration log decoder: the checked-in LaunchLab IDL does not define a migration event.

## Consumer integration contract

1. Include LaunchLab and CPMM in the **existing** transaction subscription as needed, plus the new migration EventType. `Protocol::StonkFun` still means the LaunchLab program; it does not include CPMM or filter platform identity.
2. Require exact instruction actor matching for source/execution wallets. Transaction account inclusion or an unrelated signer is not actor proof. Aggregator CPIs can use a program-controlled actor rather than the human router user; do not reinterpret it.
3. Require confirmed successful migration invocation evidence and validate the original LaunchLab route/platform/mints plus new CPMM owner/config/mints/vaults/programs before persisting linkage. Reject conflicting mappings. A generic CPMM swap never supplies StonkFun provenance.
4. Correlate settlement to the expected `(signature, outer, inner)` and validated route/mint/program/accounts. Current CPMM IDL describes amount fields as calculation results without transfer fee; they are not a general wallet-net balance proof. Obtain validated attributable token-account deltas/transfer evidence as necessary. Instruction-only zeros and instruction limits are never settlement amounts.
5. Keep discovery/backfill/restart policy outside this parser. Wallet-only subscriptions need not observe third-party migrations; new fields do not guarantee migration discovery coverage.

## Compatibility and evidence bounds

- Original fields and legacy log decoders remain; additive fields use serde defaults and are excluded from existing Borsh layout derivation. Old JSON can deserialize. Rust consumers constructing exhaustive struct literals must supply the new fields or use `..Default::default()`; this is source additive, not a promise of struct-literal compatibility.
- Saved generic CPMM multileg RPC data is replayed through RPC and both Yellowstone entry points; synthetic tests exercise repeated same-pool actors, both orientations and modes, loaded addresses, missing/mismatched/failed traces, duplicate data records, filters and migration layouts.
- Saved PumpFun/PumpSwap fixtures and existing legacy layout tests are retained. No Pump-specific parsing or arithmetic was changed.
- **No checked-in real StonkFun migration transaction is available.** Synthetic current-IDL tests do not establish historical/mainnet migration layout compatibility, Reward net settlement, account-extension support, or live copy-trading readiness.
- Full trace validation is deliberately conservative: streams missing invocation logs/heights may emit only incomplete observations plus instruction fallbacks. There is no guessed actor fallback. Functional tests are not a latency benchmark; no zero-overhead claim is made.
