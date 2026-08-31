<div align="center">
    <h1>⚡ Sol Parser SDK</h1>
    <h3><em>超低延迟的 Solana DEX 事件解析器（SIMD 优化）</em></h3>
</div>

<p align="center">
    <strong>高性能 Rust 库，提供微秒级延迟的 Solana DEX 事件解析</strong>
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

## 📦 SDK 版本

本 SDK 提供多种语言版本：

| 语言 | 仓库 | 描述 |
|------|------|------|
| **Rust** | [sol-parser-sdk](https://github.com/0xfnzero/sol-parser-sdk) | 超低延迟，SIMD 优化 |
| **Node.js** | [sol-parser-sdk-nodejs](https://github.com/0xfnzero/sol-parser-sdk-nodejs) | TypeScript/JavaScript，Node.js 支持 |
| **Python** | [sol-parser-sdk-python](https://github.com/0xfnzero/sol-parser-sdk-python) | 原生 async/await 支持 |
| **Go** | [sol-parser-sdk-golang](https://github.com/0xfnzero/sol-parser-sdk-golang) | 并发安全，goroutine 支持 |

## 这个 SDK 适合什么场景

`sol-parser-sdk` 是 Solana DEX 事件的底层 Rust 解析核心，适合交易机器人、跟单管道、狙击机器人、索引服务和流处理系统，从 Yellowstone gRPC 交易、Jito ShredStream entry、RPC 交易 payload 或账户订阅中快速解析出强类型事件。

| 方向 | 覆盖范围 |
|------|----------|
| 解析输入 | Yellowstone gRPC、ShredStream、RPC 交易、编码交易、协议账户数据 |
| DEX 协议 | PumpFun、PumpSwap、Pump Fees、Raydium LaunchLab、Raydium CPMM、Raydium CLMM、Raydium AMM V4、Meteora DAMM v2、Meteora DLMM、Meteora DBC、Orca Whirlpool |
| 解析后端 | 默认 Borsh 解析器便于维护，也可为低延迟热路径启用 zero-copy 解析器 |
| 相关 SDK | 如果需要更高层的事件流封装，请使用 [solana-streamer](https://github.com/0xfnzero/solana-streamer) |

---

## 📊 性能亮点

### ⚡ 超低延迟
- **10-20μs** 解析延迟（Release 模式）
- **零拷贝** 栈缓冲区解析
- **SIMD 加速** 模式匹配（memchr）
- **无锁队列** ArrayQueue 事件传递

### 🎚️ 灵活的顺序模式
| 模式 | 延迟 | 说明 |
|------|---------|-------------|
| **Unordered** | 10-20μs | 立即输出，超低延迟 |
| **MicroBatch** | 50-200μs | 微批次排序，时间窗口内排序 |
| **StreamingOrdered** | 0.1-5ms | 流式排序，连续序列立即释放 |
| **Ordered** | 1-50ms | 完整 slot 排序，等待整个 slot 完成 |

### 🚀 优化特性
- ✅ **零堆分配** 热路径无堆分配
- ✅ **SIMD 模式匹配** 所有协议检测 SIMD 加速
- ✅ **静态预编译查找器** 字符串搜索零开销
- ✅ **激进内联** 关键函数强制内联
- ✅ **事件类型过滤** 精准解析目标事件
- ✅ **条件 Create 检测** 仅在需要时检测
- ✅ **多种顺序模式** 延迟与顺序的灵活平衡

---

## 🔥 快速开始

### 安装

克隆仓库：

```bash
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

在 `Cargo.toml` 中添加：

```toml
[dependencies]
# 默认：Borsh 解析器
sol-parser-sdk = { path = "../sol-parser-sdk" }

# 或：零拷贝解析器（最高性能）
sol-parser-sdk = { path = "../sol-parser-sdk", default-features = false, features = ["parse-zero-copy"] }
```

### 使用 crates.io

```toml
# 在 Cargo.toml 中添加
sol-parser-sdk = "0.7.0"
```

或使用零拷贝解析器（最高性能）：

```toml
sol-parser-sdk = { version = "0.7.0", default-features = false, features = ["parse-zero-copy"] }
```

### 发布说明

#### v0.7.0

- 将公开 Solana 类型升级到 Solana 4，完整覆盖 Legacy、V0 和 V1 交易消息。
- 在生产 ShredStream 和 RPC 交易解码路径使用 `wincode 0.5.5`。同一份 21,000 字节 Entry 可复现基准中，解码延迟从旧 bincode 基线的 39.005 us/op 降到 8.956 us/op，降低 77.0%，速度提升 4.36 倍。
- 删除未使用的直接依赖并对齐 Agave 依赖族。normal 依赖图从 658 个 package/version 项降到 533 个，重复 crate 名从 111 个降到 28 个。
- Solana 4 client 依赖栈要求最低 Rust 版本为 1.91。
- 为 Meteora DLMM 增加用户 token 账户上下文，并通过 Serde 默认值保持旧数据兼容。

#### v0.6.6

- 为 PumpFun 交易事件增加 `pre_token_balance` 和 `post_token_balance`，单位为 mint 原始精度。
- 增加以 lamports 为单位的 `pre_sol_balance` 和 `post_sol_balance`，直接从交易 metadata 填充，无需额外 RPC 请求。
- 在捕获的 Yellowstone 基准 fixture 上，将 PumpFun 端到端解析延迟从 6.7349 us 降至 4.4293 us，降低 34.23%。
- 增加可复现的 Criterion 基准、Yellowstone 原始交易 fixture 和实时余额 delta 校验。

#### v0.6.4

- 为当前 Meteora DLMM swap 事件增加 `token_x_mint` 和 `token_y_mint` 上下文。
- 按事件池精确匹配 mint 账户，防止多段 DLMM 路由复用其他池的账户。
- 增加 2026-08-14 采集的主网 RPC 示例，覆盖直接 `swap` 和 CPI `swap2` 解析。
- 兼容反序列化尚未包含新 mint 字段的旧版 JSON。

#### v0.6.3

- 输出当前 Raydium LaunchLab 的 quote mint 和 global configuration 上下文，包括 USD1 池。
- 增加可选的交易费、优先费、compute budget 和 SWQoS tip 解析，覆盖 sol-trade-sdk 支持的全部服务商。
- 返回每笔已识别 tip 的服务商和收款地址；未启用交易成本解析时不分配内存，开销可忽略。
- 增加 2026-08-13 采集的 LaunchLab USD1 和交易成本主网交易 fixture，便于后续复用验证。

#### v0.6.2

- 支持当前 Meteora DLMM Anchor event-CPI 前缀布局，同时保留 legacy 后缀格式兼容。
- 按当前官方来源对齐 Meteora DLMM、Raydium CPMM/AMM V4 和 Orca Whirlpool 的事件与账户布局。
- 使用 occurrence-aware 日志/指令去重和 stack-aware CPI 合并，保留同一交易中的重复 swap 与流动性事件。
- 加入 2026-08-13 采集的可复用主网交易 fixture，覆盖 Meteora DLMM、PumpFun、PumpSwap、Raydium 和 Orca。

#### v0.6.1

- 在日志和 CPI/inner-instruction 路径中完整解析当前 PumpSwap Buy/Sell 事件尾部：cashback、buyback 费用、带符号 virtual quote reserves、boost 标记和 base supply。
- 默认和 zero-copy feature 统一使用经过校验的 PumpSwap trade decoder，同时保持历史事件布局兼容。
- 对截断尾部、非法 UTF-8、非法 Borsh bool 和字符串边界溢出直接拒绝，不再输出部分解析的事件。
- 对 buyback 和 boost 升级新增字段提供默认值，保持旧版序列化 PumpSwap 事件兼容。

#### v0.5.15

- 修复 Pump.fun `create_v2` 在 16 账户和 19 账户布局下的 quote mint 判断。
- 只有存在 19 账户 quote-pool 尾部账户时才读取追加的 quote mint；16 账户 `create_v2` 保持 SOL sentinel。
- 填充账户时按 create/create_v2 discriminator 选择对应指令，避免后续 buy/sell 指令覆盖 create 的 quote 字段。

#### v0.5.13

- gRPC 和 ShredStream 的 Pump.fun create/trade 输出会保留真实 WSOL quote mint（`So11111111111111111111111111111111111111112`）。
- 只有 legacy 或缺失 Pump.fun quote mint 字段时，才使用 Solscan SOL sentinel（`So11111111111111111111111111111111111111111`）。
- 增加合并覆盖：占位 SOL quote mint 可以被后续 log 或 instruction/account 上下文中的真实 WSOL quote mint 替换。

#### v0.5.11

- 解析 PumpSwap `create_pool` 指令参数，包括 `index`、注入数量、`coin_creator`、`is_mayhem_mode`、`is_cashback_coin`。
- 修正 PumpSwap `create_pool` 指令账户映射，按 IDL 填充 `pool`、`creator`、`base_mint`、`quote_mint`、LP/用户 token account。
- instruction 路径的 CreatePool 数据与 log 路径合并时，会保留 `is_cashback_coin`。
- 明确字段来源：PumpSwap log `CreatePoolEvent` IDL 不包含 `is_cashback_coin`；ShredStream/外层指令解析可从 instruction data 读取该字段，账户订阅可从 `Pool` account 读取权威字段。

#### v0.5.10

- PumpSwap `CreatePoolEvent` 与链上 IDL 对齐：事件暴露 `is_mayhem_mode`，但不暴露 `is_cashback_coin`。
- PumpSwap `is_cashback_coin` 保留在 `AccountPumpSwapPool` 账户事件中，因为该字段存储在链上 `Pool` account。
- 修复 PumpSwap CreatePool log payload 长度检查，包含最后的 `is_mayhem_mode` 字节。
- 文档明确 ShredStream CreatePool 事件无法恢复 `is_cashback_coin`，因为 Shred entry 不包含账户 body。

#### v0.5.9

- 实现真正的 Yellowstone gRPC `stop()`：会通知、abort 并等待当前订阅任务结束。
- 串行化 gRPC 订阅生命周期，避免并发 stop / re-subscribe 遗留旧的重连循环。
- 每次订阅使用独立 stop signal，避免新订阅误重置旧任务的停止状态。
- 流错误日志改为 `Grpc Stream error`，便于和 ShredStream 日志区分。
- 修复 warmup 测试对全局测试执行顺序的依赖。

#### v0.5.8

- ShredStream 示例增加可配置事件过滤 preset，包括 Pump.fun trade、create-trade、buy、sell、buy-exact-sol-in。
- 明确 Pump.fun `ix_name` 使用 IDL 原始 instruction name：`buy`、`buy_v2`、`buy_exact_sol_in`、`buy_exact_quote_in_v2`、`sell`、`sell_v2`。
- Pump.fun 过滤大类与 IDL 语义保持一致：`PumpFunBuy` 覆盖所有 buy 指令，`PumpFunSell` 覆盖所有 sell 指令，`PumpFunTrade` 覆盖所有 buy 和 sell 指令。
- 只订阅 `PumpFunTrade` 时，ShredStream 热路径统一输出 `DexEvent::PumpFunTrade`。

#### v0.5.5

- 对齐 Rust、Node.js、Python、Go 的 ShredStream 静态账户解析语义。
- V0 ALT-loaded 指令账户不再整条跳过，热路径用默认 pubkey 占位继续 best-effort 解析。
- 当 ShredStream 外层 program id 来自 ALT 且不在静态账户表中时，按候选 program id 做 discriminator fallback。
- 改进 Pump.fun ShredStream create/create_v2、v2 短账户交易和事件类型过滤的跨语言一致性。
- 更新 Pump.fun、PumpSwap、Pump Fees、Raydium、Orca、Meteora 的多协议路由与账户补全文档。

#### v0.5.4

- Pump.fun `create` 和 `create_v2` 统一投递为 canonical `PumpFunCreate` 事件。
- `PumpFunCreate` 和 `PumpFunCreateV2` 过滤器按同一个 create-family 订阅处理。
- canonical create 事件保留 create_v2 账户字段，Bot 不需要同时处理两个事件变体。
- 修复 gRPC log + instruction 双路径解析导致新 mint 回调重复的问题。

#### v0.5.3

- 保留真实的 Pump.fun v2 `ix_name`，包括 `buy_v2`、`sell_v2` 和 `buy_exact_quote_in_v2`。
- 改进 ShredStream 的 Pump.fun v2 解析，对 buy、sell、exact-quote 指令支持短账户列表 best-effort 解析。
- Pump.fun buy-family 过滤保持双向兼容，订阅 `PumpFunBuy` 或 `PumpFunBuyExactSolIn` 都可以匹配兼容的 buy 变体。
- 保留 ShredStream 对外层指令的 ALT/default-account best-effort 解析，同时明确 CPI/inner-only 仍无法从 Shred Entry 恢复。

### 性能测试

使用优化示例测试解析延迟：

```bash
# PumpFun 详细性能指标
cargo run --example pumpfun_with_metrics --release

# PumpSwap 详细性能指标（单事件明细 + 每 10 秒统计）
cargo run --example pumpswap_with_metrics --release


# PumpSwap 超低延迟测试
cargo run --example pumpswap_low_latency --release

# PumpSwap 事件 + MicroBatch 有序模式
cargo run --example pumpswap_ordered --release

# 预期输出：
# gRPC接收时间: 1234567890 μs
# 事件接收时间: 1234567900 μs
# 延迟时间: 10 μs  <-- 超低延迟！
```

### 示例列表

| 描述 | 运行命令 | 源码 |
|------|----------|------|
| **PumpFun** | | |
| PumpFun 事件解析 + 性能指标 | `cargo run --example pumpfun_with_metrics --release` | [examples/pumpfun_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_with_metrics.rs) |
| PumpFun 交易类型过滤 | `cargo run --example pumpfun_trade_filter --release` | [examples/pumpfun_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter.rs) |
| PumpFun 有序模式交易过滤 | `cargo run --example pumpfun_trade_filter_ordered --release` | [examples/pumpfun_trade_filter_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter_ordered.rs) |
| PumpFun 快速连接测试 | `cargo run --example pumpfun_quick_test --release` | [examples/pumpfun_quick_test.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_quick_test.rs) |
| 按签名解析 PumpFun 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` | [examples/parse_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pump_tx.rs) |
| 解析 PumpFun quote_mint 边界案例 | `TX_SIGNATURES=<sig1,sig2> cargo run --example parse_pumpfun_quote_cases --release` | [examples/parse_pumpfun_quote_cases.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpfun_quote_cases.rs) |
| 调试 PumpFun 交易 | `cargo run --example debug_pump_tx --release` | [examples/debug_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pump_tx.rs) |
| **PumpSwap** | | |
| PumpSwap 事件 + 性能统计 | `cargo run --example pumpswap_with_metrics --release` | [examples/pumpswap_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_with_metrics.rs) |
| PumpSwap 超低延迟 | `cargo run --example pumpswap_low_latency --release` | [examples/pumpswap_low_latency.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_low_latency.rs) |
| PumpSwap MicroBatch 有序 | `cargo run --example pumpswap_ordered --release` | [examples/pumpswap_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_ordered.rs) |
| 按签名解析 PumpSwap 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` | [examples/parse_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpswap_tx.rs) |
| 调试 PumpSwap 交易 | `cargo run --example debug_pumpswap_tx --release` | [examples/debug_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/debug_pumpswap_tx.rs) |
| **Meteora DAMM** | | |
| Meteora DAMM V2 事件 | `cargo run --example meteora_damm_grpc --release` | [examples/meteora_damm_grpc.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_grpc.rs) |
| 按签名解析 Meteora DAMM 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` | [examples/parse_meteora_damm_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_meteora_damm_tx.rs) |
| **非 Pump DEX dry-run 场景** | | |
| Raydium LaunchLab migration 过滤 | `cargo run --example raydium_launchlab_migration` | [examples/raydium_launchlab_migration.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_launchlab_migration.rs) |
| Raydium CPMM 新池过滤 | `cargo run --example raydium_cpmm_new_pool` | [examples/raydium_cpmm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_cpmm_new_pool.rs) |
| Raydium CLMM 价格计算 | `cargo run --example raydium_clmm_token_price` | [examples/raydium_clmm_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_clmm_token_price.rs) |
| Orca Whirlpool 价格计算 | `cargo run --example orca_whirlpool_token_price` | [examples/orca_whirlpool_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/orca_whirlpool_token_price.rs) |
| Meteora DAMM 新池基线 | `cargo run --example meteora_damm_new_pool` | [examples/meteora_damm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_new_pool.rs) |
| Meteora DBC 价格计算 | `cargo run --example meteora_dbc_token_price` | [examples/meteora_dbc_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_dbc_token_price.rs) |
| 钱包交易过滤 | `cargo run --example wallet_trade_filter` | [examples/wallet_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/wallet_trade_filter.rs) |
| gRPC 延迟与 slot 对比配置 | `cargo run --example grpc_latency_slot_compare` | [examples/grpc_latency_slot_compare.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/grpc_latency_slot_compare.rs) |
| **账户订阅** | | |
| Token 账户余额变化 | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` | [examples/token_balance_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_balance_listen.rs) |
| Nonce 账户状态变化 | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` | [examples/nonce_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/nonce_listen.rs) |
| Mint 账户信息 | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` | [examples/token_decimals_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_decimals_listen.rs) |
| PumpSwap 池账户 memcmp 订阅 | `cargo run --example pumpswap_pool_account_listen --release` | [examples/pumpswap_pool_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_pool_account_listen.rs) |
| 所有 ATA 订阅 | `cargo run --example mint_all_ata_account_listen --release` | [examples/mint_all_ata_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/mint_all_ata_account_listen.rs) |
| **ShredStream** | | |
| Jito ShredStream 订阅 | `cargo run --example shredstream_example --release` | [examples/shredstream_example.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/shredstream_example.rs) |
| **工具** | | |
| 动态更新订阅过滤器 | `cargo run --example dynamic_subscription --release` | [examples/dynamic_subscription.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/dynamic_subscription.rs) |
| 调试 PumpSwap 账户填充 | `cargo run --example test_account_filling --release` | [examples/test_account_filling.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/test_account_filling.rs) |

### 基本用法

```rust
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol,
    TransactionFilter, YellowstoneGrpc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 gRPC 客户端（默认 Unordered 模式）
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;
    
    // 或使用自定义配置启用有序模式
    let config = ClientConfig {
        order_mode: OrderMode::MicroBatch,  // 低延迟 + 有序
        micro_batch_us: 100,                // 100μs 批次窗口
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

    // 在解析前过滤事件，走最低延迟路径
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpSwapBuy,
        EventType::PumpSwapSell,
        EventType::RaydiumCpmmSwap,
    ]);

    // 订阅并获取无锁队列
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    ).await?;

    // 最小延迟消费事件
    tokio::spawn(async move {
        let mut spin_count = 0;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                // 处理事件（10-20μs 延迟！）
                println!("{:?}", event);
            } else {
                // 混合自旋等待策略
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

### ShredStream 使用（Jito）

ShredStream 通过直接订阅 Jito 的 ShredStream 服务提供超低延迟（比 gRPC 快约 50-100ms）：

```rust
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::shredstream::{ShredStreamClient, ShredStreamConfig};
use sol_parser_sdk::DexEvent;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 ShredStream 客户端
    let client = ShredStreamClient::new("http://127.0.0.1:10800").await?;

    // 或使用自定义配置
    let config = ShredStreamConfig {
        connection_timeout_ms: 5000,
        request_timeout_ms: 30000,
        max_decoding_message_size: 1024 * 1024 * 1024,
        reconnect_delay_ms: 1000,
        max_reconnect_attempts: 0, // 0 = 无限重连
    };
    let client = ShredStreamClient::new_with_config("http://127.0.0.1:10800", config).await?;

    // 使用 SDK 侧过滤，在事件转换前丢弃无关事件。
    // 如果要接收所有支持事件，可使用 `client.subscribe().await?`。
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpSwapBuy,
        EventType::RaydiumCpmmSwap,
    ]);
    let queue = client.subscribe_with_filter(Some(event_filter)).await?;

    // 消费事件
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

**ShredStream 限制：**
- 仅 `static_account_keys()` - ALT-loaded 指令账户会使用默认账户占位，外层指令会基于 data/discriminator 尽量解析
- 无 Inner Instructions - 无法从 ShredStream entry 恢复 CPI/inner-only 事件
- 无 block_time - 恒为 0
- tx_index 是 entry 内索引而非 slot 内索引

---

## 🏗️ 支持的协议

### DEX 协议
- ✅ **PumpFun** - Meme 币交易（超快零拷贝路径，含 v2 指令）
- ✅ **Pump Fees** - Pump 费用分成配置事件
- ✅ **PumpSwap** - PumpFun 交换协议
- ✅ **Raydium LaunchLab** - 代币发射平台
- ✅ **Raydium AMM V4** - 自动做市商
- ✅ **Raydium CLMM** - 集中流动性做市
- ✅ **Raydium CPMM** - 集中池做市
- ✅ **Orca Whirlpool** - 集中流动性 AMM
- ✅ **Meteora Pools** - 动态 AMM
- ✅ **Meteora DAMM v2** - 动态 AMM V2
- ✅ **Meteora DLMM** - 动态流动性做市
- 🚧 **Meteora DBC** - 已补 program id 与过滤常量，交易/账户 parser 后续补齐

### 事件类型
每个协议支持：
- 📈 **交易/兑换事件** - 买入/卖出交易
- 💧 **流动性事件** - 存款/提款
- 🏊 **池事件** - 池创建/初始化
- 🎯 **仓位事件** - 开仓/平仓（CLMM）

### 非 Pump DEX 支持矩阵

| 协议 | 事件 | 账户 | 示例 | 语言常量 |
|------|------|------|------|----------|
| Raydium LaunchLab | Trade、PoolCreate、Migrate | 待补 | Migration、buy/sell oracle 规划中 | Rust、Node、Python、Go |
| Raydium CPMM | Swap、Deposit、Withdraw、Initialize | AmmConfig、PoolState | New pool、token price | Rust、Node、Python、Go |
| Raydium CLMM | Swap、Pool、Position、Liquidity | AmmConfig、PoolState、TickArray | Token price | Rust、Node、Python、Go |
| Raydium AMM V4 | Swap、Deposit、Withdraw、Initialize2 | 待补 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Orca Whirlpool | Swap、Liquidity、Pool init | Whirlpool、Position、TickArray、FeeTier、Config | Token price | Rust、Node、Python、Go |
| Meteora Pools | Swap、Liquidity、Pool create、Fees | 待补 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DAMM V2 | Swap、Liquidity、Position | 待补 | New pool、token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DLMM | Swap、Liquidity、Bin/Position | 待补 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DBC | Swap、InitializePool、CurveComplete（Rust log parser） | 待补 | Token price、migration oracle 规划中 | Rust、Node、Python、Go |

跨语言基线见 [`protocols/canonical.json`](protocols/canonical.json)，当前审计和剩余 parser 工作见
[`docs/non-pump-dex-gap-analysis.md`](docs/non-pump-dex-gap-analysis.md)。

---

## ⚡ 性能特性

### 零拷贝解析
```rust
// PumpFun Trade 使用 512 字节栈缓冲区
const MAX_DECODE_SIZE: usize = 512;
let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

// 直接解码到栈，无堆分配
general_purpose::STANDARD
    .decode_slice(data_part.as_bytes(), &mut decode_buf)
    .ok()?;
```

### SIMD 模式匹配
```rust
// 预编译 SIMD 查找器（初始化一次）
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));

// 比 .contains() 快 3-10 倍
if PUMPFUN_FINDER.find(log_bytes).is_some() {
    return LogType::PumpFun;
}
```

### 事件类型过滤
```rust
// 单一事件类型超快路径
if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
    if log_type == LogType::PumpFun {
        return parse_pumpfun_trade(  // 零拷贝路径
            log, signature, slot, block_time, grpc_recv_us, is_created_buy
        );
    }
}
```

### 无锁队列
```rust
// 100,000 容量的 ArrayQueue
let queue = Arc::new(ArrayQueue::<DexEvent>::new(100_000));

// 非阻塞 push/pop（无互斥锁开销）
let _ = queue.push(event);
if let Some(event) = queue.pop() {
    // 处理事件
}
```

---

## 🎯 事件过滤

通过过滤特定事件减少处理开销：

### 示例：交易机器人
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

### 示例：池监控
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

`PumpSwapCreatePool` 包含 `is_mayhem_mode`。对于 `is_cashback_coin`，
ShredStream/外层指令解析会从 `create_pool` instruction args 读取；
log-only 的 `CreatePoolEvent` payload 因为 IDL 不包含该字段，会保持默认
`false`。账户里的权威值也可通过
`PumpSwapPoolAccountEvent.pool.is_cashback_coin` 读取。

**性能影响：**
- 减少 60-80% 的处理开销
- 降低内存使用
- 减少网络带宽

---

## 🔧 高级功能

### Create+Buy 检测
自动检测代币创建后立即购买的交易：

```rust
// 检测 "Program data: GB7IKAUcB3c..." 模式
let has_create = detect_pumpfun_create(logs);

// 在 Trade 事件上设置 is_created_buy 标志
if has_create {
    trade_event.is_created_buy = true;
}
```

### Pump.fun Bonding Curve v2（buy_v2 / sell_v2 / buy_exact_quote_in_v2）

SDK 已支持 Pump.fun Bonding Curve 升级引入的新 v2 交易指令。来自 `buy_v2`、`sell_v2` 和 `buy_exact_quote_in_v2` 的事件日志通过相同的零拷贝路径解析，并映射到已有事件类型：

| ix_name in TradeEvent | DexEvent 枚举变体 |
|----------------------|-----------------|
| `"buy"` / `"buy_v2"` / `"buy_exact_quote_in"` / `"buy_exact_quote_in_v2"` | `DexEvent::PumpFunBuy` |
| `"sell"` / `"sell_v2"` | `DexEvent::PumpFunSell` |
| `"buy_exact_sol_in"` | `DexEvent::PumpFunBuyExactSolIn` |

无需修改现有事件处理代码 — v2 事件通过相同的 `PumpFunTradeEvent` 结构体投递，`ix_name` 字段会正确填充。指令层已识别 `buy_v2`（`[184, 23, 238, 97, 103, 197, 211, 61]`）、`sell_v2`（`[93, 246, 130, 60, 231, 233, 64, 178]`）和 `buy_exact_quote_in_v2`（`[194, 171, 28, 70, 104, 77, 91, 47]`）的 discriminator。

`CreateEvent` 现在也会暴露 `quote_mint` 和 `virtual_quote_reserves`，USDC 报价池可以据此和 SOL 池区分，并使用正确的 quote 侧初始储备。

### 动态订阅
无需重连即可更新过滤器：

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### 顺序模式
根据场景选择延迟与顺序的平衡：

```rust
use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

// 超低延迟（无顺序保证）
let config = ClientConfig {
    order_mode: OrderMode::Unordered,
    ..ClientConfig::default()
};

// 低延迟微批次排序（50-200μs）
let config = ClientConfig {
    order_mode: OrderMode::MicroBatch,
    micro_batch_us: 100,  // 100μs 批次窗口
    ..ClientConfig::default()
};

// 流式排序，连续序列立即释放（0.1-5ms）
let config = ClientConfig {
    order_mode: OrderMode::StreamingOrdered,
    order_timeout_ms: 50,  // 不完整序列超时
    ..ClientConfig::default()
};

// 完整 slot 排序（1-50ms，等待整个 slot）
let config = ClientConfig {
    order_mode: OrderMode::Ordered,
    order_timeout_ms: 100,
    ..ClientConfig::default()
};
```

### 性能指标
```rust
let config = ClientConfig {
    enable_metrics: true,
    ..ClientConfig::default()
};

let grpc = YellowstoneGrpc::new_with_config(endpoint, token, config)?;
```

---

## 📁 项目结构

```
src/
├── core/
│   └── events.rs          # 事件定义
├── grpc/
│   ├── client.rs          # Yellowstone gRPC 客户端
│   ├── buffers.rs         # SlotBuffer 和 MicroBatchBuffer
│   └── types.rs           # OrderMode、ClientConfig、过滤器
├── shredstream/
│   ├── client.rs          # Jito ShredStream 客户端
│   ├── config.rs          # ShredStreamConfig
│   └── proto/             # Protobuf 定义
├── logs/
│   ├── optimized_matcher.rs  # SIMD 日志检测
│   ├── zero_copy_parser.rs   # 零拷贝解析
│   ├── pumpfun.rs         # PumpFun 解析器
│   ├── raydium_*.rs       # Raydium 解析器
│   ├── orca_*.rs          # Orca 解析器
│   └── meteora_*.rs       # Meteora 解析器
├── instr/
│   └── *.rs               # 指令解析器
├── warmup/
│   └── mod.rs             # 解析器预热（自动调用）
└── lib.rs
```

---

## 🚀 优化技术

### 1. **SIMD 字符串匹配**
- 所有 `.contains()` 替换为 `memmem::Finder`
- 性能提升 3-10 倍
- 预编译静态查找器

### 2. **零拷贝解析**
- 栈分配缓冲区（512 字节）
- 热路径无堆分配
- 内联辅助函数

### 3. **事件类型过滤**
- 协议级别早期过滤
- 条件 Create 检测
- 单类型超快路径

### 4. **无锁队列**
- ArrayQueue（100K 容量）
- 自旋等待混合策略
- 无互斥锁开销

### 5. **激进内联**
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

## 📊 性能基准

### 解析延迟（Release 模式）
| 协议 | 平均延迟 | 最小 | 最大 |
|----------|-------------|-----|-----|
| PumpFun Trade（零拷贝） | 10-15μs | 8μs | 20μs |
| Raydium AMM V4 Swap | 15-20μs | 12μs | 25μs |
| Orca Whirlpool Swap | 15-20μs | 12μs | 25μs |

### SIMD 模式匹配
| 操作 | 优化前（contains） | 优化后（SIMD） | 提升 |
|-----------|------------------|--------------|---------|
| 协议检测 | 50-100ns | 10-20ns | 3-10x |
| Create 事件检测 | 150ns | 30ns | 5x |

---

## 📄 许可证

MIT License

## 📞 联系方式

- **仓库**: https://github.com/0xfnzero/sol-parser-sdk
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE

---

## ⚠️ 性能建议

1. **使用事件过滤** - 源头过滤可获得 60-80% 性能提升
2. **Release 模式运行** - `cargo build --release` 获得完整优化
3. **使用 sudo 测试** - `sudo cargo run --example basic --release` 获得精确计时
4. **监控延迟** - 生产环境检查 `grpc_recv_us` 和队列延迟
5. **调整队列大小** - 根据吞吐量调整 ArrayQueue 容量
6. **自旋等待策略** - 根据使用场景调整自旋计数（默认：1000）

## 🔬 开发

```bash
# 运行测试
cargo test

# 构建 release 二进制
cargo build --release

# 生成文档
cargo doc --open
```
