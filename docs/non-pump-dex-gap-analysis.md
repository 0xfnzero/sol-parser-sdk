# Non-Pump DEX Gap Analysis

Date: 2026-05-30

## Scope

This audit keeps `sol-parser-sdk` as the runtime architecture and uses
`Shyft-to/solana-defi` only as a development oracle: IDL references, sample
transactions/accounts, and scenario fixtures for golden tests.

First-stage scope is Raydium, Orca, and Meteora. Fluxbeam, Moonshot, Jupiter,
Defi API examples, and bot examples remain out of core SDK scope.

## Canonical Baseline

The cross-language baseline lives at `protocols/canonical.json`.

Confirmed program-id baseline:

| Protocol | Program id | Notes |
| --- | --- | --- |
| PumpFun | `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` | Existing |
| PumpSwap | `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA` | Existing |
| PumpFees | `pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ` | Rust existed, other languages needed parity |
| RaydiumLaunchlab | `LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj` | Canonical `RaydiumLaunchlab*` event names |
| Raydium CPMM | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` | Existing |
| Raydium CLMM | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | Node/Python/Go had stale variants |
| Raydium AMM V4 | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | Existing |
| Orca Whirlpool | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | Rust existed, Node/Python partial |
| Meteora Pools | `Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB` | Rust existed, Node/Python partial |
| Meteora DAMM V2 | `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG` | Existing |
| Meteora DLMM | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | Rust existed, Node/Python partial |
| Meteora DBC | `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN` | New protocol baseline, parser pending |

## Shyft Strengths To Borrow

### Raydium

Shyft has good scenario coverage for:

- LaunchLab migration and LaunchLab buy/sell direction examples.
- CPMM new pool and token price examples.
- CLMM new pool, token price, Token-2022 and v2 instruction variants.
- Account streaming examples for AMM V4, CPMM, CLMM, and LaunchLab.

SDK gaps:

- Rust has broad Raydium log/instruction coverage, but golden tests should be
  pinned against Shyft fixtures for LaunchLab migration, CPMM new pool, and CLMM
  token price.
- LaunchLab account parsing is still missing from `parse_account_unified`.
- Node/Python/Go had program-id drift for LaunchLab and Raydium CLMM.

### Orca Whirlpool

Shyft has useful Whirlpool account-update and transaction examples, especially
for Whirlpool, Position, TickArray, FeeTier, Config and V2/adaptive-fee paths.

SDK gaps:

- Rust recognizes core Whirlpool transaction events, but account parsers for
  Whirlpool, Position, TickArray, FeeTier and Config are still missing.
- V2/Token Extension/adaptive fee instruction variants need explicit fixture
  coverage before claiming accuracy parity.
- Node/Python protocol tables did not expose Orca Whirlpool in the same way as
  Rust.

### Meteora

Shyft has the strongest useful coverage for Meteora:

- DAMM V2 accounts, new pools, token price and buy/sell examples.
- DLMM accounts and transactions.
- Pools account and transaction examples.
- DBC virtual pool, new minted tokens, migration and token price examples.

SDK gaps:

- Rust supports Pools/DAMM V2/DLMM transaction events, but account parsers are
  still mostly absent.
- Meteora DBC was absent as an SDK protocol baseline and needs parser work for
  swap, virtual-pool initialization, curve complete and migration.
- Golden fixtures should compare key fields only: program id, event type,
  signature, pool, mints, vaults, user, raw amounts, u128/sqrt price as decimal
  strings, and normalized buy/sell direction.

## Completed In First Pass

- Added `protocols/canonical.json` as the protocol/program-id/discriminator
  baseline.
- Added `Protocol::MeteoraDbc` and the Meteora DBC program id to Rust gRPC and
  instruction constants.
- Added a Rust program-aware Meteora DBC log parser for `EvtSwap`,
  `EvtInitializePool` and `EvtCurveComplete`; migration/account parsing remains
  pending.
- Added Rust account parsers for Raydium CPMM `AmmConfig` / `PoolState` and
  Orca Whirlpool `Whirlpool`, `Position`, `TickArray`, `FeeTier` and
  `WhirlpoolsConfig`; Node/Python/Go now expose the corresponding account event
  type constants.
- Planned parity updates for Node/Python/Go protocol tables to match Rust.
- Added market helper direction and price formulas as additive APIs.
- Added lightweight examples for the requested non-Pump scenarios.

## Pending Parser Work

These items need deeper parser implementation and golden fixtures:

1. Raydium LaunchLab account parser: pool/config/vesting/migration state.
2. Raydium CPMM golden fixtures: pool/config/vault state for new pool detection.
3. Orca Whirlpool golden fixtures plus remaining account variants: adaptive fee
   tier and token badge.
4. Meteora Pools/DAMM V2/DLMM account parser: pool, bin array, position and
   fee/config accounts.
5. Meteora DBC transaction parser: swap, initialize virtual pool, curve complete
   and migrations.
6. Meteora DBC account parser: VirtualPool, PoolConfig and Config.
7. Cross-language golden fixture runner: Rust is source of truth; Node, Python
   and Go should compare JSON output against the same fixture set.

## Test Strategy

- Use Shyft examples as fixture/oracle sources, not runtime dependencies.
- Store small sanitized fixtures in the SDK repos and compare only fields that
  are semantically stable across languages.
- Keep u64/u128 and BigInt fields as decimal strings in JSON.
- Gate real gRPC smoke tests behind environment variables. Default CI should
  run deterministic golden and parity tests only.
- Keep all PumpFun/PumpSwap regression tests in place to protect latency,
  deduplication and existing event naming.

## ShredStream Note

ShredStream parsing remains constrained by available transaction account keys.
For V0/ALT transactions, parser output can be incomplete until loaded addresses
are available. The recommended path is:

1. Parse what can be determined directly from the ShredStream transaction.
2. Use RPC/Yellostone transaction completion for V0/ALT loaded addresses.
3. Re-run the same Rust source-of-truth parser so JSON output stays identical.
