<div align="center">
    <h1>Sol Parser SDK — Rust</h1>
    <p>面向 Solana DEX 的 Rust 事件解析 SDK</p>
    <a href="https://crates.io/crates/sol-parser-sdk"><img src="https://img.shields.io/crates/v/sol-parser-sdk.svg" alt="sol-parser-sdk"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
</div>

<p align="center">
    <a href="README.md">English</a> · <a href="README_CN.md">中文</a> ·
    <a href="examples/README.md">示例</a> ·
    <a href="docs/README.md">文档</a> ·
    <a href="RELEASE_NOTES.md">发布与迁移</a> ·
    <a href="#sdk-版本与相关项目">相关 SDK</a>
</p>

从 gRPC、ShredStream、RPC 交易及协议账户解析类型化事件，用于索引、监控和交易系统。

当前版本：**0.9.1**。升级前请阅读 [迁移说明](RELEASE_NOTES.md)；历史变更见 [版本记录](CHANGELOG_CN.md)。

## 目录

- [SDK 版本与相关项目](#sdk-版本与相关项目)
- [快速开始](#快速开始)
- [这个 SDK 适合什么场景](#这个-sdk-适合什么场景)
- [交易来源与解析边界](#交易来源与解析边界)
- [支持的协议](#支持的协议)
- [使用文档](#使用文档)

<a id="sdk-版本与相关项目"></a>

## 📦 SDK 版本与相关项目

各仓库版本独立，依赖兼容要求以对应 SDK 的安装说明为准。

| 语言 | 仓库 | 用途 | 版本 |
|----------|------------|-------------|---------|
| Rust | [sol-parser-sdk](https://github.com/0xfnzero/sol-parser-sdk) | DEX 交易与账户事件解析 | `v0.9.1` |
| Node.js | [sol-parser-sdk-nodejs](https://github.com/0xfnzero/sol-parser-sdk-nodejs) | DEX 交易与账户事件解析 | `v0.9.1` |
| Python | [sol-parser-sdk-python](https://github.com/0xfnzero/sol-parser-sdk-python) | DEX 交易与账户事件解析 | `v0.9.1` |
| Go | [sol-parser-sdk-golang](https://github.com/0xfnzero/sol-parser-sdk-golang) | DEX 交易与账户事件解析 | `v0.9.1` |
| Rust | [sol-trade-sdk](https://github.com/0xfnzero/sol-trade-sdk) | 交易构建、签名与多通道提交 | `v9.0.1` |
| Node.js | [sol-trade-sdk-nodejs](https://github.com/0xfnzero/sol-trade-sdk-nodejs) | 交易构建、签名与多通道提交 | `v9.0.1` |
| Python | [sol-trade-sdk-python](https://github.com/0xfnzero/sol-trade-sdk-python) | 交易构建、签名与多通道提交 | `v9.0.1` |
| Go | [sol-trade-sdk-golang](https://github.com/0xfnzero/sol-trade-sdk-golang) | 交易构建、签名与多通道提交 | `v9.0.1` |
| Rust | [sol-shred-sdk](https://github.com/0xfnzero/sol-shred-sdk) | 原始 Shred 解码与 ShredStream 指令事件 | `v6.0.1` |
| Rust | [solana-streamer](https://github.com/0xfnzero/solana-streamer) | gRPC / ShredStream 订阅与事件流封装 | `v5.0.0` |
| Rust | [sol-trade-router-sdk](https://github.com/0xfnzero/sol-trade-router-sdk) | 链上多跳 Router 与客户端 SDK | `v0.3.0` |

<a id="快速开始"></a>

## 🔥 快速开始

### 安装

```toml
# 在 Cargo.toml 中添加
sol-parser-sdk = "0.9.1"
```

或使用零拷贝解析器（最高性能）：

```toml
sol-parser-sdk = { version = "0.9.1", default-features = false, features = ["parse-zero-copy"] }
```


### 从源码使用

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
| PumpFun 事件解析 + 性能指标 | `cargo run --example pumpfun_with_metrics --release` | [examples/pumpfun_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_with_metrics.rs) | `v0.9.1` |
| PumpFun 交易类型过滤 | `cargo run --example pumpfun_trade_filter --release` | [examples/pumpfun_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter.rs) | `v0.9.1` |
| PumpFun 有序模式交易过滤 | `cargo run --example pumpfun_trade_filter_ordered --release` | [examples/pumpfun_trade_filter_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_trade_filter_ordered.rs) | `v0.9.1` |
| PumpFun 快速连接测试 | `cargo run --example pumpfun_quick_test --release` | [examples/pumpfun_quick_test.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpfun_quick_test.rs) | `v0.9.1` |
| 按签名解析 PumpFun 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` | [examples/parse_pump_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pump_tx.rs) | `v0.9.1` |
| 解析 PumpFun quote_mint 边界案例 | `TX_SIGNATURES=<sig1,sig2> cargo run --example parse_pumpfun_quote_cases --release` | [examples/parse_pumpfun_quote_cases.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpfun_quote_cases.rs) | `v0.9.1` |
| 抓取任意交易并离线回放（保存原始 meta/logs/账户） | `cargo run --example rpc_transaction_replay -- --fetch <SIGNATURE> /tmp/transaction.json` | [examples/rpc_transaction_replay.rs](examples/rpc_transaction_replay.rs) | `v0.9.1` |
| **PumpSwap** | | |
| PumpSwap 事件 + 性能统计 | `cargo run --example pumpswap_with_metrics --release` | [examples/pumpswap_with_metrics.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_with_metrics.rs) | `v0.9.1` |
| PumpSwap 超低延迟 | `cargo run --example pumpswap_low_latency --release` | [examples/pumpswap_low_latency.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_low_latency.rs) | `v0.9.1` |
| PumpSwap MicroBatch 有序 | `cargo run --example pumpswap_ordered --release` | [examples/pumpswap_ordered.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_ordered.rs) | `v0.9.1` |
| 按签名解析 PumpSwap 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` | [examples/parse_pumpswap_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_pumpswap_tx.rs) | `v0.9.1` |
| **Meteora DAMM** | | |
| Meteora DAMM V2 事件 | `cargo run --example meteora_damm_grpc --release` | [examples/meteora_damm_grpc.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_grpc.rs) | `v0.9.1` |
| 按签名解析 Meteora DAMM 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` | [examples/parse_meteora_damm_tx.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/parse_meteora_damm_tx.rs) | `v0.9.1` |
| **非 Pump DEX dry-run 场景** | | |
| LaunchLab migration 过滤 | `cargo run --example raydium_launchlab_migration` | [examples/raydium_launchlab_migration.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_launchlab_migration.rs) | `v0.9.1` |
| Raydium CPMM 新池过滤 | `cargo run --example raydium_cpmm_new_pool` | [examples/raydium_cpmm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_cpmm_new_pool.rs) | `v0.9.1` |
| Raydium CLMM 价格计算 | `cargo run --example raydium_clmm_token_price` | [examples/raydium_clmm_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/raydium_clmm_token_price.rs) | `v0.9.1` |
| Orca Whirlpool 价格计算 | `cargo run --example orca_whirlpool_token_price` | [examples/orca_whirlpool_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/orca_whirlpool_token_price.rs) | `v0.9.1` |
| Meteora DAMM 新池基线 | `cargo run --example meteora_damm_new_pool` | [examples/meteora_damm_new_pool.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_damm_new_pool.rs) | `v0.9.1` |
| Meteora DBC 价格计算 | `cargo run --example meteora_dbc_token_price` | [examples/meteora_dbc_token_price.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/meteora_dbc_token_price.rs) | `v0.9.1` |
| 钱包交易过滤 | `cargo run --example wallet_trade_filter` | [examples/wallet_trade_filter.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/wallet_trade_filter.rs) | `v0.9.1` |
| gRPC 延迟与 slot 对比配置 | `cargo run --example grpc_latency_slot_compare` | [examples/grpc_latency_slot_compare.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/grpc_latency_slot_compare.rs) | `v0.9.1` |
| **账户订阅** | | |
| Token 账户余额变化 | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` | [examples/token_balance_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_balance_listen.rs) | `v0.9.1` |
| Nonce 账户状态变化 | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` | [examples/nonce_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/nonce_listen.rs) | `v0.9.1` |
| Mint 账户信息 | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` | [examples/token_decimals_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/token_decimals_listen.rs) | `v0.9.1` |
| PumpSwap 池账户 memcmp 订阅 | `cargo run --example pumpswap_pool_account_listen --release` | [examples/pumpswap_pool_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/pumpswap_pool_account_listen.rs) | `v0.9.1` |
| 所有 ATA 订阅 | `cargo run --example mint_all_ata_account_listen --release` | [examples/mint_all_ata_account_listen.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/mint_all_ata_account_listen.rs) | `v0.9.1` |
| **ShredStream** | | |
| Jito ShredStream 订阅 | `cargo run --example shredstream_example --release` | [examples/shredstream_example.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/shredstream_example.rs) | `v0.9.1` |
| **工具** | | |
| 动态更新订阅过滤器 | `cargo run --example dynamic_subscription --release` | [examples/dynamic_subscription.rs](https://github.com/0xfnzero/sol-parser-sdk/blob/main/examples/dynamic_subscription.rs) | `v0.9.1` |

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

<a id="这个-sdk-适合什么场景"></a>

## 这个 SDK 适合什么场景

`sol-parser-sdk` 是 Solana DEX 事件的底层 Rust 解析核心，适合交易机器人、跟单管道、狙击机器人、索引服务和流处理系统，从 Yellowstone gRPC 交易、Jito ShredStream entry、RPC 交易 payload 或账户订阅中快速解析出强类型事件。

| 方向 | 覆盖范围 |
|------|----------|
| 解析输入 | Yellowstone gRPC、ShredStream、RPC 交易、编码交易、协议账户数据 |
| DEX 协议 | PumpFun、PumpSwap、Pump Fees、LaunchLab（含 StonkFun）、Raydium CPMM、Raydium CLMM、Raydium AMM V4、Meteora DAMM v2、Meteora DLMM、Meteora DBC、Orca Whirlpool |
| 解析后端 | 默认 Borsh 解析器便于维护，也可为低延迟热路径启用 zero-copy 解析器 |
| 相关 SDK | 如果需要更高层的事件流封装，请使用 [solana-streamer](https://github.com/0xfnzero/solana-streamer) |

<a id="交易来源与解析边界"></a>

## 交易来源与解析边界

gRPC/RPC 使用提供的执行元数据和实际 CPI 指令；Shred 只能看到外层指令与本地已解析的账户。未知第三方指令在自身账户中引用 Pump.fun 时，可回调 `PumpFunTrade` 候选，携带 `third_program` 原始输入与 `Unresolved` 状态；这不证明交易已执行，也不确定方向或金额。买入／卖出专用过滤器不会把未知方向当作成交。

解析热路径不调用 RPC。按 `parse_status` 判断证据，缺失字段的默认值不代表实际零成交。账户顺序与原始指令数据保留在 `third_program`；交易序号和接收时间仍在事件元数据中，本地 Shred 序号不等于全 slot 执行顺序。详见 [第三方包装交易](docs/wrapped-pumpfun.md)。

<a id="股票行情订阅"></a>

## 股票行情订阅

可选的股票／ETF 成交价订阅支持用户扩展目录，缺少实时换算价或流动性时保持未知。默认目录按已验证池子的流动侧储备至少 500 美元筛选，不设交易量门槛，同一 mint 优先已知费用更低的池子。历史目录不代表实时可成交深度。

详见 [股票行情](docs/stock-prices.md) 与 [跨 DEX 股票路由](docs/cross-dex-stock-routes.md)；离线回放使用 `cargo run --example stock_route_replay`。

<a id="性能亮点"></a>

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
| **Ordered** | 1-50ms | 缓存 slot 排序，新 slot 或超时触发输出 |

Ordered 保留最后输出的 `(slot, tx_index)` 水位。新 slot 刷新会关闭旧 slot；超时刷新后，同 slot 中更高索引的交易仍可继续输出。已关闭 slot 或水位之前/等于水位的迟到数据会被丢弃，并记录 `Ordered continuity break` 警告。同一交易内事件保持解析顺序；此模式不保证上游数据完整。

### 🚀 优化特性
- ✅ **零堆分配** 热路径无堆分配
- ✅ **SIMD 模式匹配** 所有协议检测 SIMD 加速
- ✅ **静态预编译查找器** 字符串搜索零开销
- ✅ **激进内联** 关键函数强制内联
- ✅ **事件类型过滤** 精准解析目标事件
- ✅ **条件 Create 检测** 仅在需要时检测
- ✅ **多种顺序模式** 延迟与顺序的灵活平衡

<a id="支持的协议"></a>

## 🏗️ 支持的协议

### DEX 协议
- ✅ **PumpFun** - Meme 币交易（超快零拷贝路径，含 v2 指令）
- ✅ **Pump Fees** - Pump 费用分成配置事件
- ✅ **PumpSwap** - PumpFun 交换协议
- ✅ **LaunchLab** - 代币发射平台，支持识别 StonkFun
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
| LaunchLab | Trade、PoolCreate、Migrate | 池/配置快照 | Migration、buy/sell oracle 规划中 | Rust、Node、Python、Go |
| Raydium CPMM | Swap、Deposit、Withdraw、Initialize | AmmConfig、PoolState | New pool、token price | Rust、Node、Python、Go |
| Raydium CLMM | Swap、Pool、Position、Liquidity | AmmConfig、PoolState、TickArray | Token price | Rust、Node、Python、Go |
| Raydium AMM V4 | Swap、Deposit、Withdraw、Initialize2 | 待补 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Orca Whirlpool | Swap、Liquidity、Pool init | Whirlpool、Position、TickArray、FeeTier、Config | Token price | Rust、Node、Python、Go |
| Meteora Pools | Swap、Liquidity、Pool create、Fees | 待补 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DAMM V2 | Swap、Liquidity、Position | 待补 | New pool、token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DLMM | Swap、Liquidity、Bin/Position | 池/bin array/bitmap 快照 | Token price oracle 规划中 | Rust、Node、Python、Go |
| Meteora DBC | Swap、InitializePool、CurveComplete（Rust log parser） | 待补 | Token price、migration oracle 规划中 | Rust、Node、Python、Go |

跨语言基线见 [`protocols/canonical.json`](protocols/canonical.json)。通过 `AccountRawSnapshot` 显式订阅原始账户；专用快照覆盖 LaunchLab 池/配置、DLMM 池/bin array/bitmap、Whirlpool 动态数组/adaptive Oracle 和 CLMM bitmap extension。

<a id="性能特性"></a>

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

<a id="事件过滤"></a>

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
- 跳过未订阅事件的解析，实际收益需按交易类型和负载测量
- 降低内存使用
- 减少网络带宽

<a id="高级功能"></a>

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

// 缓存 slot 排序（新 slot 或超时触发输出）
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

<a id="项目结构"></a>

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

<a id="优化技术"></a>

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

<a id="性能基准"></a>

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

<a id="性能建议"></a>

## ⚠️ 性能建议

1. **使用事件过滤** - 跳过不需要的事件类型，使用代表性流量测量实际收益
2. **Release 模式运行** - `cargo build --release` 获得完整优化
3. **可重复测量** - 固定构建配置、输入样本和机器负载；提权不会让计时更精确
4. **监控延迟** - 生产环境检查 `grpc_recv_us` 和队列延迟
5. **调整队列大小** - 根据吞吐量调整 ArrayQueue 容量
6. **自旋等待策略** - 根据使用场景调整自旋计数（默认：1000）

<a id="开发"></a>

## 🔬 开发

```bash
# 运行测试
cargo test

# 构建 release 二进制
cargo build --release

# 生成文档
cargo doc --open
```

PumpFun create/create_v2 的共享主网样本、重放方法和验证边界见 [验证说明](https://github.com/0xfnzero/sol-parser-sdk-golang/tree/main/validation/pumpfun_create_20261007)。

<a id="使用文档"></a>

## 使用文档

从 [示例入口](examples/README.md) 和 [文档索引](docs/README.md) 开始。
示例统一使用 `协议或领域_对象_动作` 命名；保留既有公开名称及官方 IDL 方法名。
股票行情与跨 DEX 路由见 [行情说明](docs/stock-prices.md) 和
[股票交易路径](docs/cross-dex-stock-routes.md)，历史目录不代表实时可成交深度。

- [账户订阅](docs/ACCOUNT_SUBSCRIPTIONS.md)
- [路由分析](docs/ROUTE_ANALYSIS.md)
- [CPMM creator-fee](docs/cpmm-creator-fee-share.md)
- [Pump/PumpSwap 账户布局](docs/PUMP_PUMP_AMM_PARSER_ACCOUNTS.md)
- [gRPC 与 RPC](docs/grpc-vs-rpc.md)
- [延迟测量排错](docs/troubleshooting-latency.md)


<a id="许可证"></a>

## 📄 许可证

MIT License

<a id="联系方式"></a>

## 📞 联系方式

- **仓库**: https://github.com/0xfnzero/sol-parser-sdk
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE


股票池同步：33 种官方股票/ETF、58 个股票池；配置扩展、USDC 起点、缓存选路与使用边界见 [11 SDK 同步说明](docs/STOCK_BRIDGE_SYNC.md)。
