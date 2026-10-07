# Failed-transaction and skipped-test audit — 2026-10-07

## Findings and fix

The eight failed mainnet transactions failed on-chain: six attempted to allocate an already-used mint account; two had insufficient SOL. No parser fix can change their historical chain outcome. A new launch must use an unused mint account and sufficient SOL for the transfer/rent/fees.

Reviewing these previously excluded samples exposed an SDK defect: Go, Node.js and Python could emit instruction/log Create or Buy events for failed transactions, even though all of the transaction's program effects were rolled back. Python RPC metadata omitted `err`; Go and Node.js Yellowstone adapters also lost the failure flag.

The corrected high-level RPC/gRPC event paths suppress all DEX events when the transaction has a failure status. RPC metadata retains the original error; RPC-to-Yellowstone conversion preserves error **presence**, without pretending the JSON error is a Yellowstone binary error enum. Native protobuf errors remain usable as failure indicators even when their encoded payload is empty. Python's full subscription callback suppresses both instruction and log events.

The Rust RPC parser already suppressed failed transactions; the mainnet regression now explicitly asserts this behavior for the eight failed fixtures instead of skipping them. Raw ShredStream instruction decoding in sol-shred-sdk has no execution status and reports instruction intent; callers must use confirmed RPC/gRPC status to decide whether a launch committed.

## Actual failed transactions

| Signature | Slot | Confirmed chain failure |
| --- | ---: | --- |
| [2dabmQiykzC1tsNmgtRFbsRJjEKMfUySgScJc2J2uq8CQtoXWZ3YyT99p3nVuCXifEhLkyUe4Sf4nfe2UkFT8AJr](https://solscan.io/tx/2dabmQiykzC1tsNmgtRFbsRJjEKMfUySgScJc2J2uq8CQtoXWZ3YyT99p3nVuCXifEhLkyUe4Sf4nfe2UkFT8AJr) | 299999997 | Mint account already in use |
| [3GufGVVRrnrH1Yuto57jDeTKXeki6ooE9w6h4qYRvNm5FcCpxwDHNvzHXdpAMbxRmukWJpSPHUKHEjnFTCC979ka](https://solscan.io/tx/3GufGVVRrnrH1Yuto57jDeTKXeki6ooE9w6h4qYRvNm5FcCpxwDHNvzHXdpAMbxRmukWJpSPHUKHEjnFTCC979ka) | 454059364 | Insufficient SOL / lamports |
| [3LQK2JHVbVSSfyCYwTwzoYTY9NEN4gZXzqw3TvAvMJHsvPsopCHtsa5ALhj4tFn8xWbmJ5DsYMMcnPukTBkSjtQf](https://solscan.io/tx/3LQK2JHVbVSSfyCYwTwzoYTY9NEN4gZXzqw3TvAvMJHsvPsopCHtsa5ALhj4tFn8xWbmJ5DsYMMcnPukTBkSjtQf) | 299999997 | Mint account already in use |
| [3MRWKDUcD5CGn5Xz4qWyGA8Y9udw4SsSr4gcxCjTUqCFt6Zc6uiwHF1wLZXKoqBcGEM2kruBS85ymYxmYk8FLGBk](https://solscan.io/tx/3MRWKDUcD5CGn5Xz4qWyGA8Y9udw4SsSr4gcxCjTUqCFt6Zc6uiwHF1wLZXKoqBcGEM2kruBS85ymYxmYk8FLGBk) | 299999997 | Mint account already in use |
| [3NBSCgAXpNqg7JBUzp6RsfZYzf2QbbHi6r2wcoBF4Lpuzozae7ExUJUGoqpVCXXs8cbibr6aWySH1bsg86Enuhez](https://solscan.io/tx/3NBSCgAXpNqg7JBUzp6RsfZYzf2QbbHi6r2wcoBF4Lpuzozae7ExUJUGoqpVCXXs8cbibr6aWySH1bsg86Enuhez) | 299999991 | Mint account already in use |
| [3QVpcUq64D4DUV9empFr3G98pCEa1Cp2fAcFJKyiJGsQ2ir2S4yBLhHYXPNmLfCE8KVDgLApfBYTFM7pf9JVPbsb](https://solscan.io/tx/3QVpcUq64D4DUV9empFr3G98pCEa1Cp2fAcFJKyiJGsQ2ir2S4yBLhHYXPNmLfCE8KVDgLApfBYTFM7pf9JVPbsb) | 389243699 | Insufficient SOL / lamports |
| [3REXMDkvZsWvgc8czYpfn24avz18APMuzbV2VgQBeymArdJvfQcfcpKr46f3j1H2bAiymFneaNocPH7KzPUPbR1S](https://solscan.io/tx/3REXMDkvZsWvgc8czYpfn24avz18APMuzbV2VgQBeymArdJvfQcfcpKr46f3j1H2bAiymFneaNocPH7KzPUPbR1S) | 299999991 | Mint account already in use |
| [3zC4bw1osyebZ77KagXYP3b5rEBHvJCgcwCriCLc5gAUp9Ev6Bx5ZMbqnYpkKE2VgZgMFcZN5FYKQfcKL45MWpyu](https://solscan.io/tx/3zC4bw1osyebZ77KagXYP3b5rEBHvJCgcwCriCLc5gAUp9Ev6Bx5ZMbqnYpkKE2VgZgMFcZN5FYKQfcKL45MWpyu) | 299999991 | Mint account already in use |

The detailed JSON audit retains `meta.err` and the actual failure log message for each signature. The eight actual failures are now checked by Go/Node.js/Python/Rust RPC regressions; they produce no committed DEX events.

## Previously skipped or ignored tests

- The eight Node.js tests in `current_mainnet_transactions.test.ts` were opt-in network tests. Enabled them with `RUN_MAINNET_TESTS=1`: all eight passed against the public mainnet RPC, including PumpFun/PumpSwap, Meteora, Orca and Raydium fixtures.
- The Rust parser's ignored route timing test passed when run with `cargo test --lib -- --ignored`.
- sol-shred-sdk's ignored route timing and decoder microbenchmark tests both passed with the same command. These are manual timing tests, not previously failed tests.

## Final validation

- Go: `go test ./...` and `go vet ./...` passed, including the eight real failed transactions and native gRPC failure status tests.
- Node.js: build passed; full suite with `RUN_MAINNET_TESTS=1` and the shared 36-transaction corpus: **311 passed, zero skipped**.
- Python: full suite with the shared 36-transaction corpus: **277 passed**. Tests include RPC conversion, instruction-only gRPC parsing and the full subscription callback on failed transactions.
- Rust: shared-corpus RPC regression: **2 passed** (covering every successful sample plus all eight actual failed transactions); the ignored timing test passed.
- sol-shred-sdk: both ignored manual tests passed; its existing default/zero-copy correctness suites passed in the earlier synchronized-fix verification.

At verification time, changes were local and uncommitted. No transactions were submitted to Solana. Publication is tracked by the repository release.
