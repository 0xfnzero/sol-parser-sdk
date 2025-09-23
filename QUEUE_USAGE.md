# 队列版本使用示例

## 修改完成

✅ 所有日志解析函数已添加 `grpc_recv_us` 参数
✅ metadata在创建时直接设置 `grpc_recv_us`
✅ 队列接收端自动打印耗时

## 使用方式

### 方法1: 队列接收（推荐，性能最优）

```rust
use sol_parser_sdk::grpc::{YellowstoneGrpc, TransactionFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YellowstoneGrpc::new(
        "https://your-grpc-endpoint.com".to_string(),
        Some("your-token".to_string())
    )?;

    // 获取事件接收器（无锁队列）
    let rx = client.subscribe_dex_events_with_channel(
        vec![TransactionFilter::pumpfun()],
        vec![],
        None,
    ).await?;

    // 异步消费事件
    tokio::spawn(async move {
        while let Ok(event) = rx.recv() {
            // 自动打印：⏱️  队列接收耗时: XXXμs
            println!("收到事件: {:?}", event);
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### 方法2: 回调方式（兼容旧代码）

```rust
// 回调版本内部使用队列，会自动打印耗时
client.subscribe_dex_events(
    vec![TransactionFilter::pumpfun()],
    vec![],
    None,
    |event| {
        // ⏱️  队列接收耗时: XXXμs （自动打印）
        println!("Event: {:?}", event);
    }
).await?;
```

## 性能指标

**端到端耗时** = 从gRPC接收交易 → 队列发送 → 队列接收

输出示例：
```
⏱️  队列接收耗时: 125μs
```

包含：
- 解析时间
- 填充账户时间
- 队列传输时间

预期性能：**50-200μs**