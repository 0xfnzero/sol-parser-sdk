use sol_parser_sdk::grpc::types::event_type_from_dex_event;
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, Protocol, TransactionFilter, YellowstoneGrpc,
};
use std::time::Duration;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🚀 Quick Test - Subscribing to ALL PumpFun events...");

    let config = ClientConfig { enable_metrics: true, ..Default::default() };

    // Configure the endpoint and optional authentication through the environment.
    let grpc = YellowstoneGrpc::new_with_config(
        grpc_config::endpoint()?,
        grpc_config::auth_token(),
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
    let deadline = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(deadline);

    // 简单循环，打印前 10 个事件
    loop {
        if let Some(event) = queue.pop() {
            event_count += 1;
            let event_type = event_type_from_dex_event(&event);

            println!("✅ Event #{}: {:?} (Queue: {})", event_count, event_type, queue.len());

            if event_count >= 10 {
                println!("\n🎉 Received {} events! Test successful!", event_count);
                break;
            }
        }

        tokio::select! {
            _ = &mut deadline => {
                println!("Finished after 60 seconds: {event_count} events received.");
                break;
            }
            _ = tokio::signal::ctrl_c() => break,
            _ = tokio::time::sleep(Duration::from_millis(1)) => {}
        }
    }

    grpc.stop().await;
    Ok(())
}
