<div align="center">
    <h1>⚡ Sol Parser SDK</h1>
    <h3><em>Ultra-low latency Solana DEX event parser with SIMD optimization</em></h3>
</div>

<p align="center">
    <strong>High-performance Rust library for parsing Solana DEX events with microsecond-level latency</strong>
</p>

<p align="center">
    <a href="https://crates.io/crates/sol-parser-sdk">
        <img src="https://img.shields.io/crates/v/sol-parser-sdk.svg" alt="Crates.io">
    </a>
    <a href="https://docs.rs/sol-parser-sdk">
        <img src="https://docs.rs/sol-parser-sdk/badge.svg" alt="Documentation">
    </a>
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License">
    </a>
</p>

<p align="center">
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana">
    <img src="https://img.shields.io/badge/SIMD-FF6B6B?style=for-the-badge&logo=intel&logoColor=white" alt="SIMD">
    <img src="https://img.shields.io/badge/gRPC-4285F4?style=for-the-badge&logo=grpc&logoColor=white" alt="gRPC">
</p>

<p align="center">
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README_CN.md">中文</a> |
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README.md">English</a> |
    <a href="https://fnzero.dev/">Website</a> |
    <a href="https://t.me/fnzero_group">Telegram</a> |
    <a href="https://discord.gg/vuazbGkqQE">Discord</a>
</p>

---

## 📦 SDK Versions

This SDK is available in multiple languages:

