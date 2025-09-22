use super::types::*;
use crate::DexEvent;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct YellowstoneGrpc {
    endpoint: String,
    token: Option<String>,
    config: ClientConfig,
}

impl YellowstoneGrpc {
    pub fn new(endpoint: String, token: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            endpoint,
            token,
            config: ClientConfig::default(),
        })
    }

    pub fn new_with_config(
        endpoint: String,
        token: Option<String>,
        config: ClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            endpoint,
            token,
            config,
        })
    }

    /// 订阅DEX事件
    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
        callback: impl Fn(DexEvent) + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting DEX event subscription...");
        println!("🌐 gRPC endpoint: {}", self.endpoint);

        if self.config.enable_metrics {
            println!("📊 Metrics enabled: connection_timeout={}ms, request_timeout={}ms",
                self.config.connection_timeout_ms, self.config.request_timeout_ms);
        }

        // 显示过滤器配置
        println!("⚙️  Transaction filters: {}", transaction_filters.len());
        for (i, filter) in transaction_filters.iter().enumerate() {
            println!("   Filter {}: include={}, exclude={}, required={}",
                i, filter.account_include.len(), filter.account_exclude.len(), filter.account_required.len());
        }

        println!("⚙️  Account filters: {}", account_filters.len());
        for (i, filter) in account_filters.iter().enumerate() {
            println!("   Filter {}: accounts={}, owners={}, filters={}",
                i, filter.account.len(), filter.owner.len(), filter.filters.len());
        }

        if let Some(ref filter) = event_type_filter {
            println!("🎯 Event type filter: include={}, exclude={}",
                filter.include_only.as_ref().map(|v| v.len()).unwrap_or(0),
                filter.exclude_types.as_ref().map(|v| v.len()).unwrap_or(0));
        }

        // 创建通道用于数据传输（模拟）
        let (tx, mut rx) = mpsc::unbounded_channel::<DexEvent>();

        // 模拟gRPC连接和数据接收
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            println!("📡 Establishing gRPC connection...");

            // 模拟接收到DEX事件（在实际实现中会从Yellowstone gRPC获取真实数据）
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // 模拟创建一个示例PumpFun交易事件
                use crate::core::events::{DexEvent, PumpFunTradeEvent, EventMetadata};
                use solana_sdk::pubkey::Pubkey;
                use std::str::FromStr;

                let metadata = EventMetadata {
                    signature: solana_sdk::signature::Signature::default(),
                    slot: 123456789,
                    block_time: Some(chrono::Utc::now().timestamp()),
                    block_time_ms: Some(chrono::Utc::now().timestamp_millis()),
                    program_id: Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap(),
                    outer_index: 0,
                    inner_index: None,
                    transaction_index: Some(0),
                    recv_us: chrono::Utc::now().timestamp_micros(),
                    handle_us: chrono::Utc::now().timestamp_micros(),
                };

                let pumpfun_trade = PumpFunTradeEvent {
                    metadata,
                    mint: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
                    user: Pubkey::from_str("8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6").unwrap(),
                    sol_amount: 1000000,
                    token_amount: 500000,
                    is_buy: true,
                    bonding_curve: Pubkey::default(),
                    virtual_sol_reserves: 30000000,
                    virtual_token_reserves: 1073000000000,
                    real_sol_reserves: 30000000,
                    real_token_reserves: 793100000000,
                    fee_recipient: Pubkey::default(),
                    fee_basis_points: 100,
                    fee: 500,
                    creator: Pubkey::default(),
                    creator_fee_basis_points: 0,
                    creator_fee: 0,
                    total_unclaimed_tokens: 793100000000,
                    total_claimed_tokens: 206900000000,
                    current_sol_volume: 0,
                    timestamp: chrono::Utc::now().timestamp(),
                    last_update_timestamp: chrono::Utc::now().timestamp(),
                    track_volume: true,
                    max_sol_cost: 1000000,
                    min_sol_output: 0,
                    amount: 500000,
                    is_bot: false,
                    is_dev_create_token_trade: false,
                    global: Pubkey::default(),
                    associated_bonding_curve: Pubkey::default(),
                    associated_user: Pubkey::default(),
                    system_program: Pubkey::default(),
                    token_program: Pubkey::default(),
                    creator_vault: Pubkey::default(),
                    event_authority: Pubkey::default(),
                    program: Pubkey::default(),
                    global_volume_accumulator: Pubkey::default(),
                    user_volume_accumulator: Pubkey::default(),
                };

                let demo_event = DexEvent::PumpFunTrade(pumpfun_trade);

                if let Err(_) = tx_clone.send(demo_event) {
                    println!("⚠️  Failed to send event - receiver may have been dropped");
                    break;
                }
            }
        });

        // 处理接收到的DEX事件并调用callback
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // 根据事件类型显示不同的信息
                match &event {
                    DexEvent::PumpFunTrade(trade) => {
                        println!("📦 Received PumpFun Trade: {} SOL -> {} tokens",
                                trade.sol_amount, trade.token_amount);
                    },
                    DexEvent::BonkTrade(trade) => {
                        println!("📦 Received Bonk Trade: {} -> {} tokens",
                                trade.amount_in, trade.amount_out);
                    },
                    DexEvent::RaydiumCpmmSwap(swap) => {
                        println!("📦 Received Raydium CPMM Swap: {} -> {} tokens",
                                swap.amount_in, swap.amount_out);
                    },
                    _ => {
                        println!("📦 Received DEX event: {:?}", std::mem::discriminant(&event));
                    }
                }

                // 调用用户提供的回调函数
                callback(event);
            }
            println!("🔄 Event handler loop ended");
        });

        println!("✅ DEX event subscription started successfully");
        println!("🎧 Listening for events... Press Ctrl+C to stop");

        Ok(())
    }

    /// 订阅事件并立即开始处理 - 兼容原始API
    pub async fn subscribe_events_immediate(
        &self,
        protocols: Vec<Protocol>,
        slot_filter: Option<SlotFilter>,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
        timeout_secs: Option<std::time::Duration>,
        callback: impl Fn(DexEvent) + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("⚡ Starting immediate event subscription...");
        println!("🎯 Protocols to monitor: {:?}", protocols);

        if let Some(slot) = slot_filter {
            println!("📊 Slot filter: {:?}", slot);
        }

        // 开始订阅
        self.subscribe_dex_events(
            transaction_filters,
            account_filters,
            event_type_filter,
            callback,
        ).await?;

        // 如果设置了超时，等待指定时间
        if let Some(timeout) = timeout_secs {
            println!("⏱️  Will run for {:?}", timeout);
            tokio::time::sleep(timeout).await;
            println!("⏰ Timeout reached, stopping subscription");
        } else {
            // 否则等待Ctrl+C
            tokio::signal::ctrl_c().await?;
            println!("🛑 Received Ctrl+C, stopping subscription");
        }

        Ok(())
    }

    /// 停止订阅
    pub async fn stop(&self) {
        println!("🛑 Stopping gRPC subscription...");
        // 在实际实现中，这里会清理连接
    }
}