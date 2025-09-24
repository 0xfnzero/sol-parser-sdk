use sol_parser_sdk::grpc::{
    ClientConfig, Protocol, YellowstoneGrpc, TransactionFilter, AccountFilter, EventTypeFilter, EventType,
};
use sol_parser_sdk::DexEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("Starting Sol Parser SDK Example...");
    run_example().await?;
    Ok(())
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Subscribing to Yellowstone gRPC events...");

    // Create low-latency configuration
    let mut config: ClientConfig = ClientConfig::default();
    config.enable_metrics = true; // Enable performance monitoring
    config.connection_timeout_ms = 10000;
    config.request_timeout_ms = 30000;
    config.enable_tls = true;

    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    println!("✅ gRPC client created successfully");

    // Monitor only PumpFun protocol for focused events
    let protocols = vec![
        Protocol::PumpFun,
        // 暂时只监控PumpFun，减少网络流量
        // Protocol::PumpSwap,
        // Protocol::Bonk,
        // Protocol::RaydiumCpmm,
        // Protocol::RaydiumClmm,
        // Protocol::RaydiumAmmV4,
    ];

    println!("📊 Protocols to monitor: {:?}", protocols);

    // Create filters using the new pattern
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("🎧 Starting subscription...");
    println!("🔍 Monitoring programs for DEX events...");

    // 只解析 PumpFun Trade 事件
    let event_filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);

    // 使用无锁 ArrayQueue（零拷贝模式）
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    )
    .await?;

    // 高性能消费事件（无锁队列）
    tokio::spawn(async move {
        let mut spin_count = 0u32;
        loop {
            // 使用 try-recv 非阻塞轮询，降低延迟
            if let Some(event) = queue.pop() {
                spin_count = 0; // 重置自旋计数

                // 计算从gRPC接收到队列接收的耗时
                let queue_recv_us = unsafe {
                    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                    libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
                    (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
                };

                match &event {
                    DexEvent::PumpFunTrade(e) => {
                        let latency_us = queue_recv_us - e.metadata.grpc_recv_us;
                        println!("\ngRPC接收时间: {} μs", e.metadata.grpc_recv_us);
                        println!("事件接收时间: {} μs", queue_recv_us);
                        println!("事件解析耗时: {} μs", latency_us);
                        println!("================================================");
                        println!("{:?}", event);
                    },
                    DexEvent::PumpFunCreate(e) => {
                        let latency_us = queue_recv_us - e.metadata.grpc_recv_us;
                        println!("\ngRPC接收时间: {} μs", e.metadata.grpc_recv_us);
                        println!("事件接收时间: {} μs", queue_recv_us);
                        println!("事件解析耗时: {} μs", latency_us);
                        println!("================================================");
                        println!("{:?}", event);
                    },
                    _ => {}
                }
            } else {
                // 混合策略：先自旋等待，如果长时间没数据才 yield
                spin_count += 1;
                if spin_count < 1000 {
                    // 短暂自旋，降低延迟
                    std::hint::spin_loop();
                } else {
                    // 超过阈值后 yield CPU，避免 100% 占用
                    tokio::task::yield_now().await;
                    spin_count = 0;
                }
            }
        }
    });

    // Auto-stop after 1000 seconds for testing
    let grpc_clone = grpc.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1000)).await;
        println!("⏰ Auto-stopping after timeout...");
        grpc_clone.stop().await;
    });

    println!("🛑 Press Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;
    println!("👋 Shutting down gracefully...");

    Ok(())
}