| Language | Repository | Description |
|----------|------------|-------------|
| **Rust** | [sol-parser-sdk](https://github.com/0xfnzero/sol-parser-sdk) | Ultra-low latency with SIMD optimization |
| **Node.js** | [sol-parser-sdk-nodejs](https://github.com/0xfnzero/sol-parser-sdk-nodejs) | TypeScript/JavaScript for Node.js |
| **Python** | [sol-parser-sdk-python](https://github.com/0xfnzero/sol-parser-sdk-python) | Async/await native support |
| **Go** | [sol-parser-sdk-golang](https://github.com/0xfnzero/sol-parser-sdk-golang) | Concurrent-safe with goroutine support |

## What This SDK Is For

`sol-parser-sdk` is the low-level Rust parser core for Solana DEX events. It is designed for trading bots, copy-trading pipelines, sniper bots, indexers, and stream processors that need fast, typed parsing from Yellowstone gRPC transactions, Jito ShredStream entries, RPC transaction payloads, or account subscriptions.

| Area | Coverage |
|------|----------|
| Parser inputs | Yellowstone gRPC, ShredStream, RPC transactions, encoded transactions, protocol account data |
| DEX protocols | PumpFun, PumpSwap, Pump Fees, Raydium LaunchLab, Raydium CPMM, Raydium CLMM, Raydium AMM V4, Meteora DAMM v2, Meteora DLMM, Meteora DBC, Orca Whirlpool |
| Parser backends | Default Borsh parser for maintainability, optional zero-copy parser for latency-sensitive hot paths |
| Related SDK | Use [solana-streamer](https://github.com/0xfnzero/solana-streamer) when you want a higher-level streaming facade over this parser core |

---

## 📊 Performance Highlights

### ⚡ Ultra-Low Latency
- **10-20μs** parsing latency in release mode
- **Zero-copy** parsing with stack-allocated buffers
- **SIMD-accelerated** pattern matching (memchr)
- **Lock-free** ArrayQueue for event delivery

### 🎚️ Flexible Order Modes
| Mode | Latency | Description |
|------|---------|-------------|
| **Unordered** | 10-20μs | Immediate output, ultra-low latency |
| **MicroBatch** | 50-200μs | Micro-batch ordering with time window |
| **StreamingOrdered** | 0.1-5ms | Stream ordering with continuous sequence release |
| **Ordered** | 1-50ms | Full slot ordering, wait for complete slot |

### 🚀 Optimization Highlights
- ✅ **Zero heap allocation** for hot paths
- ✅ **SIMD pattern matching** for all protocol detection
- ✅ **Static pre-compiled finders** for string search
- ✅ **Inline functions** with aggressive optimization
- ✅ **Event type filtering** for targeted parsing
- ✅ **Conditional Create detection** (only when needed)
- ✅ **Multiple order modes** for latency vs ordering trade-off

---

## 🔥 Quick Start

### Installation

Clone the repository:

```bash
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

Add to your `Cargo.toml`:

```toml
[dependencies]
# Default: Borsh parser
sol-parser-sdk = { path = "../sol-parser-sdk" }

# Or: Zero-copy parser (maximum performance)
sol-parser-sdk = { path = "../sol-parser-sdk", default-features = false, features = ["parse-zero-copy"] }
```

### Use crates.io

```toml
# Add to your Cargo.toml
sol-parser-sdk = "0.6.6"
```

Or with the zero-copy parser (maximum performance):

```toml
sol-parser-sdk = { version = "0.6.6", default-features = false, features = ["parse-zero-copy"] }
```

### Release Notes

#### v0.6.6

- Adds `pre_token_balance` and `post_token_balance` to PumpFun trade events in raw mint units.
- Adds `pre_sol_balance` and `post_sol_balance` in lamports, filled directly from transaction metadata without extra RPC calls.
- Reduces end-to-end PumpFun Yellowstone parsing latency by 34.23% on the captured benchmark fixture, from 6.7349 us to 4.4293 us.
- Adds reproducible Criterion benchmarks, a captured Yellowstone transaction fixture, and live balance-delta validation.

#### v0.6.4

- Adds `token_x_mint` and `token_y_mint` context to current Meteora DLMM swap events.
- Anchors mint enrichment to each event pool so multi-leg DLMM routes cannot reuse another pool's accounts.
- Adds reusable mainnet RPC examples captured on 2026-08-14 for direct `swap` and CPI `swap2` parsing.
- Keeps JSON produced before the new mint fields compatible with Serde deserialization.

#### v0.6.3

- Exposes current Raydium LaunchLab quote mint and global configuration context, including USD1 pools.
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

### Performance Testing

Transaction-cost parsing is opt-in and does not run on the default DEX event path:

```rust
use sol_parser_sdk::{parse_yellowstone_transaction_cost, SwqosProvider};

let cost = parse_yellowstone_transaction_cost(&transaction, &meta).unwrap();
let jito_tip = cost.tip_lamports_for(SwqosProvider::Jito);
```

Yellowstone and RPC results use status metadata for the authoritative transaction fee and
confirmed tip status. `transaction_fee_lamports` already includes the priority fee; relay tips
are separate transfers and are added only in `total_fee_and_tip_lamports`. Tip recipients for every
SWQoS provider supported by `sol-trade-sdk` are recognized automatically, and each `TipPayment`
identifies its provider. ShredStream can expose requested compute-budget values and outer tip
transfers, but cannot confirm fees, execution, inner instructions, or ALT-loaded tip recipients.

Test parsing latency with the optimized examples:

```bash
# PumpFun with detailed metrics (per-event + 10s stats)
cargo run --example pumpfun_with_metrics --release

# PumpSwap with detailed metrics (per-event + 10s stats)
cargo run --example pumpswap_with_metrics --release

# PumpSwap ultra-low latency test
cargo run --example pumpswap_low_latency --release

# PumpSwap with MicroBatch ordering
cargo run --example pumpswap_ordered --release

# Expected output:
# gRPC receive time: 1234567890 μs
# Event receive time: 1234567900 μs
# Latency: 10 μs  <-- Ultra-low latency!
```

### Examples

| Description | Run Command | Source Code |
|-------------|-------------|-------------|
| **PumpFun** | | |
| PumpFun event parsing with metrics | `cargo run --example pumpfun_with_metrics --release` | [examples/pumpfun_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_with_metrics.rs) |
| PumpFun trade type filtering | `cargo run --example pumpfun_trade_filter --release` | [examples/pumpfun_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter.rs) |
| PumpFun trade with ordered mode | `cargo run --example pumpfun_trade_filter_ordered --release` | [examples/pumpfun_trade_filter_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter_ordered.rs) |
| Quick PumpFun connection test | `cargo run --example pumpfun_quick_test --release` | [examples/pumpfun_quick_test.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_quick_test.rs) |
| Parse PumpFun tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` | [examples/parse_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pump_tx.rs) |
| Parse PumpFun quote-mint cases | `TX_SIGNATURES=<sig1,sig2> cargo run --example parse_pumpfun_quote_cases --release` | [examples/parse_pumpfun_quote_cases.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpfun_quote_cases.rs) |
| Debug PumpFun transaction | `cargo run --example debug_pump_tx --release` | [examples/debug_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pump_tx.rs) |
| **PumpSwap** | | |
| PumpSwap events with metrics | `cargo run --example pumpswap_with_metrics --release` | [examples/pumpswap_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_with_metrics.rs) |
| PumpSwap ultra-low latency | `cargo run --example pumpswap_low_latency --release` | [examples/pumpswap_low_latency.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_low_latency.rs) |
| PumpSwap with MicroBatch ordering | `cargo run --example pumpswap_ordered --release` | [examples/pumpswap_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_ordered.rs) |
| Parse PumpSwap tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` | [examples/parse_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpswap_tx.rs) |
| Debug PumpSwap transaction | `cargo run --example debug_pumpswap_tx --release` | [examples/debug_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pumpswap_tx.rs) |
| **Meteora DAMM** | | |
| Meteora DAMM V2 events | `cargo run --example meteora_damm_grpc --release` | [examples/meteora_damm_grpc.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_grpc.rs) |
| Parse Meteora DAMM tx by signature | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` | [examples/parse_meteora_damm_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_meteora_damm_tx.rs) |
| **Non-Pump DEX dry-run scenarios** | | |
| Raydium LaunchLab migration filter | `cargo run --example raydium_launchlab_migration` | [examples/raydium_launchlab_migration.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_launchlab_migration.rs) |
| Raydium CPMM new pool filter | `cargo run --example raydium_cpmm_new_pool` | [examples/raydium_cpmm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_cpmm_new_pool.rs) |
| Raydium CLMM token price math | `cargo run --example raydium_clmm_token_price` | [examples/raydium_clmm_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_clmm_token_price.rs) |
| Orca Whirlpool token price math | `cargo run --example orca_whirlpool_token_price` | [examples/orca_whirlpool_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/orca_whirlpool_token_price.rs) |
| Meteora DAMM new pool baseline | `cargo run --example meteora_damm_new_pool` | [examples/meteora_damm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_new_pool.rs) |
| Meteora DBC token price math | `cargo run --example meteora_dbc_token_price` | [examples/meteora_dbc_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_dbc_token_price.rs) |
| Wallet trade filter | `cargo run --example wallet_trade_filter` | [examples/wallet_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/wallet_trade_filter.rs) |
| gRPC latency slot compare config | `cargo run --example grpc_latency_slot_compare` | [examples/grpc_latency_slot_compare.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/grpc_latency_slot_compare.rs) |
| **Account subscription** | | |
| Token account balance updates | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` | [examples/token_balance_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_balance_listen.rs) |
| Nonce account state changes | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` | [examples/nonce_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/nonce_listen.rs) |
| Mint account info | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` | [examples/token_decimals_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_decimals_listen.rs) |
| PumpSwap pool accounts via memcmp | `cargo run --example pumpswap_pool_account_listen --release` | [examples/pumpswap_pool_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_pool_account_listen.rs) |
| All ATAs for mints | `cargo run --example mint_all_ata_account_listen --release` | [examples/mint_all_ata_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/mint_all_ata_account_listen.rs) |
| **ShredStream** | | |
| Jito ShredStream subscription | `cargo run --example shredstream_example --release` | [examples/shredstream_example.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/shredstream_example.rs) |
| **Utility** | | |
| Dynamic subscription filters | `cargo run --example dynamic_subscription --release` | [examples/dynamic_subscription.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/dynamic_subscription.rs) |
| Debug PumpSwap account filling | `cargo run --example test_account_filling --release` | [examples/test_account_filling.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/test_account_filling.rs) |

### Basic Usage

```rust
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol,
    TransactionFilter, YellowstoneGrpc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create gRPC client with default config (Unordered mode)
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;
    
    // Or with custom config for ordered events
    let config = ClientConfig {
        order_mode: OrderMode::MicroBatch,  // Low latency + ordering
        micro_batch_us: 100,                // 100μs batch window
        ..ClientConfig::default()
    };
    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    let protocols = vec![Protocol::PumpFun, Protocol::PumpSwap, Protocol::RaydiumCpmm];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    // Filter before parsing for the lowest-latency path.
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpSwapBuy,
        EventType::PumpSwapSell,
        EventType::RaydiumCpmmSwap,
    ]);

    // Subscribe and get lock-free queue
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    ).await?;

    // Consume events with minimal latency
    tokio::spawn(async move {
        let mut spin_count = 0;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                // Process event (10-20μs latency!)
                println!("{:?}", event);
            } else {
                // Hybrid spin-wait strategy
                spin_count += 1;
                if spin_count < 1000 {
                    std::hint::spin_loop();
                } else {
                    tokio::task::yield_now().await;
                    spin_count = 0;
                }
            }
        }
    });

    Ok(())
}
```

### ShredStream Usage (Jito)

ShredStream provides ultra-low latency (~50-100ms faster than gRPC) by directly subscribing to Jito's ShredStream service:

```rust
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::shredstream::{ShredStreamClient, ShredStreamConfig};
use sol_parser_sdk::DexEvent;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create ShredStream client
    let client = ShredStreamClient::new("http://127.0.0.1:10800").await?;

    // Or with custom config
    let config = ShredStreamConfig {
        connection_timeout_ms: 5000,
        request_timeout_ms: 30000,
        max_decoding_message_size: 1024 * 1024 * 1024,
        reconnect_delay_ms: 1000,
        max_reconnect_attempts: 0, // 0 = infinite reconnect
    };
    let client = ShredStreamClient::new_with_config("http://127.0.0.1:10800", config).await?;

    // Subscribe with SDK-side filtering before event conversion.
    // Use `client.subscribe().await?` to receive every supported event.
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpSwapBuy,
        EventType::RaydiumCpmmSwap,
    ]);
    let queue = client.subscribe_with_filter(Some(event_filter)).await?;

    // Consume events
    loop {
        if let Some(event) = queue.pop() {
            match &event {
                DexEvent::PumpFunTrade(e) => {
                    println!("PumpFun Trade: mint={}, is_buy={}", e.mint, e.is_buy);
                }
                DexEvent::PumpSwapBuy(e) => {
                    println!("PumpSwap Buy: pool={}", e.pool);
                }
                _ => {}
            }
        } else {
            std::hint::spin_loop();
        }
    }
}
```

**ShredStream Limitations:**
- Only `static_account_keys()` - ALT-loaded instruction accounts use default placeholders, while outer instructions are parsed best-effort from data/discriminators
- No Inner Instructions - CPI/inner-only events cannot be recovered from ShredStream entries
- No block_time - always 0
- tx_index is entry-level, not slot-level

---

## 🏗️ Supported Protocols

### DEX Protocols
- ✅ **PumpFun** - Meme coin trading (ultra-fast zero-copy path, incl. v2 instructions)
- ✅ **Pump Fees** - Pump fee-sharing configuration events
- ✅ **PumpSwap** - PumpFun swap protocol
- ✅ **Raydium LaunchLab** - Token launch platform
- ✅ **Raydium AMM V4** - Automated Market Maker
- ✅ **Raydium CLMM** - Concentrated Liquidity
- ✅ **Raydium CPMM** - Concentrated Pool
- ✅ **Orca Whirlpool** - Concentrated liquidity AMM
- ✅ **Meteora Pools** - Dynamic AMM
- ✅ **Meteora DAMM v2** - Dynamic AMM V2
- ✅ **Meteora DLMM** - Dynamic Liquidity Market Maker
- 🚧 **Meteora DBC** - Program id and filters added; transaction/account parser pass pending

### Event Types
Each protocol supports:
- 📈 **Trade/Swap Events** - Buy/sell transactions
- 💧 **Liquidity Events** - Deposits/withdrawals
- 🏊 **Pool Events** - Pool creation/initialization
- 🎯 **Position Events** - Open/close positions (CLMM)

### Non-Pump DEX Support Matrix

| Protocol | Events | Accounts | Examples | Language constants |
|----------|--------|----------|----------|--------------------|
| Raydium LaunchLab | Trade, pool create, migrate | Pending | Migration, buy/sell oracle planned | Rust, Node, Python, Go |
| Raydium CPMM | Swap, deposit, withdraw, initialize | AmmConfig, PoolState | New pool, token price | Rust, Node, Python, Go |
| Raydium CLMM | Swap, pool, position, liquidity | AmmConfig, PoolState, TickArray | Token price | Rust, Node, Python, Go |
| Raydium AMM V4 | Swap, deposit, withdraw, initialize2 | Pending | Token price oracle planned | Rust, Node, Python, Go |
| Orca Whirlpool | Swap, liquidity, pool init | Whirlpool, Position, TickArray, FeeTier, Config | Token price | Rust, Node, Python, Go |
| Meteora Pools | Swap, liquidity, pool create, fees | Pending | Token price oracle planned | Rust, Node, Python, Go |
| Meteora DAMM V2 | Swap, liquidity, position | Pending | New pool, token price oracle planned | Rust, Node, Python, Go |
| Meteora DLMM | Swap, liquidity, bin/position | Pending | Token price oracle planned | Rust, Node, Python, Go |
| Meteora DBC | Swap, initialize pool, curve complete (Rust log parser) | Pending | Token price, migration oracle planned | Rust, Node, Python, Go |

The canonical cross-language baseline is tracked in
[`protocols/canonical.json`](protocols/canonical.json). The current audit and
remaining parser work are documented in
[`docs/non-pump-dex-gap-analysis.md`](docs/non-pump-dex-gap-analysis.md).

---

## ⚡ Performance Features

### Zero-Copy Parsing
```rust
// Stack-allocated 512-byte buffer for PumpFun Trade
const MAX_DECODE_SIZE: usize = 512;
let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

