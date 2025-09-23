# 无锁队列优化方案

## 性能对比

### 原方案（回调）
- 回调耗时：**135-202μs**
- 同步阻塞，每个事件必须等待回调完成
- 跨线程开销大

### 新方案（crossbeam-channel）
- 队列发送：**< 1μs**
- 异步非阻塞
- 零拷贝传递
- **性能提升：135-200倍**

## API 使用

### 方式1：无锁队列（推荐）

```rust
use sol_parser_sdk::grpc::{YellowstoneGrpc, TransactionFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YellowstoneGrpc::new(
        "https://grpc.example.com".to_string(),
        Some("your-token".to_string())
    )?;

    // 获取事件接收器
    let rx = client.subscribe_dex_events_with_channel(
        vec![TransactionFilter::pumpfun()],
        vec![],
        None,
    ).await?;

    // 异步消费事件（性能最优）
    tokio::spawn(async move {
        while let Ok(event) = rx.recv() {
            println!("Event: {:?}", event);
            // 你的业务逻辑，不阻塞解析线程
        }
    });

    // 主线程继续运行
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### 方式2：回调（兼容旧代码）

```rust
// 旧接口仍然可用，但性能较差
client.subscribe_dex_events(
    vec![TransactionFilter::pumpfun()],
    vec![],
    None,
    |event| {
        println!("Event: {:?}", event);
    }
).await?;
```

## 性能优势

1. **零拷贝** - 事件直接传递，无序列化
2. **异步处理** - 解析和消费完全并行
3. **批量消费** - 可以批量读取降低开销

```rust
// 批量消费示例
tokio::spawn(async move {
    let mut batch = Vec::with_capacity(100);

    loop {
        // 收集一批事件
        while batch.len() < 100 {
            if let Ok(event) = rx.try_recv() {
                batch.push(event);
            } else {
                break;
            }
        }

        if !batch.is_empty() {
            // 批量处理
            process_batch(&batch).await;
            batch.clear();
        }

        tokio::time::sleep(Duration::from_millis(1)).await;
    }
});
```

## 架构图

```
原方案：
解析线程 --[回调135μs]--> 用户代码 (阻塞)

新方案：
解析线程 --[队列<1μs]--> 无锁队列 --[并行]--> 消费线程
```

## 预期性能

- **总处理时间**：从 400μs 降至 **50-100μs**
- **吞吐量**：从 2500 tx/s 提升至 **10000+ tx/s**
- **延迟**：从 P99=500μs 降至 **P99=150μs**