# Solana DEX Event Streaming with gRPC

这个示例展示如何使用 `solana-streamer-sdk` 进行 Solana DEX 事件的流式处理，支持 gRPC 订阅、事件过滤和账号过滤。

## 功能特性

✅ **gRPC 订阅** - 连接到 Yellowstone gRPC 端点
✅ **事件过滤** - 按事件类型筛选
✅ **账号过滤** - 按程序 ID 和账户地址筛选
✅ **交易过滤** - 灵活的交易数据过滤
✅ **DexEvent 模式匹配** - 使用标准 Rust match 模式，不依赖宏

## 基本用法

```rust
use solana_streamer_sdk::{
    DexEvent,
    streaming::{
        ClientConfig, YellowstoneGrpc, Protocol,
        TransactionFilter, AccountFilter, EventTypeFilter,
        program_ids::*,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 gRPC 客户端配置
    let mut config = ClientConfig::low_latency();
    config.enable_metrics = true;

    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    // 2. 定义要监控的协议
    let protocols = vec![
        Protocol::PumpFun,
        Protocol::PumpSwap,
        Protocol::Bonk,
        Protocol::RaydiumCpmm,
        Protocol::RaydiumClmm,
        Protocol::RaydiumAmmV4,
    ];

    // 3. 设置账户过滤器
    let account_include = vec![
        PUMPFUN_PROGRAM_ID.to_string(),
        PUMPSWAP_PROGRAM_ID.to_string(),
        BONK_PROGRAM_ID.to_string(),
        RAYDIUM_CPMM_PROGRAM_ID.to_string(),
        RAYDIUM_CLMM_PROGRAM_ID.to_string(),
        RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
    ];

    let transaction_filter = TransactionFilter {
        account_include: account_include.clone(),
        account_exclude: vec![],
        account_required: vec![],
    };

    let account_filter = AccountFilter {
        account: vec![],
        owner: account_include,
        filters: vec![]
    };

    // 4. 设置事件过滤器（可选）
    let event_type_filter = None; // 接收所有事件
    // 或者只接收特定事件类型：
    // let event_type_filter = Some(EventTypeFilter::include_only(vec![
    //     StreamingEventType::PumpFunTrade,
    //     StreamingEventType::BonkTrade,
    // ]));

    // 5. 定义事件处理回调
    let callback = create_event_callback();

    // 6. 开始订阅
    grpc.subscribe_events_immediate(
        protocols,
        None, // slot_filter
        vec![transaction_filter],
        vec![account_filter],
        event_type_filter,
        None, // timeout
        callback,
    ).await?;

    Ok(())
}

// 7. 事件处理函数 - 使用标准 match 模式
fn create_event_callback() -> impl Fn(DexEvent) {
    |event: DexEvent| {
        match event {
            // Bonk 事件
            DexEvent::BonkTrade(e) => {
                println!("Bonk 交易: pool={}, user={}, is_buy={}",
                    e.pool_state, e.user, e.is_buy);
            },
            DexEvent::BonkPoolCreate(e) => {
                println!("Bonk 池创建: pool={}, symbol={}",
                    e.pool_state, e.base_mint_param.symbol);
            },

            // PumpFun 事件
            DexEvent::PumpFunTrade(e) => {
                println!("PumpFun 交易: mint={}, user={}, is_buy={}",
                    e.mint, e.user, e.is_buy);
            },
            DexEvent::PumpFunCreate(e) => {
                println!("PumpFun 代币创建: mint={}, bonding_curve={}",
                    e.mint, e.bonding_curve);
            },

            // Raydium 事件
            DexEvent::RaydiumCpmmSwap(e) => {
                println!("Raydium CPMM 交换: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmSwap(e) => {
                println!("Raydium CLMM 交换: pool={}", e.pool);
            },
            DexEvent::RaydiumAmmV4Swap(e) => {
                println!("Raydium AMM V4 交换: amm={}", e.amm);
            },

            // 其他事件
            _ => {
                println!("其他事件类型");
            },
        }
    }
}
```

## 配置选项

### 客户端配置

```rust
// 低延迟配置
let config = ClientConfig::low_latency();

// 高吞吐量配置
let config = ClientConfig::high_throughput();

// 自定义配置
let mut config = ClientConfig::default();
config.enable_metrics = true;
config.connection_timeout_ms = 5000;
```

### 事件类型过滤

```rust
use solana_streamer_sdk::streaming::{EventTypeFilter, StreamingEventType};

// 只接收特定事件类型
let filter = EventTypeFilter::include_only(vec![
    StreamingEventType::PumpFunTrade,
    StreamingEventType::BonkTrade,
    StreamingEventType::RaydiumCpmmSwap,
]);

// 排除特定事件类型
let filter = EventTypeFilter::exclude_types(vec![
    StreamingEventType::TokenAccount,
    StreamingEventType::NonceAccount,
]);
```

### 账户和交易过滤

```rust
// 交易过滤器
let transaction_filter = TransactionFilter::new()
    .include_account("program_id_1")
    .include_account("program_id_2")
    .exclude_account("unwanted_program_id");

// 账户过滤器
let account_filter = AccountFilter::new()
    .add_owner("program_id_1")
    .add_owner("program_id_2")
    .add_account("specific_account_address");
```

## 支持的协议

- **PumpFun** - 代币创建和交易
- **PumpSwap** - DEX 交易和流动性
- **Bonk** - Bonk 协议交易
- **Raydium CPMM** - Raydium 恒定乘积做市商
- **Raydium CLMM** - Raydium 集中流动性做市商
- **Raydium AMM V4** - Raydium AMM 第四版
- **Orca Whirlpool** - Orca 集中流动性
- **Meteora** - Meteora 协议

## 支持的事件类型

每个协议都支持多种事件类型，包括：

- 交易/交换事件
- 流动性添加/移除事件
- 池创建/初始化事件
- 位置管理事件
- 账户变更事件

## 运行示例

```bash
# 编译示例
cargo check --example grpc_streaming_example

# 运行示例（需要有效的 gRPC 端点）
cargo run --example grpc_streaming_example
```

## 核心优势

1. **类型安全** - 完整的 Rust 类型系统支持
2. **性能优化** - 低延迟配置和高效解析
3. **灵活过滤** - 多层过滤系统
4. **易于使用** - 标准 Rust 模式匹配，无需宏
5. **可扩展** - 易于添加新协议和事件类型

这个流式处理系统为 Solana DEX 事件监控提供了完整的解决方案，适用于交易机器人、分析工具和实时监控应用。