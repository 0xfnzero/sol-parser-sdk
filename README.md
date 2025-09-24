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
    <a href="https://github.com/0xfnzero/solana-streamer/blob/main/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License">
    </a>
</p>

<p align="center">
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana">
    <img src="https://img.shields.io/badge/SIMD-FF6B6B?style=for-the-badge&logo=intel&logoColor=white" alt="SIMD">
    <img src="https://img.shields.io/badge/gRPC-4285F4?style=for-the-badge&logo=grpc&logoColor=white" alt="gRPC">
</p>

---

## 📊 Performance Highlights

### ⚡ Ultra-Low Latency
- **10-20μs** parsing latency in release mode
- **Zero-copy** parsing with stack-allocated buffers
- **SIMD-accelerated** pattern matching (memchr)
- **Lock-free** ArrayQueue for event delivery

### 🚀 Optimization Highlights
- ✅ **Zero heap allocation** for hot paths
- ✅ **SIMD pattern matching** for all protocol detection
- ✅ **Static pre-compiled finders** for string search
- ✅ **Inline functions** with aggressive optimization
- ✅ **Event type filtering** for targeted parsing
- ✅ **Conditional Create detection** (only when needed)

---

## 🔥 Quick Start

### Installation

```shell
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

### Performance Testing

Test parsing latency with the optimized example:

```bash
# Run performance test (requires sudo for high-precision timing)
sudo cargo run --example basic --release

# Expected output:
# gRPC接收时间: 1234567890 μs
# 事件接收时间: 1234567900 μs
# 事件解析耗时: 10 μs  <-- Ultra-low latency!
```

**Why sudo?** The example uses `libc::clock_gettime(CLOCK_REALTIME)` for microsecond-precision timing, which may require elevated permissions on some systems.

### Basic Usage

```rust
use sol_parser_sdk::grpc::{YellowstoneGrpc, EventTypeFilter, EventType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create gRPC client
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;

    // Filter for PumpFun Trade events only (ultra-fast path)
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunTrade
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

---

## 🏗️ Supported Protocols

### DEX Protocols
- ✅ **PumpFun** - Meme coin trading (ultra-fast zero-copy path)
- ✅ **PumpSwap** - PumpFun swap protocol
- ✅ **Raydium AMM V4** - Automated Market Maker
- ✅ **Raydium CLMM** - Concentrated Liquidity
- ✅ **Raydium CPMM** - Concentrated Pool
- ✅ **Orca Whirlpool** - Concentrated liquidity AMM
- ✅ **Meteora AMM** - Dynamic AMM
- ✅ **Meteora DAMM** - Dynamic AMM V2
- ✅ **Meteora DLMM** - Dynamic Liquidity Market Maker
- ✅ **Bonk Launchpad** - Token launch platform

### Event Types
Each protocol supports:
- 📈 **Trade/Swap Events** - Buy/sell transactions
- 💧 **Liquidity Events** - Deposits/withdrawals
- 🏊 **Pool Events** - Pool creation/initialization
- 🎯 **Position Events** - Open/close positions (CLMM)

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
    EventType::PumpFunTrade,
    EventType::RaydiumAmmV4Swap,
    EventType::RaydiumClmmSwap,
    EventType::OrcaWhirlpoolSwap,
]);
```

### Example: Pool Monitor
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunCreate,
    EventType::RaydiumClmmCreatePool,
    EventType::OrcaWhirlpoolInitialize,
]);
```

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

### Dynamic Subscription
Update filters without reconnecting:

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### Performance Metrics
```rust
let mut config = ClientConfig::default();
config.enable_metrics = true;

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
│   └── types.rs           # Filter & config types
├── logs/
│   ├── optimized_matcher.rs  # SIMD log detection
│   ├── zero_copy_parser.rs   # Zero-copy parsing
│   ├── pumpfun.rs         # PumpFun parser
│   ├── raydium_*.rs       # Raydium parsers
│   ├── orca_*.rs          # Orca parsers
│   └── meteora_*.rs       # Meteora parsers
├── instr/
│   └── *.rs               # Instruction parsers
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

- **Repository**: https://github.com/0xfnzero/solana-streamer
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

# Run performance example
sudo cargo run --example basic --release

# Build release binary
cargo build --release

# Generate docs
cargo doc --open
```