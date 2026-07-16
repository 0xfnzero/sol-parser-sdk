//! ShredStream PumpFun 代币创建和首次买入监听示例
//!
//! 展示如何使用 ShredStream 监听 PumpFun 的：
//! - 代币创建事件 (CREATE / CREATE_V2)
//! - 开发者首次买入事件 (is_created_buy = true)
//!
//! ## 运行方式
//! cargo run --example shredstream_pump_snipe -- --endpoint http://127.0.0.1:10800

use sol_parser_sdk::core::now_micros;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::shredstream::{ShredStreamClient, ShredStreamConfig};
use sol_parser_sdk::DexEvent;
use std::collections::HashSet;
use std::env;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 ShredStream PumpFun Sniper Monitor");
    println!("======================================\n");

    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    let mut endpoint = "http://127.0.0.1:10800".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--endpoint" | "-e" => {
                if i + 1 < args.len() {
                    endpoint = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --endpoint requires a value");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --example shredstream_pump_snipe -- [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -e, --endpoint <URL>  ShredStream endpoint URL (default: http://127.0.0.1:10800)");
                println!("  -h, --help            Print this help message");
                println!();
                println!("Examples:");
                println!("  cargo run --example shredstream_pump_snipe");
                println!("  cargo run --example shredstream_pump_snipe -- --endpoint http://192.168.1.100:10800");
                println!();
                println!(
                    "This example uses filter: PumpFunCreate + PumpFunCreateV2 + PumpFunTrade"
                );
                println!("For more filter presets, run: cargo run --example shredstream_example -- --help");
                std::process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }

    run_snipe_monitor(&endpoint).await
}