// Decode directly to stack, no heap allocation
general_purpose::STANDARD
    .decode_slice(data_part.as_bytes(), &mut decode_buf)
    .ok()?;
```

### SIMD Pattern Matching
```rust
// Pre-compiled SIMD finders (initialized once)
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));

// 3-10x faster than .contains()
if PUMPFUN_FINDER.find(log_bytes).is_some() {
    return LogType::PumpFun;
}
```

### Event Type Filtering
```rust
// Ultra-fast path for single event type
if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
    if log_type == LogType::PumpFun {
        return parse_pumpfun_trade(  // Zero-copy path
            log, signature, slot, block_time, grpc_recv_us, is_created_buy
        );
    }
}
```

### Lock-Free Queue
```rust
// ArrayQueue with 100,000 capacity
let queue = Arc::new(ArrayQueue::<DexEvent>::new(100_000));

// Non-blocking push/pop (no mutex overhead)
let _ = queue.push(event);
if let Some(event) = queue.pop() {
    // Process event
}
```

---

## 🎯 Event Filtering

Reduce processing overhead by filtering specific events:

### Example: Trading Bot
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunBuy,
    EventType::PumpFunSell,
    EventType::PumpFunBuyExactSolIn,
    EventType::PumpSwapBuy,
    EventType::PumpSwapSell,
    EventType::RaydiumLaunchlabTrade,
    EventType::RaydiumCpmmSwap,
    EventType::RaydiumAmmV4Swap,
    EventType::RaydiumClmmSwap,
    EventType::OrcaWhirlpoolSwap,
    EventType::MeteoraPoolsSwap,
    EventType::MeteoraDammV2Swap,
    EventType::MeteoraDlmmSwap,
]);
```

