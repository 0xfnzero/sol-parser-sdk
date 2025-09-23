use std::time::Instant;
use sol_parser_sdk::{DexEvent, parse_transaction_events_streaming, StreamingEventListener};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 示例：展示流式回调解析器的使用
fn main() {
    println!("🚀 Sol Parser SDK 流式回调示例");

    // 模拟交易数据
    let instruction_data = vec![1, 2, 3, 4]; // 示例指令数据
    let accounts = vec![Pubkey::default(); 5]; // 示例账户
    let logs = vec![
        "Program data: aabbccdd".to_string(),
        "Another log line".to_string(),
    ]; // 示例日志
    let signature = Signature::default();
    let slot = 123456789;
    let block_time = Some(1640995200); // 示例时间戳
    let program_id = Pubkey::default();

    println!("\n📊 对比批量回调 vs 流式回调:");

    // 1. 传统批量回调方式
    println!("\n--- 批量回调方式 ---");
    let start = Instant::now();
    let events = sol_parser_sdk::parse_transaction_events(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        &program_id,
    );
    let batch_duration = start.elapsed();

    println!("解析耗时: {:?}", batch_duration);
    println!("解析出的事件数量: {}", events.len());

    for (i, event) in events.iter().enumerate() {
        println!("  事件 {}: {:?}", i + 1, get_event_type(event));
    }

    // 2. 新的流式回调方式
    println!("\n--- 流式回调方式 ---");
    let start = Instant::now();
    let mut event_count = 0;

    parse_transaction_events_streaming(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        &program_id,
        |event| {
            event_count += 1;
            let current_time = start.elapsed().as_micros();
            println!("⚡ 事件 {} 立即回调 ({}μs): {:?}", event_count, current_time, get_event_type(&event));

            // 模拟一些处理时间（比如发送到消息队列、数据库等）
            std::thread::sleep(std::time::Duration::from_millis(1));

            println!("   ✅ 事件 {} 处理完成", event_count);
            // 这里可以立即进行事件处理，比如：
            // - 发送到消息队列
            // - 更新数据库
            // - 发送通知
            // - 计算实时指标
        }
    );

    let streaming_duration = start.elapsed();
    println!("总流式解析耗时: {:?}", streaming_duration);
    println!("流式处理的事件数量: {}", event_count);

    // 3. 使用监听器模式
    println!("\n--- 监听器模式 ---");
    let mut listener = ExampleStreamingListener::new();

    sol_parser_sdk::parse_transaction_with_streaming_listener(
        &instruction_data,
        &accounts,
        &logs,
        signature,
        slot,
        block_time,
        &program_id,
        &mut listener,
    );

    println!("监听器处理的事件数量: {}", listener.event_count);

    println!("\n✅ 流式回调的优势:");
    println!("  📍 实时性: 每个事件都能立即被处理");
    println!("  🚀 低延迟: 不需要等待所有事件解析完成");
    println!("  💾 内存效率: 避免了大量事件的批量缓存");
    println!("  ⚡ 响应速度: 适合高频交易监控和实时分析");
}

/// 获取事件类型的简化描述
fn get_event_type(event: &DexEvent) -> &'static str {
    match event {
        DexEvent::PumpFunTrade(_) => "PumpFun交易",
        DexEvent::PumpFunCreate(_) => "PumpFun创建",
        DexEvent::PumpFunComplete(_) => "PumpFun完成",
        DexEvent::PumpFunMigrate(_) => "PumpFun迁移",
        DexEvent::BonkTrade(_) => "Bonk交易",
        DexEvent::BonkPoolCreate(_) => "Bonk池创建",
        DexEvent::RaydiumClmmSwap(_) => "Raydium CLMM交换",
        DexEvent::RaydiumCpmmSwap(_) => "Raydium CPMM交换",
        DexEvent::RaydiumAmmV4Swap(_) => "Raydium AMM V4交换",
        DexEvent::OrcaWhirlpoolSwap(_) => "Orca Whirlpool交换",
        DexEvent::MeteoraPoolsSwap(_) => "Meteora Pools交换",
        DexEvent::MeteoraDammV2Swap(_) => "Meteora DAMM V2交换",
        _ => "其他事件",
    }
}

/// 示例流式事件监听器
struct ExampleStreamingListener {
    event_count: usize,
    start_time: Instant,
}

impl ExampleStreamingListener {
    fn new() -> Self {
        Self {
            event_count: 0,
            start_time: Instant::now(),
        }
    }
}

impl StreamingEventListener for ExampleStreamingListener {
    fn on_dex_event_streaming(&mut self, event: DexEvent) {
        self.event_count += 1;
        let elapsed = self.start_time.elapsed();

        println!(
            "🎯 监听器接收事件 {} ({}μs): {}",
            self.event_count,
            elapsed.as_micros(),
            get_event_type(&event)
        );

        // 这里可以实现具体的业务逻辑：
        match event {
            DexEvent::PumpFunTrade(trade) => {
                println!("   💰 PumpFun交易: {} SOL", trade.sol_amount as f64 / 1e9);
            },
            DexEvent::RaydiumClmmSwap(swap) => {
                println!("   🔄 Raydium CLMM交换: 池 {}", swap.pool);
            },
            _ => {
                println!("   📝 其他类型事件");
            }
        }
    }
}