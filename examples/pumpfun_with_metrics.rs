use sol_parser_sdk::core::now_micros; // 使用 SDK 的高性能时钟
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, Protocol, TransactionFilter,
    YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// 辅助函数：更新最小值和最大值
fn update_min_max(min: &Arc<AtomicU64>, max: &Arc<AtomicU64>, value: u64) {
    // 更新最小值
    let mut current_min = min.load(Ordering::Relaxed);
    while value < current_min {
        match min.compare_exchange(current_min, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current_min = x,
        }
    }

    // 更新最大值
    let mut current_max = max.load(Ordering::Relaxed);
    while value > current_max {
        match max.compare_exchange(current_max, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current_max = x,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("Starting Sol Parser SDK Example with Metrics...");
    run_example().await?;
    Ok(())
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Subscribing to Yellowstone gRPC events...");

    // Create low-latency configuration
    let config = ClientConfig {
        enable_metrics: true,
        connection_timeout_ms: 10000,
        request_timeout_ms: 30000,
        enable_tls: true,
        ..Default::default()
    };

    const GRPC_ENDPOINT: &str = "https://solana-yellowstone-grpc.publicnode.com:443";
    const GRPC_AUTH_TOKEN: &str =
        "cd1c3642f88c86f9f8e7f15831faf9f067b997c6ac2b72c81d115e8d071af77a";
    let grpc = YellowstoneGrpc::new_with_config(
        GRPC_ENDPOINT.to_string(),
        Some(std::env::var("GRPC_AUTH_TOKEN").unwrap_or_else(|_| GRPC_AUTH_TOKEN.to_string())),
        config,
    )?;

    println!("✅ gRPC client created successfully");

    let protocols = vec![Protocol::PumpFun];
    println!("📊 Protocols to monitor: {:?}", protocols);

    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("🎧 Starting subscription...");
    println!("🔍 Monitoring programs for DEX events...");

    // 订阅 PumpFun Buy 和 Sell 事件（明确指定）
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpFunBuyExactSolIn,
        EventType::PumpFunCreate,
    ]);

    println!("📋 Event Filter: Buy, Sell, BuyExactSolIn, Create");

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    // 性能统计
    let event_count = Arc::new(AtomicU64::new(0));
    let total_latency = Arc::new(AtomicU64::new(0));
    let min_latency = Arc::new(AtomicU64::new(u64::MAX));
    let max_latency = Arc::new(AtomicU64::new(0));

    // 克隆用于统计报告
    let stats_count = event_count.clone();
    let stats_total = total_latency.clone();
    let stats_min = min_latency.clone();
    let stats_max = max_latency.clone();
    let queue_for_stats = queue.clone();

    // 统计报告线程
    tokio::spawn(async move {
        let mut last_count = 0u64;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let count = stats_count.load(Ordering::Relaxed);
            let total = stats_total.load(Ordering::Relaxed);
            let min = stats_min.load(Ordering::Relaxed);
            let max = stats_max.load(Ordering::Relaxed);
            let queue_len = queue_for_stats.len();

            if count > 0 {
                let avg = total / count;
                let events_per_sec = (count - last_count) as f64 / 10.0;

                println!("\n╔════════════════════════════════════════════════════╗");
                println!("║          性能统计 (10秒间隔)                       ║");
                println!("╠════════════════════════════════════════════════════╣");
                println!("║  事件总数: {:>10}                              ║", count);
                println!("║  事件速率: {:>10.1} events/sec                  ║", events_per_sec);
                println!("║  队列长度: {:>10}                              ║", queue_len);
                println!("║  平均延迟: {:>10} μs                           ║", avg);
                println!(
                    "║  最小延迟: {:>10} μs                           ║",
                    if min == u64::MAX { 0 } else { min }
                );
                println!("║  最大延迟: {:>10} μs                           ║", max);
                println!("╚════════════════════════════════════════════════════╝\n");

                if queue_len > 1000 {
                    println!("⚠️  警告: 队列堆积 ({}), 消费速度 < 生产速度", queue_len);
                }
            }

            last_count = count;
        }
    });

    // 克隆用于消费者线程
    let consumer_event_count = event_count.clone();
    let consumer_total_latency = total_latency.clone();
    let consumer_min_latency = min_latency.clone();
    let consumer_max_latency = max_latency.clone();

    // 高性能消费事件
    tokio::spawn(async move {
        let mut spin_count = 0u32;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;

                // 使用与 SDK 相同的时钟源（性能：10-50ns，比 clock_gettime 快 20-100 倍）
                let queue_recv_us = now_micros();

                // 获取元数据（所有 PumpFun 事件共享相同的元数据结构）
                let grpc_recv_us_opt = match &event {
                    DexEvent::PumpFunBuy(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpFunSell(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpFunBuyExactSolIn(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpFunTrade(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpFunCreate(e) => Some(e.metadata.grpc_recv_us),
                    _ => None,
                };

                if let Some(grpc_recv_us) = grpc_recv_us_opt {
                    let latency_us = (queue_recv_us - grpc_recv_us) as u64;

                    // 更新统计
                    consumer_event_count.fetch_add(1, Ordering::Relaxed);
                    consumer_total_latency.fetch_add(latency_us, Ordering::Relaxed);
                    update_min_max(&consumer_min_latency, &consumer_max_latency, latency_us);

                    // 打印时间指标和事件数据
                    println!("\n================================================");
                    println!("gRPC接收时间: {} μs", grpc_recv_us);
                    println!("事件接收时间: {} μs", queue_recv_us);
                    println!("延迟时间: {} μs", latency_us);
                    println!("队列长度: {}", queue.len());
                    println!("================================================");
                    println!("{:?}", event);
                    println!();
                }
            } else {
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

    println!("🛑 Press Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;
    println!("👋 Shutting down gracefully...");

    Ok(())
}
