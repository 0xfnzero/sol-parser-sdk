# PumpFun create / create_v2 cross-language regression verification

Date: 2026-10-07. At verification time, changes were local and uncommitted. Publication is tracked by the repository release.

## Corrected behavior

- Select creation account context by the actual create/create_v2 discriminator and event mint, including CPI invocations. Require the corresponding minimum IDL account count (14/16). Decline ambiguous matches; never select a larger buy invocation as creation context.
- The current official public Pump IDL defines 16 fixed create_v2 accounts and **no quote mint/vault/token-program accounts**. Positions 16/17/18 are not interpreted as quote accounts. Preserve quote fields from decoded events and existing authoritative enrichment.
- Recognize the historical CreateEvent layout: three Borsh strings followed by exactly 96 bytes (mint, curve, user). Reject partial historical/modern layouts. Rust handles both direct logs and pre-decoded log/CPI event paths under both parser features.
- Rust and sol-shred-sdk also recognize historical create instruction arguments ending after the three strings, leaving unavailable creator metadata unspecified.
- Node.js and Python fill the complete legacy creation account context. Python declares the canonical create account fields so dataclass serialization retains them.
- sol-shred-sdk and the Rust parser SDK apply the quote fix to their ShredStream instruction paths as well.

## Mainnet evidence

Source: `sol-parser-sdk-golang/validation/pumpfun_create_20261007/transactions` and its `sources.json`, `pump_public_idl.json`, and `REPORT.md`.
Official IDL source: https://raw.githubusercontent.com/pump-fun/pump-public-docs/main/idl/pump.json

The shared corpus contains 36 signed getTransaction responses: 3 successful legacy creates, 25 successful create_v2 transactions, and 8 failed transactions. All four SDKs replayed the corpus. For each successful transaction, regression assertions compare creation mint/user/token program against the actual creating instruction and reject unrelated quote account values. Failed transactions are excluded from successful-launch assertions.

Rust RPC verification reconstructs the original signed binary transaction from the unparsed JSON response because its RPC parser expects binary encoding. sol-shred-sdk replays that same reconstructed signed transaction with the actual loaded addresses. These are offline RPC-derived replays, not independent live gRPC or raw-shred captures. This corpus does not establish USDC-launch coverage.

Each repository also includes five self-contained RPC fixtures: two historical legacy creates, a newer legacy create, and two recent create_v2 transactions. Regression tests additionally cover CPI creates, multiple mints, ambiguous duplicate creates, a longer unrelated buy, arbitrary remaining accounts, preservation of decoded quote fields, and historical payload bounds.

## Validation

| SDK | Full test results |
| --- | --- |
| sol-parser-sdk (Rust) | Default: 476 passed / 1 ignored; zero-copy: 474 passed / 1 ignored |
| sol-shred-sdk | Default: 398 passed / 2 ignored; zero-copy: 396 passed / 2 ignored |
| sol-parser-sdk-nodejs | TypeScript build passed; full suite with shared corpus: 302 passed / 8 skipped |
| sol-parser-sdk-python | Full suite with shared corpus: 275 passed |

All repositories passed `git diff --check`. Existing unrelated sol-shred-sdk working changes were preserved.

Run the new regression tests from each repository with `PUMPFUN_CREATE_CORPUS` pointing to the shared transactions directory to repeat all 36 samples; without that variable they use the repository's five saved fixtures.

- Rust: `cargo test --test pumpfun_create_regression`; also `cargo test --no-default-features --features parse-zero-copy --test pumpfun_create_regression`.
- sol-shred-sdk: the same Cargo test commands. On this macOS host, Cargo required `DYLD_LIBRARY_PATH` and `LIBCLANG_PATH` to point to the Xcode toolchain `usr/lib` directory for its existing RocksDB build dependency.
- Node.js: `npm test -- --run src/pumpfun_create_regression.test.ts`.
- Python: `.venv/bin/python -m pytest -q tests/test_pumpfun_create_regression.py`.