### Example: Pool Monitor
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunCreate,
    EventType::PumpFeesUpdateFeeShares,
    EventType::PumpSwapCreatePool,
    EventType::AccountPumpSwapPool,
    EventType::RaydiumCpmmInitialize,
    EventType::RaydiumClmmCreatePool,
    EventType::OrcaWhirlpoolPoolInitialized,
    EventType::MeteoraPoolsPoolCreated,
    EventType::MeteoraDammV2CreatePosition,
    EventType::MeteoraDlmmInitializePool,
]);
```

`PumpSwapCreatePool` includes `is_mayhem_mode`. For `is_cashback_coin`,
ShredStream/outer-instruction parsing reads the flag from the `create_pool`
instruction args, while log-only `CreatePoolEvent` payloads keep the default
`false` because the log event IDL does not carry this field. The authoritative
account value is also available from
`PumpSwapPoolAccountEvent.pool.is_cashback_coin`.

**Performance Impact:**
- 60-80% reduction in processing
- Lower memory usage
- Reduced network bandwidth

---

## 🔧 Advanced Features

### Create+Buy Detection
Automatically detects when a token is created and immediately bought in the same transaction:

```rust
// Detects "Program data: GB7IKAUcB3c..." pattern
let has_create = detect_pumpfun_create(logs);