async fn run_snipe_monitor(endpoint: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 配置
    let config = ShredStreamConfig {
        connection_timeout_ms: 5000,
        request_timeout_ms: 30000,
        max_decoding_message_size: 1024 * 1024 * 1024,
        reconnect_delay_ms: 1000,
        max_reconnect_attempts: 0,
    };

    println!("📋 Configuration:");
    println!("   Endpoint: {}", endpoint);
    println!("   Target: PumpFun CREATE/CREATE_V2 + PumpFunTrade events");
    println!();

    // 创建客户端
    let client = ShredStreamClient::new_with_config(endpoint, config).await?;

    println!("✅ ShredStream client connected");
    println!("🎧 Listening for PumpFun token creation and first buy...\n");

    // ShredStream 使用本地热路径事件过滤。这个狙击示例需要 create + trade：
    // - PumpFunCreate / PumpFunCreateV2 用来记录新 mint
    // - PumpFunTrade 会把买/卖/精确 SOL 买入统一回调为 DexEvent::PumpFunTrade
    // 其它可配置 filter preset 见 `shredstream_example --help`。
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunCreate,
        EventType::PumpFunCreateV2,
        EventType::PumpFunTrade,
    ]);
    let queue = client.subscribe_with_filter(Some(event_filter)).await?;

    // 用于追踪已创建的代币（检测同交易买入）
    let created_tokens: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // 消费事件
    let consumer_queue = queue.clone();
    let consumer_created = created_tokens.clone();

    tokio::spawn(async move {
        let mut spin_count = 0u32;

        loop {
            if let Some(event) = consumer_queue.pop() {
                spin_count = 0;

                let recv_us = now_micros();

                match &event {
                    // ========== 代币创建事件 ==========
                    DexEvent::PumpFunCreate(create_event) => {
                        let mint = create_event.mint.to_string();
                        let creator = create_event.creator.to_string();
                        let user = create_event.user.to_string();
                        let latency_us = recv_us - create_event.metadata.grpc_recv_us;

                        // 记录已创建的代币
                        consumer_created.lock().unwrap().insert(mint.clone());

                        println!("\n");
                        println!(
                            "╔══════════════════════════════════════════════════════════════════╗"
                        );
                        println!(
                            "║                   🎯 PUMPFUN 代币创建 (Legacy)                    ║"
                        );
                        println!(
                            "╠══════════════════════════════════════════════════════════════════╣"
                        );
                        println!("║  代币名称: {:<50} ║", truncate(&create_event.name, 50));
                        println!("║  代币符号: {:<50} ║", truncate(&create_event.symbol, 50));
                        println!("║  Mint:     {:<50} ║", mint);
                        println!("║  创建者:   {:<50} ║", creator);
                        println!("║  用户:     {:<50} ║", user);
                        println!("║  Bonding:  {:<50} ║", create_event.bonding_curve.to_string());
                        println!("║  Slot:     {:<50} ║", create_event.metadata.slot);
                        println!(
                            "║  Sig:      {:<50} ║",
                            truncate(&create_event.metadata.signature.to_string(), 50)
                        );
                        println!("╠────────────────────────────────────────────────────────────────────╣");
                        println!("║  📊 延迟:  {} μs", latency_us);
                        println!(
                            "╚══════════════════════════════════════════════════════════════════╝"
                        );
                    }

                    // ========== 代币创建 V2 事件 ==========
                    DexEvent::PumpFunCreateV2(create_v2_event) => {
                        let mint = create_v2_event.mint.to_string();
                        let creator = create_v2_event.creator.to_string();
                        let user = create_v2_event.user.to_string();
                        let latency_us = recv_us - create_v2_event.metadata.grpc_recv_us;

                        // 记录已创建的代币
                        consumer_created.lock().unwrap().insert(mint.clone());

                        println!("\n");
                        println!(
                            "╔══════════════════════════════════════════════════════════════════╗"
                        );
                        println!(
                            "║                 🎯 PUMPFUN 代币创建 V2 (SPL-22)                   ║"
                        );
                        println!(
                            "╠══════════════════════════════════════════════════════════════════╣"
                        );
                        println!("║  代币名称: {:<50} ║", truncate(&create_v2_event.name, 50));
                        println!("║  代币符号: {:<50} ║", truncate(&create_v2_event.symbol, 50));
                        println!("║  Mint:     {:<50} ║", mint);
                        println!("║  创建者:   {:<50} ║", creator);
                        println!("║  用户:     {:<50} ║", user);
                        println!(
                            "║  Bonding:  {:<50} ║",
                            create_v2_event.bonding_curve.to_string()
                        );
                        println!("║  Slot:     {:<50} ║", create_v2_event.metadata.slot);
                        println!(
                            "║  Sig:      {:<50} ║",
                            truncate(&create_v2_event.metadata.signature.to_string(), 50)
                        );
                        println!("╠────────────────────────────────────────────────────────────────────╣");
                        println!("║  📊 延迟:  {} μs", latency_us);
                        println!(
                            "╚══════════════════════════════════════════════════════════════════╝"
                        );
                    }

                    // ========== 交易事件（检测 is_created_buy） ==========
                    DexEvent::PumpFunTrade(trade_event) => {
                        // 只关注买入
                        if trade_event.is_buy {
                            let mint = trade_event.mint.to_string();
                            let is_created_buy = trade_event.is_created_buy;

                            // 检查是否是刚刚创建的代币的首次买入
                            let is_new_token = consumer_created.lock().unwrap().contains(&mint);

                            if is_created_buy {
                                let ix_type = match trade_event.ix_name.as_str() {
                                    "buy" => "BUY",
                                    "buy_exact_sol_in" => "BUY_EXACT_SOL_IN",
                                    _ => "BUY",
                                };
                                let latency_us = recv_us - trade_event.metadata.grpc_recv_us;
                                println!("\n");
                                println!("╔══════════════════════════════════════════════════════════════════╗");
                                println!("║              🚀 PUMPFUN {} 开发者首次买入 (is_created_buy)        ║", ix_type);
                                println!("╠══════════════════════════════════════════════════════════════════╣");
                                println!("║  Mint:        {:<47} ║", mint);
                                println!("║  买入者:      {:<47} ║", trade_event.user.to_string());
                                println!(
                                    "║  Bonding:     {:<47} ║",
                                    trade_event.bonding_curve.to_string()
                                );
                                println!(
                                    "║  SOL 数量:    {:<47} ║",
                                    format!("{}", trade_event.sol_amount)
                                );
                                println!(
                                    "║  Token 数量:  {:<47} ║",
                                    format!("{}", trade_event.token_amount)
                                );
                                println!("║  Slot:        {:<47} ║", trade_event.metadata.slot);
                                println!(
                                    "║  新创建代币:  {:<47} ║",
                                    if is_new_token { "✅ 是" } else { "❌ 否" }
                                );
                                println!(
                                    "║  Sig:         {:<47} ║",
                                    truncate(&trade_event.metadata.signature.to_string(), 47)
                                );
                                println!("╠────────────────────────────────────────────────────────────────────╣");
                                println!("║  📊 延迟:     {} μs", latency_us);
                                println!("╚══════════════════════════════════════════════════════════════════╝");
                            }
                        }
                    }

                    _ => {}
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

    // 统计报告
    let stats_queue = queue.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let queue_len = stats_queue.len();
            println!("\n[统计] 队列长度: {} | 等待事件...", queue_len);
        }
    });

    println!("🛑 Press Ctrl+C to stop...\n");
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down gracefully...");

    client.stop().await;
    Ok(())
}

// 截断字符串到指定长度
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
