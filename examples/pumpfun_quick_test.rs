use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, Protocol, TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🚀 Quick Test - Subscribing to ALL PumpFun events...");

    let config = ClientConfig { enable_metrics: true, ..Default::default() };

    // publicnode gRPC 需要 token，可从环境变量 GRPC_AUTH_TOKEN 覆盖
    const GRPC_ENDPOINT: &str = "https://solana-yellowstone-grpc.publicnode.com:443";
    const GRPC_AUTH_TOKEN: &str =
        "cd1c3642f88c86f9f8e7f15831faf9f067b997c6ac2b72c81d115e8d071af77a";
    let grpc = YellowstoneGrpc::new_with_config(
        GRPC_ENDPOINT.to_string(),
        Some(std::env::var("GRPC_AUTH_TOKEN").unwrap_or_else(|_| GRPC_AUTH_TOKEN.to_string())),
        config,
    )?;

    let protocols = vec![Protocol::PumpFun];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("✅ Subscribing... (no event filter - will show ALL events)");

    // 无过滤器 - 订阅所有事件
    let queue = grpc
        .subscribe_dex_events(
            vec![transaction_filter],
            vec![account_filter],
            None, // 无过滤 - 所有事件都会显示
        )
        .await?;

    println!("🎧 Listening for events... (waiting up to 60 seconds)\n");

    let mut event_count = 0;
    let start = std::time::Instant::now();

    // 简单循环，打印前 10 个事件
    loop {
        if let Some(event) = queue.pop() {
            event_count += 1;
            let event_type = match &event {
                DexEvent::PumpFunCreate(_) => "PumpFunCreate",
                DexEvent::PumpFunTrade(_) => "PumpFunTrade",
                DexEvent::PumpFunBuy(_) => "PumpFunBuy",
                DexEvent::PumpFunSell(_) => "PumpFunSell",
                DexEvent::PumpFunMigrate(_) => "PumpFunMigrate",
                _ => "Other",
            };

            println!("✅ Event #{}: {} (Queue: {})", event_count, event_type, queue.len());

            if event_count >= 10 {
                println!("\n🎉 Received {} events! Test successful!", event_count);
                break;
            }
        } else {
            tokio::task::yield_now().await;
        }

        // 60 秒超时
        if start.elapsed().as_secs() > 60 {
            if event_count == 0 {
                println!("⏰ Timeout: No events received in 60 seconds.");
                println!("   This might indicate:");
                println!("   - Network connectivity issues");
                println!("   - gRPC endpoint is down");
                println!("   - Very low market activity (rare)");
            } else {
                println!("\n✅ Received {} events in 60 seconds", event_count);
            }
            break;
        }
    }

    Ok(())
}