// Sets is_created_buy flag on Trade events
if has_create {
    trade_event.is_created_buy = true;
}
```

### Pump.fun Bonding Curve v2 (buy_v2 / sell_v2 / buy_exact_quote_in_v2)

The SDK recognizes Pump.fun's new v2 trading instructions introduced in the Bonding Curve upgrade. Event logs from `buy_v2`, `sell_v2`, and `buy_exact_quote_in_v2` are parsed with the same zero-copy path and mapped to the existing event types:

| ix_name in TradeEvent | DexEvent Variant |
|----------------------|-----------------|
| `"buy"` / `"buy_v2"` / `"buy_exact_quote_in"` / `"buy_exact_quote_in_v2"` | `DexEvent::PumpFunBuy` |
| `"sell"` / `"sell_v2"` | `DexEvent::PumpFunSell` |
| `"buy_exact_sol_in"` | `DexEvent::PumpFunBuyExactSolIn` |

No changes are required in your event handling code — v2 events arrive through the same `PumpFunTradeEvent` struct with the correct `ix_name` field populated. Instruction discriminators for `buy_v2` (`[184, 23, 238, 97, 103, 197, 211, 61]`), `sell_v2` (`[93, 246, 130, 60, 231, 233, 64, 178]`), and `buy_exact_quote_in_v2` (`[194, 171, 28, 70, 104, 77, 91, 47]`) are recognized at the instruction parser level.

`CreateEvent` also exposes `quote_mint` and `virtual_quote_reserves`, so USDC quote pools can be distinguished from native SOL pools and initialized with the correct quote-side reserve.

### Dynamic Subscription
Update filters without reconnecting:

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### Order Modes
Choose the right balance between latency and ordering:

```rust
use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

