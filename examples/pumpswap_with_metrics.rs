//! PumpSwap 事件解析 + 详细性能指标
//!
//! 与 pumpfun_with_metrics 对应，订阅 PumpSwap 协议并打印：
//! - 每个事件的明细数据（Buy/Sell/CreatePool/AddLiquidity/RemoveLiquidity）
//! - gRPC 接收时间、队列接收时间、端到端延迟（μs）
//! - 每 10 秒汇总：事件总数、速率、队列长度、平均/最小/最大延迟
//!
//! 运行: `cargo run --example pumpswap_with_metrics --release`

use sol_parser_sdk::core::now_micros;
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, Protocol, TransactionFilter,
    YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn update_min_max(min: &Arc<AtomicU64>, max: &Arc<AtomicU64>, value: u64) {
    let mut current_min = min.load(Ordering::Relaxed);
    while value < current_min {
        match min.compare_exchange(current_min, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current_min = x,
        }
    }
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

    println!("PumpSwap event parsing with detailed performance metrics");
    run_example().await?;
    Ok(())
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Subscribing to Yellowstone gRPC (PumpSwap)...");

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

    let protocols = vec![Protocol::PumpSwap];
    println!("📊 Protocols to monitor: {:?}", protocols);

    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("🎧 Starting subscription...");
    println!("🔍 Monitoring PumpSwap programs for DEX events...");

    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpSwapBuy,
        EventType::PumpSwapSell,
        EventType::PumpSwapCreatePool,
        EventType::PumpSwapLiquidityAdded,
        EventType::PumpSwapLiquidityRemoved,
    ]);

    println!("📋 Event Filter: Buy, Sell, CreatePool, LiquidityAdded, LiquidityRemoved");

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    let event_count = Arc::new(AtomicU64::new(0));
    let total_latency = Arc::new(AtomicU64::new(0));
    let min_latency = Arc::new(AtomicU64::new(u64::MAX));
    let max_latency = Arc::new(AtomicU64::new(0));

    let stats_count = event_count.clone();
    let stats_total = total_latency.clone();
    let stats_min = min_latency.clone();
    let stats_max = max_latency.clone();
    let queue_for_stats = queue.clone();

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
                println!("║      PumpSwap 性能统计 (10秒间隔)                  ║");
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

    let consumer_event_count = event_count.clone();
    let consumer_total_latency = total_latency.clone();
    let consumer_min_latency = min_latency.clone();
    let consumer_max_latency = max_latency.clone();

    tokio::spawn(async move {
        let mut spin_count = 0u32;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;

                let queue_recv_us = now_micros();

                let grpc_recv_us_opt = match &event {
                    DexEvent::PumpSwapBuy(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpSwapSell(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpSwapCreatePool(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpSwapLiquidityAdded(e) => Some(e.metadata.grpc_recv_us),
                    DexEvent::PumpSwapLiquidityRemoved(e) => Some(e.metadata.grpc_recv_us),
                    _ => None,
                };

                if let Some(grpc_recv_us) = grpc_recv_us_opt {
                    let latency_us = (queue_recv_us - grpc_recv_us) as u64;

                    consumer_event_count.fetch_add(1, Ordering::Relaxed);
                    consumer_total_latency.fetch_add(latency_us, Ordering::Relaxed);
                    update_min_max(&consumer_min_latency, &consumer_max_latency, latency_us);

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
