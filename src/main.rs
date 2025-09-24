use sol_parser_sdk::grpc::{
    ClientConfig, Protocol, YellowstoneGrpc, TransactionFilter, AccountFilter, EventTypeFilter, EventType,
};
use sol_parser_sdk::{DexEvent, EventListener, parse_transaction_events};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("Starting Sol Parser SDK Example...");
    test_grpc_streaming().await?;
    Ok(())
}

async fn test_grpc_streaming() -> Result<(), Box<dyn std::error::Error>> {
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
    let queue = grpc.subscribe_dex_events_zero_copy(
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
                        println!(" ");
                        println!("事件解析耗时: {}μs", latency_us);
                        println!("================================================");
                        println!("{:?}", e);
                    },
                    DexEvent::PumpFunCreate(e) => {
                        let latency_us = queue_recv_us - e.metadata.grpc_recv_us;
                        println!(" ");
                        println!("事件解析耗时: {}μs", latency_us);
                        println!("================================================");
                        println!("{:?}", e);
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

// Example of implementing custom event listener
#[allow(dead_code)]
struct CustomEventListener {
    pub event_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[allow(dead_code)]
impl EventListener for CustomEventListener {
    fn on_dex_event(&self, event: &DexEvent) {
        self.event_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Custom processing logic
        match event {
            DexEvent::PumpFunTrade(trade) if trade.sol_amount > 1_000_000 => {
                println!("🔥 Large PumpFun trade detected: {} SOL!", trade.sol_amount);
            },
            DexEvent::RaydiumCpmmSwap(swap) if swap.amount_in > 10_000_000 => {
                println!("💎 Large Raydium CPMM swap detected: {} tokens!", swap.amount_in);
            },
            _ => {} // Ignore other events
        }
    }
}

// Example of using the parser directly (without gRPC streaming)
#[allow(dead_code)]
fn example_direct_parsing() {
    use solana_sdk::{pubkey::Pubkey, signature::Signature};
    use std::str::FromStr;

    // Example transaction data (would come from actual Solana transactions)
    let instruction_data = vec![/* instruction bytes */];
    let accounts = vec![
        Pubkey::from_str("11111111111111111111111111111111").unwrap(),
        // ... other account pubkeys
    ];
    let logs = vec![
        "Program log: Instruction: Swap".to_string(),
        // ... other log lines
    ];
    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(1640995200);
    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();

    // Parse events from transaction data
    let events = parse_transaction_events(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        &program_id,
    );

    println!("Parsed {} events from transaction", events.len());
    for event in events {
        println!("Event: {:?}", event);
    }
}