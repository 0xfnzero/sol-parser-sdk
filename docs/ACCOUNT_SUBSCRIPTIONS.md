# Account subscriptions

Yellowstone gRPC delivers executed transaction metadata and account updates. `solana-streamer` provides a callback facade over this parser; use `sol-shred-sdk` for raw shred decoding. Shred data alone cannot prove execution success or recover logs and inner instructions.

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