// Ultra-low latency (no ordering guarantee)
let config = ClientConfig {
    order_mode: OrderMode::Unordered,
    ..ClientConfig::default()
};

// Low latency with micro-batch ordering (50-200μs)
let config = ClientConfig {
    order_mode: OrderMode::MicroBatch,
    micro_batch_us: 100,  // 100μs batch window
    ..ClientConfig::default()
};

// Stream ordering with continuous sequence release (0.1-5ms)
let config = ClientConfig {
    order_mode: OrderMode::StreamingOrdered,
    order_timeout_ms: 50,  // Timeout for incomplete sequences
    ..ClientConfig::default()
};

// Full slot ordering (1-50ms, wait for complete slot)
let config = ClientConfig {
    order_mode: OrderMode::Ordered,
    order_timeout_ms: 100,
    ..ClientConfig::default()
};
```

### Performance Metrics
```rust
let config = ClientConfig {
    enable_metrics: true,
    ..ClientConfig::default()
};

let grpc = YellowstoneGrpc::new_with_config(endpoint, token, config)?;
```

---

## 📁 Project Structure

```
src/
├── core/
│   └── events.rs          # Event definitions
├── grpc/
│   ├── client.rs          # Yellowstone gRPC client
│   ├── buffers.rs         # SlotBuffer & MicroBatchBuffer
│   └── types.rs           # OrderMode, ClientConfig, filters
├── shredstream/
│   ├── client.rs          # Jito ShredStream client
│   ├── config.rs          # ShredStreamConfig
│   └── proto/             # Protobuf definitions
├── logs/
│   ├── optimized_matcher.rs  # SIMD log detection
│   ├── zero_copy_parser.rs   # Zero-copy parsing
│   ├── pumpfun.rs         # PumpFun parser
│   ├── raydium_*.rs       # Raydium parsers
│   ├── orca_*.rs          # Orca parsers
│   └── meteora_*.rs       # Meteora parsers
├── instr/
│   └── *.rs               # Instruction parsers
├── warmup/
│   └── mod.rs             # Parser warmup (auto-called)
└── lib.rs
```

---

## 🚀 Optimization Techniques

### 1. **SIMD String Matching**
- Replaced all `.contains()` with `memmem::Finder`
- 3-10x performance improvement
- Pre-compiled static finders

### 2. **Zero-Copy Parsing**
- Stack-allocated buffers (512 bytes)
- No heap allocation in hot path
- Inline helper functions

### 3. **Event Type Filtering**
- Early filtering at protocol level
- Conditional Create detection
- Single-type ultra-fast path

### 4. **Lock-Free Queue**
- ArrayQueue (100K capacity)
- Spin-wait hybrid strategy
- No mutex overhead

### 5. **Aggressive Inlining**
```rust
#[inline(always)]
fn read_u64_le_inline(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[offset..offset + 8]);
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}
```

---

## 📊 Benchmarks

### Parsing Latency (Release Mode)
| Protocol | Avg Latency | Min | Max |
|----------|-------------|-----|-----|
| PumpFun Trade (zero-copy) | 10-15μs | 8μs | 20μs |
| Raydium AMM V4 Swap | 15-20μs | 12μs | 25μs |
| Orca Whirlpool Swap | 15-20μs | 12μs | 25μs |

### SIMD Pattern Matching
| Operation | Before (contains) | After (SIMD) | Speedup |
|-----------|------------------|--------------|---------|
| Protocol detection | 50-100ns | 10-20ns | 3-10x |
| Create event detection | 150ns | 30ns | 5x |

---

## 📄 License

MIT License

## 📞 Contact

- **Repository**: https://github.com/0xfnzero/sol-parser-sdk
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE

---

## ⚠️ Performance Tips

1. **Use Event Filtering** - Filter at the source for 60-80% performance gain
2. **Run in Release Mode** - `cargo build --release` for full optimization
3. **Test with sudo** - `sudo cargo run --example basic --release` for accurate timing
4. **Monitor Latency** - Check `grpc_recv_us` and queue latency in production
5. **Tune Queue Size** - Adjust ArrayQueue capacity based on your throughput
6. **Spin-Wait Strategy** - Tune spin count (default: 1000) for your use case

## 🔬 Development

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Generate docs
cargo doc --open
```
