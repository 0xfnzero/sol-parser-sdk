use solana_streamer_sdk::{
    DexEvent,
    streaming::{
        ClientConfig, YellowstoneGrpc, Protocol, StreamingEventType, EventTypeFilter,
        TransactionFilter, AccountFilter, program_ids::*,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Yellowstone gRPC Streamer...");
    test_grpc().await?;
    Ok(())
}

async fn test_grpc() -> Result<(), Box<dyn std::error::Error>> {
    println!("Subscribing to Yellowstone gRPC events...");

    // Create low-latency configuration
    let mut config: ClientConfig = ClientConfig::low_latency();
    // Enable performance monitoring, has performance overhead, disabled by default
    config.enable_metrics = true;
    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    println!("GRPC client created successfully");

    let callback = create_event_callback();

    // Will try to parse corresponding protocol events from transactions
    let protocols = vec![
        Protocol::PumpFun,
        Protocol::PumpSwap,
        Protocol::Bonk,
        Protocol::RaydiumCpmm,
        Protocol::RaydiumClmm,
        Protocol::RaydiumAmmV4,
    ];

    println!("Protocols to monitor: {:?}", protocols);

    // Filter accounts
    let account_include = vec![
        PUMPFUN_PROGRAM_ID.to_string(),        // Listen to pumpfun program ID
        PUMPSWAP_PROGRAM_ID.to_string(),       // Listen to pumpswap program ID
        BONK_PROGRAM_ID.to_string(),           // Listen to bonk program ID
        RAYDIUM_CPMM_PROGRAM_ID.to_string(),   // Listen to raydium_cpmm program ID
        RAYDIUM_CLMM_PROGRAM_ID.to_string(),   // Listen to raydium_clmm program ID
        RAYDIUM_AMM_V4_PROGRAM_ID.to_string(), // Listen to raydium_amm_v4 program ID
    ];
    let account_exclude = vec![];
    let account_required = vec![];

    // Listen to transaction data
    let transaction_filter = TransactionFilter {
        account_include: account_include.clone(),
        account_exclude,
        account_required,
    };

    // Listen to account data belonging to owner programs -> account event monitoring
    let account_filter = AccountFilter {
        account: vec![],
        owner: account_include.clone(),
        filters: vec![]
    };

    // Event filtering
    // No event filtering, includes all events
    let event_type_filter = None;
    // Only include PumpSwapBuy events and PumpSwapSell events
    // let event_type_filter = Some(EventTypeFilter::include_only(vec![StreamingEventType::PumpFunTrade]));

    println!("Starting to listen for events, press Ctrl+C to stop...");
    println!("Monitoring programs: {:?}", account_include);

    println!("Starting subscription...");

    grpc.subscribe_events_immediate(
        protocols,
        None,
        vec![transaction_filter],
        vec![account_filter],
        event_type_filter,
        None,
        callback,
    )
    .await?;

    // 支持 stop 方法，测试代码 -  异步1000秒之后停止
    let grpc_clone = grpc.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1000)).await;
        grpc_clone.stop().await;
    });

    println!("Waiting for Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}

fn create_event_callback() -> impl Fn(DexEvent) {
    |event: DexEvent| {
        println!(
            "🎉 Event received! Type: {:?}",
            get_event_type_name(&event)
        );

        match event {
            // -------------------------- block meta -----------------------
            DexEvent::BlockMeta(e) => {
                println!("BlockMetaEvent: {:?}", e.metadata.handle_us);
            },

            // -------------------------- bonk -----------------------
            DexEvent::BonkPoolCreate(e) => {
                // When using grpc, you can get block_time from each event
                println!("block_time: {:?}, block_time_ms: {:?}", e.metadata.block_time, e.metadata.block_time_ms);
                println!("BonkPoolCreateEvent: {:?}", e.base_mint_param.symbol);
            },
            DexEvent::BonkTrade(e) => {
                println!("BonkTradeEvent: pool_state={}, user={}, is_buy={}",
                    e.pool_state, e.user, e.is_buy);
            },
            DexEvent::BonkMigrateAmm(e) => {
                println!("BonkMigrateAmmEvent: old_pool={}, new_pool={}",
                    e.old_pool, e.new_pool);
            },

            // -------------------------- pumpfun -----------------------
            DexEvent::PumpFunTrade(e) => {
                println!("PumpFunTradeEvent: mint={}, user={}, is_buy={}",
                    e.mint, e.user, e.is_buy);
            },
            DexEvent::PumpFunMigrate(e) => {
                println!("PumpFunMigrateEvent: mint={}, bonding_curve={}",
                    e.mint, e.bonding_curve);
            },
            DexEvent::PumpFunCreate(e) => {
                println!("PumpFunCreateTokenEvent: mint={}, bonding_curve={}",
                    e.mint, e.bonding_curve);
            },

            // -------------------------- pumpswap -----------------------
            DexEvent::PumpSwapBuy(e) => {
                println!("Buy event: pool_id={}, user={}", e.pool_id, e.user);
            },
            DexEvent::PumpSwapSell(e) => {
                println!("Sell event: pool_id={}, user={}", e.pool_id, e.user);
            },
            DexEvent::PumpSwapCreatePool(e) => {
                println!("CreatePool event: pool_id={}, creator={}", e.pool_id, e.creator);
            },

            // -------------------------- raydium_cpmm -----------------------
            DexEvent::RaydiumCpmmSwap(e) => {
                println!("RaydiumCpmmSwapEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumCpmmDeposit(e) => {
                println!("RaydiumCpmmDepositEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumCpmmInitialize(e) => {
                println!("RaydiumCpmmInitializeEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumCpmmWithdraw(e) => {
                println!("RaydiumCpmmWithdrawEvent: pool={}", e.pool);
            },

            // -------------------------- raydium_clmm -----------------------
            DexEvent::RaydiumClmmSwap(e) => {
                println!("RaydiumClmmSwapEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmClosePosition(e) => {
                println!("RaydiumClmmClosePositionEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => {
                println!("RaydiumClmmDecreaseLiquidityEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmCreatePool(e) => {
                println!("RaydiumClmmCreatePoolEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
                println!("RaydiumClmmIncreaseLiquidityEvent: pool={}", e.pool);
            },
            DexEvent::RaydiumClmmOpenPosition(e) => {
                println!("RaydiumClmmOpenPositionEvent: pool={}", e.pool);
            },

            // -------------------------- raydium_amm_v4 -----------------------
            DexEvent::RaydiumAmmV4Swap(e) => {
                println!("RaydiumAmmV4SwapEvent: amm={}", e.amm);
            },
            DexEvent::RaydiumAmmV4Deposit(e) => {
                println!("RaydiumAmmV4DepositEvent: amm={}", e.amm);
            },
            DexEvent::RaydiumAmmV4Initialize2(e) => {
                println!("RaydiumAmmV4Initialize2Event: amm={}", e.amm);
            },
            DexEvent::RaydiumAmmV4Withdraw(e) => {
                println!("RaydiumAmmV4WithdrawEvent: amm={}", e.amm);
            },
            DexEvent::RaydiumAmmV4WithdrawPnl(e) => {
                println!("RaydiumAmmV4WithdrawPnlEvent: amm={}", e.amm);
            },

            // -------------------------- account -----------------------
            DexEvent::TokenAccount(e) => {
                println!("TokenAccountEvent: mint={}, owner={}", e.mint, e.owner);
            },
            DexEvent::NonceAccount(e) => {
                println!("NonceAccountEvent: pubkey={}, authority={}", e.pubkey, e.authority);
            },
            DexEvent::TokenInfo(e) => {
                println!("TokenInfoEvent: mint={}", e.mint);
            },

            // -------------------------- other events -----------------------
            _ => {
                println!("Other event type received: {:?}", get_event_type_name(&event));
            },
        }
    }
}

/// 获取事件类型名称
fn get_event_type_name(event: &DexEvent) -> &'static str {
    match event {
        DexEvent::BlockMeta(_) => "BlockMeta",
        DexEvent::BonkPoolCreate(_) => "BonkPoolCreate",
        DexEvent::BonkTrade(_) => "BonkTrade",
        DexEvent::BonkMigrateAmm(_) => "BonkMigrateAmm",
        DexEvent::PumpFunTrade(_) => "PumpFunTrade",
        DexEvent::PumpFunCreate(_) => "PumpFunCreate",
        DexEvent::PumpFunComplete(_) => "PumpFunComplete",
        DexEvent::PumpFunMigrate(_) => "PumpFunMigrate",
        DexEvent::PumpSwapBuy(_) => "PumpSwapBuy",
        DexEvent::PumpSwapSell(_) => "PumpSwapSell",
        DexEvent::PumpSwapCreatePool(_) => "PumpSwapCreatePool",
        DexEvent::RaydiumCpmmSwap(_) => "RaydiumCpmmSwap",
        DexEvent::RaydiumCpmmDeposit(_) => "RaydiumCpmmDeposit",
        DexEvent::RaydiumCpmmWithdraw(_) => "RaydiumCpmmWithdraw",
        DexEvent::RaydiumCpmmInitialize(_) => "RaydiumCpmmInitialize",
        DexEvent::RaydiumClmmSwap(_) => "RaydiumClmmSwap",
        DexEvent::RaydiumClmmCreatePool(_) => "RaydiumClmmCreatePool",
        DexEvent::RaydiumClmmOpenPosition(_) => "RaydiumClmmOpenPosition",
        DexEvent::RaydiumClmmClosePosition(_) => "RaydiumClmmClosePosition",
        DexEvent::RaydiumClmmIncreaseLiquidity(_) => "RaydiumClmmIncreaseLiquidity",
        DexEvent::RaydiumClmmDecreaseLiquidity(_) => "RaydiumClmmDecreaseLiquidity",
        DexEvent::RaydiumAmmV4Swap(_) => "RaydiumAmmV4Swap",
        DexEvent::RaydiumAmmV4Deposit(_) => "RaydiumAmmV4Deposit",
        DexEvent::RaydiumAmmV4Withdraw(_) => "RaydiumAmmV4Withdraw",
        DexEvent::RaydiumAmmV4Initialize2(_) => "RaydiumAmmV4Initialize2",
        DexEvent::RaydiumAmmV4WithdrawPnl(_) => "RaydiumAmmV4WithdrawPnl",
        DexEvent::TokenAccount(_) => "TokenAccount",
        DexEvent::NonceAccount(_) => "NonceAccount",
        DexEvent::TokenInfo(_) => "TokenInfo",
        _ => "Unknown",
    }
}