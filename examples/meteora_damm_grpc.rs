//! Meteora DAMM gRPC Streaming Example
//!
//! Demonstrates how to:
//! - Subscribe to Meteora DAMM protocol events via gRPC
//! - Filter specific event types: Swap, Swap2, AddLiquidity, RemoveLiquidity
//! - Display event details with latency metrics
//!
//! Usage:
//! ```bash
//! cargo run --example meteora_damm_grpc --release
//! ```

use sol_parser_sdk::core::now_micros;
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol,
    TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[path = "support/grpc_config.rs"]
mod grpc_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🚀 Meteora DAMM gRPC Streaming Example");
    println!("========================================\n");

    run_example().await?;
    Ok(())
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create ultra-low latency configuration
    // NOTE: Use Unordered mode for lowest latency (10-20μs)
    //       MicroBatch mode has no periodic flush, events may be delayed until next batch
    let config = ClientConfig {
        enable_metrics: true,
        connection_timeout_ms: 10000,
        request_timeout_ms: 30000,
        enable_tls: true,
        order_mode: OrderMode::Unordered, // Ultra-low latency mode
        ..Default::default()
    };

    println!("📋 Configuration:");
    println!("   Order Mode: {:?} (ultra-low latency)", config.order_mode);
    println!();

    // publicnode gRPC 需要 token；endpoint/token 可由环境变量覆盖
    let grpc_endpoint = grpc_config::endpoint()?;
    let grpc_token = grpc_config::auth_token();

    let grpc = YellowstoneGrpc::new_with_config(grpc_endpoint.clone(), grpc_token, config)?;

    println!("✅ gRPC client created (parser pre-warmed)");
    println!("Using configured gRPC endpoint");

    // Monitor only Meteora DAMM V2 protocol
    let protocols = vec![Protocol::MeteoraDammV2];
    println!("📊 Protocols: {:?}", protocols);

    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    // ========== Event Type Filter Examples ==========
    //
    // Example 1: Subscribe to Swap events only (V2)
    // let event_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2Swap]);
    //
    // Example 2: Subscribe to liquidity events only (V2)
    // let event_filter = EventTypeFilter::include_only(vec![
    //     EventType::MeteoraDammV2AddLiquidity,
    //     EventType::MeteoraDammV2RemoveLiquidity,
    // ]);

    // Default: Subscribe to all Meteora DAMM V2 event types
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::MeteoraDammV2Swap,
        EventType::MeteoraDammV2AddLiquidity,
        EventType::MeteoraDammV2RemoveLiquidity,
        EventType::MeteoraDammV2CreatePosition,
        EventType::MeteoraDammV2ClosePosition,
    ]);

    println!("🎯 Event Filter: Swap, Swap2, AddLiquidity, RemoveLiquidity");
    println!("🎧 Starting subscription...\n");

    let queue = grpc
        .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    // Statistics
    let mut event_count = 0u64;
    let mut swap_count = 0u64;
    let swap2_count = 0u64; // reserved for future V1/V2 split
    let mut add_liquidity_count = 0u64;
    let mut remove_liquidity_count = 0u64;

    // High-performance event consumer
    tokio::spawn(async move {
        let mut spin_count = 0u32;

        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                event_count += 1;

                // Get current time (microseconds) - use same clock source as events
                let now_us = now_micros();

                match &event {
                    DexEvent::MeteoraDammV2Swap(e) => {
                        swap_count += 1;
                        let latency_us = now_us - e.metadata.grpc_recv_us;

                        println!("┌─────────────────────────────────────────────────────────────");
                        println!("│ 🔄 Meteora DAMM SWAP (V2) #{}", event_count);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Signature  : {}", e.metadata.signature);
                        println!(
                            "│ Slot       : {} | TxIndex: {}",
                            e.metadata.slot, e.metadata.tx_index
                        );
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Pool       : {}", e.pool);
                        println!(
                            "│ Direction  : {}",
                            if e.trade_direction == 0 { "A→B" } else { "B→A" }
                        );
                        println!("│ Amount In  : {}", e.amount_in);
                        println!("│ Min Out    : {}", e.minimum_amount_out);
                        println!("│ Actual Out : {}", e.output_amount);
                        println!("│ Actual In  : {}", e.actual_amount_in);
                        println!("│ LP Fee     : {}", e.lp_fee);
                        println!("│ Protocol   : {}", e.protocol_fee);
                        println!(
                            "│ Referral   : {} (has_referral: {})",
                            e.referral_fee, e.has_referral
                        );
                        println!("│ Sqrt Price : {}", e.next_sqrt_price);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ 📊 Latency : {} μs", latency_us);
                        println!(
                            "│ 📊 Stats   : Swap={} Swap2={} AddLiq={} RemLiq={}",
                            swap_count, swap2_count, add_liquidity_count, remove_liquidity_count
                        );
                        println!(
                            "└─────────────────────────────────────────────────────────────\n"
                        );
                    }

                    DexEvent::MeteoraDammV2AddLiquidity(e) => {
                        add_liquidity_count += 1;
                        let latency_us = now_us - e.metadata.grpc_recv_us;

                        println!("┌─────────────────────────────────────────────────────────────");
                        println!("│ ➕ Meteora DAMM ADD LIQUIDITY (V2) #{}", event_count);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Signature  : {}", e.metadata.signature);
                        println!(
                            "│ Slot       : {} | TxIndex: {}",
                            e.metadata.slot, e.metadata.tx_index
                        );
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Pool       : {}", e.pool);
                        println!("│ Position   : {}", e.position);
                        println!("│ Token A In : {}", e.token_a_amount);
                        println!("│ Token B In : {}", e.token_b_amount);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ 📊 Latency : {} μs", latency_us);
                        println!(
                            "│ 📊 Stats   : Swap={} Swap2={} AddLiq={} RemLiq={}",
                            swap_count, swap2_count, add_liquidity_count, remove_liquidity_count
                        );
                        println!(
                            "└─────────────────────────────────────────────────────────────\n"
                        );
                    }

                    DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
                        remove_liquidity_count += 1;
                        let latency_us = now_us - e.metadata.grpc_recv_us;

                        println!("┌─────────────────────────────────────────────────────────────");
                        println!("│ ➖ Meteora DAMM REMOVE LIQUIDITY (V2) #{}", event_count);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Signature  : {}", e.metadata.signature);
                        println!(
                            "│ Slot       : {} | TxIndex: {}",
                            e.metadata.slot, e.metadata.tx_index
                        );
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ Pool       : {}", e.pool);
                        println!("│ Position   : {}", e.position);
                        println!("│ Token A Out: {}", e.token_a_amount);
                        println!("│ Token B Out: {}", e.token_b_amount);
                        println!("├─────────────────────────────────────────────────────────────");
                        println!("│ 📊 Latency : {} μs", latency_us);
                        println!(
                            "│ 📊 Stats   : Swap={} Swap2={} AddLiq={} RemLiq={}",
                            swap_count, swap2_count, add_liquidity_count, remove_liquidity_count
                        );
                        println!(
                            "└─────────────────────────────────────────────────────────────\n"
                        );
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

    // Auto-stop timer
    let grpc_clone = grpc.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        println!("⏰ Auto-stopping after 10 minutes...");
        grpc_clone.stop().await;
    });

    println!("🛑 Press Ctrl+C to stop...\n");
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down gracefully...");

    Ok(())
}
