# Comparison with solana-streamer & Migration Notes

This document compares **sol-parser-sdk** with **solana-streamer** and describes what has been migrated and what remains as a possible future addition.

## Feature comparison

| Feature | solana-streamer | sol-parser-sdk |
|--------|------------------|----------------|
| Yellowstone gRPC | ✅ | ✅ |
| **ShredStream** | ✅ | ❌ (candidate for future migration) |
| Transaction event parsing | ✅ | ✅ (same protocols: PumpFun, PumpSwap, RaydiumLaunchlab, Raydium, Meteora) |
| Account subscription | ✅ | ✅ |
| Account memcmp filters | ✅ | ✅ (`account_filter_memcmp`, see below) |
| TokenAccount / NonceAccount / PumpSwapPool account events | ✅ | ✅ |
| Event type filtering | ✅ | ✅ |
| Dynamic subscription update | ✅ | ✅ (`update_subscription`) |
| API style | Callback-based `subscribe_events_immediate(callback)` | Queue-based `subscribe_dex_events() -> Arc<ArrayQueue<DexEvent>>` |
| Order modes | N/A | ✅ Unordered / Ordered / StreamingOrdered / MicroBatch |
| RPC parsing | ✅ (parse_tx_events) | ✅ (`parse_transaction_from_rpc`, parse_*_tx examples) |

## Migrated into sol-parser-sdk (this repo)

- **Account subscription examples** (aligned with sol-parser-sdk’s queue-based API and layout):
  - `token_balance_listen` – subscribe to a single token account by pubkey.
  - `nonce_listen` – subscribe to a nonce account.
  - `token_decimals_listen` – subscribe to a mint account (TokenInfo / decimals).
  - `pumpswap_pool_account_listen` – subscribe to PumpSwap pool accounts via **memcmp** (e.g. mint at offset 32).
  - `mint_all_ata_account_listen` – subscribe to all ATAs for one or more mints via **memcmp** (mint at offset 0).
- **Memcmp helper** – `grpc::account_filter_memcmp(offset, bytes)` to build `SubscribeRequestFilterAccountsFilter` for use in `AccountFilter::filters`.

## Not in sol-parser-sdk (possible future migration)

- **ShredStream**  
  solana-streamer supports a ShredStream gRPC client (`ShredStreamGrpc`, `shredstream_subscribe`) that subscribes to entries and parses transactions from shreds.  
  Migrating this would require:
  - ShredStream proto / client (e.g. `SubscribeEntriesRequest`, `Entry` with serialized `Vec<Entry>`).
  - A new module (e.g. `shred` or `shred_stream`) that uses the existing parser (instruction/log) to produce `DexEvent`s.
  - Deciding how to expose it (queue vs callback) to match sol-parser-sdk’s architecture.

## Using account subscription (sol-parser-sdk style)

- **No memcmp** – use `AccountFilter { account: vec![pubkey], owner: vec![], filters: vec![] }`.
- **With memcmp** – use `account_filter_memcmp(offset, bytes)` and pass the result in `AccountFilter::filters`:

```rust
use sol_parser_sdk::grpc::{account_filter_memcmp, AccountFilter, EventType, EventTypeFilter, TransactionFilter, YellowstoneGrpc};

let mint = solana_sdk::pubkey::Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
let acc = AccountFilter {
    account: vec![],
    owner: vec![],
    filters: vec![account_filter_memcmp(0, mint.to_bytes().to_vec())],
};
let queue = grpc.subscribe_dex_events(
    vec![TransactionFilter::default()],
    vec![acc],
    Some(EventTypeFilter::include_only(vec![EventType::TokenAccount])),
).await?;
// consume events from queue.pop()
```

## Running the new examples

```bash
# Token account by pubkey
TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release

# Nonce account
NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release

# Mint (decimals/supply)
MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release

# PumpSwap pool(s) by memcmp
cargo run --example pumpswap_pool_account_listen --release

# All ATAs for one or two mints (optional MINT=<pubkey>)
cargo run --example mint_all_ata_account_listen --release
```

Optional env: `GRPC_ENDPOINT`, `GRPC_AUTH_TOKEN`.
