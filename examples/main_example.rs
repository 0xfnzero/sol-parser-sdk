use sol_parser_sdk::grpc::{
    ClientConfig, Protocol, YellowstoneGrpc, TransactionFilter, AccountFilter,
};
use sol_parser_sdk::{DexEvent, EventListener, parse_transaction_events};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let callback = create_event_callback();

    // Monitor these DEX protocols
    let protocols = vec![
        Protocol::PumpFun,
        Protocol::PumpSwap,
        Protocol::Bonk,
        Protocol::RaydiumCpmm,
        Protocol::RaydiumClmm,
        Protocol::RaydiumAmmV4,
    ];

    println!("📊 Protocols to monitor: {:?}", protocols);

    // Create filters using the new pattern
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("🎧 Starting subscription...");
    println!("🔍 Monitoring programs for DEX events...");

    grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        None, // event_type_filter
        move |event| callback(event),
    )
    .await?;

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

fn create_event_callback() -> impl Fn(DexEvent) {
    |event: DexEvent| {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        match event {
            // ========================== PumpFun Events ==========================
            DexEvent::PumpFunTrade(e) => {
                println!(
                    "[{}] 🟢 PumpFun Trade: {} {} tokens for {} SOL | Mint: {} | User: {} | Buy: {}",
                    timestamp,
                    e.token_amount,
                    if e.is_buy { "bought" } else { "sold" },
                    e.sol_amount,
                    e.mint,
                    e.user,
                    e.is_buy
                );
            },
            DexEvent::PumpFunCreate(e) => {
                println!(
                    "[{}] 🆕 PumpFun Create: New token {} | Creator: {} | Initial: {} SOL",
                    timestamp,
                    e.mint,
                    e.creator,
                    e.virtual_sol_reserves
                );
            },

            // ========================== Bonk Events ==========================
            DexEvent::BonkTrade(e) => {
                println!(
                    "[{}] 🔴 Bonk Trade: {} -> {} | Pool: {} | User: {}",
                    timestamp,
                    e.amount_in,
                    e.amount_out,
                    e.pool_state,
                    e.user
                );
            },

            // ========================== Raydium CPMM Events ==========================
            DexEvent::RaydiumCpmmSwap(e) => {
                println!(
                    "[{}] 🔵 Raydium CPMM Swap: {} -> {} | Pool: {} | User: {} | BaseInput: {}",
                    timestamp,
                    e.amount_in,
                    e.amount_out,
                    e.pool,
                    e.user,
                    e.is_base_input
                );
            },
            DexEvent::RaydiumCpmmDeposit(e) => {
                println!(
                    "[{}] 📈 Raydium CPMM Deposit: LP {} | Tokens: {} + {} | Pool: {}",
                    timestamp,
                    e.lp_token_amount,
                    e.token0_amount,
                    e.token1_amount,
                    e.pool
                );
            },
            DexEvent::RaydiumCpmmWithdraw(e) => {
                println!(
                    "[{}] 📉 Raydium CPMM Withdraw: LP {} | Tokens: {} + {} | Pool: {}",
                    timestamp,
                    e.lp_token_amount,
                    e.token0_amount,
                    e.token1_amount,
                    e.pool
                );
            },
            DexEvent::RaydiumCpmmInitialize(e) => {
                println!(
                    "[{}] 🎯 Raydium CPMM Initialize: Pool {} | Creator: {} | Initial: {} + {}",
                    timestamp,
                    e.pool,
                    e.creator,
                    e.init_amount0,
                    e.init_amount1
                );
            },

            // ========================== Raydium CLMM Events ==========================
            DexEvent::RaydiumClmmSwap(e) => {
                println!(
                    "[{}] 🟡 Raydium CLMM Swap: {} | Pool: {} | User: {} | BaseInput: {}",
                    timestamp,
                    e.amount,
                    e.pool,
                    e.user,
                    e.is_base_input
                );
            },
            DexEvent::RaydiumClmmOpenPosition(e) => {
                println!(
                    "[{}] 📍 Raydium CLMM Open Position: Pool {} | User: {} | Ticks: {} to {} | Liquidity: {}",
                    timestamp,
                    e.pool,
                    e.user,
                    e.tick_lower_index,
                    e.tick_upper_index,
                    e.liquidity
                );
            },
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
                println!(
                    "[{}] ⬆️ Raydium CLMM Increase Liquidity: Pool {} | User: {} | Liquidity: {} | Max: {} + {}",
                    timestamp,
                    e.pool,
                    e.user,
                    e.liquidity,
                    e.amount0_max,
                    e.amount1_max
                );
            },

            // ========================== Raydium AMM V4 Events ==========================
            DexEvent::RaydiumAmmV4Swap(e) => {
                println!(
                    "[{}] 🟠 Raydium AMM V4 Swap: {} -> {} | AMM: {}",
                    timestamp,
                    e.amount_in,
                    e.amount_out,
                    e.amm
                );
            },
            DexEvent::RaydiumAmmV4Deposit(e) => {
                println!(
                    "[{}] 📊 Raydium AMM V4 Deposit: Max Coin: {} | Max PC: {} | AMM: {}",
                    timestamp,
                    e.max_coin_amount,
                    e.max_pc_amount,
                    e.amm
                );
            },

            // ========================== PumpSwap Events ==========================
            DexEvent::PumpSwapBuy(e) => {
                println!(
                    "[{}] 💰 PumpSwap Buy: {} -> {} | Pool: {} | User: {}",
                    timestamp,
                    e.sol_amount,
                    e.token_amount,
                    e.pool_id,
                    e.user
                );
            },
            DexEvent::PumpSwapSell(e) => {
                println!(
                    "[{}] 💸 PumpSwap Sell: {} -> {} | Pool: {} | User: {}",
                    timestamp,
                    e.token_amount,
                    e.sol_amount,
                    e.pool_id,
                    e.user
                );
            },

            // ========================== Other Events ==========================
            _ => {
                println!(
                    "[{}] ℹ️ Other Event: {:?}",
                    timestamp,
                    std::mem::discriminant(&event)
                );
            }
        }
    }
}

// Example of implementing custom event listener
struct CustomEventListener {
    pub event_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

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