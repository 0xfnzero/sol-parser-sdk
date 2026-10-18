# Native Rust offline Pump simulations

Run from this repository's root, using Cargo only:

```sh
cargo run --offline --example pump_offline_bank
cargo test --offline --test pump_offline_bank --test pump_shred_routes
```

The example builds, signs, simulates, commits locally and parses transactions in Rust. The integration test calls the same example module, so tests exercise the executable example rather than a separate implementation. No Python interpreter, subprocess, RPC endpoint, external wallet or broadcasts are involved. LiteSVM and the snapshot decoder are development dependencies; the library's runtime dependencies and hot path are unchanged.

Fetch Cargo dependencies once before using `--offline` on a new checkout. On macOS, native dependencies may require `DYLD_LIBRARY_PATH` and `LIBCLANG_PATH` pointing to your Xcode toolchain's `usr/lib` directory. Keep separate `CARGO_TARGET_DIR` directories for unrelated SDK/interop builds.

Configuration defaults to `config.json` in this directory, independent of the working directory. The example accepts an optional JSON config path after `--`:

```sh
cargo run --offline --example pump_offline_bank -- examples/offline_bank/config.json
```

Snapshot paths are relative to this fixture directory. Increase `repetitions` for more runs (minimum 2). Snapshot SHA-256 mismatches, execution errors, parser mismatches and nondeterministic results all fail the command.

## Executed matrix

Every repetition executes 20 freshly signed V0 transactions: curve→curve buy and sell, pool→curve, curve→pool and pool→pool, each with static keys/real runtime ALT loading and success/final-slippage failure. It verifies:

- Signature and blockhash checks are enabled in LiteSVM; every signed wire passes SDK verification.
- Simulation does not mutate bank accounts; local commit matches the simulated success state.
- Inputs are debited and outputs credited, including native SOL payout.
- Parent slippage rejection (6004) rolls back every tracked non-payer account after inner execution; only the actual signature fee is charged.
- Damaged signatures and newly signed invalid writable/readonly ALT indices are rejected before instructions run without changing state.
- SDK ShredStream parsing emits one route intent with correct accounts and arguments, on both successful and failed transaction inputs.
- Both per-hop log events match `expected.json`: venues, amounts, fees and instruction names. Failed execution logs describe attempted events, not settled fills. The parser SDK also checks its execution-aware RPC-format decoder suppresses fills on parent failure; this is an in-memory conversion, not an RPC call.
- Repetitions produce identical signed wires, execution logs, inner instructions, account balances and compute units.

The separate `pump_shred_routes` test covers six original signed mainnet wires, third-party wrappers (including V1), strict cached ALT resolution, malformed layouts, filters and allocation rejection. **Third-party wrapper programs are not executed by this bank bundle.** Arbitrary CPI cannot be reconstructed from raw shred transaction bytes.

## Frozen snapshot provenance

`snapshot.json.gz` contains unmodified deployed SBF programs, captured accounts and transaction inputs from the 2026-10-09 Pump validation fork. Users were locally funded, quote depth locally enabled, and synthetic token-paired coins/pools prepared through executed create/migrate instructions. Only pre-route state is stored; every run executes the transactions again. User, ATA and user-volume PDA identities are remapped to a public deterministic test key inside the example.

The compressed snapshot SHA-256 is pinned in `config.json`; `programs.json` records each program-byte SHA-256. `expected.json` was independently decoded using official Pump/Pump AMM IDLs for the same frozen pre-route state. The public test key must never receive funds.

These tests validate the pinned deployment's correctness. They do not claim current mainnet enablement, support for arbitrary wrapper versions or production latency. Refreshing snapshots requires a deliberate fixture and oracle update; execution never fetches live accounts or programs.
