use crate::common::AnyResult;
use crate::parser::{SimpleEventParser, ParsedEvent};
use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use solana_sdk::pubkey::Pubkey;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccountsFilter,
    SubscribeRequestFilterTransactions, SubscribeUpdate,
};
use yellowstone_grpc_client::GeyserGrpcClient;
use prost_types::Timestamp;
use solana_sdk::signature::Signature;

/// 事件回调类型 - 直接回调解析后的事件
pub type EventCallback = Box<dyn Fn(&ParsedEvent) + Send + Sync>;

/// 交易过滤器
#[derive(Debug, Clone, Default)]
pub struct TransactionFilter {
    pub account_include: Vec<String>,
    pub account_exclude: Vec<String>,
    pub account_required: Vec<String>,
}

/// 账户过滤器
#[derive(Debug, Clone, Default)]
pub struct AccountFilter {
    pub account: Vec<String>,
    pub owner: Vec<String>,
    pub filters: Vec<SubscribeRequestFilterAccountsFilter>,
}

/// 简化的 YellowstoneGrpc 客户端 - 直接回调，无队列
pub struct YellowstoneGrpc {
    pub endpoint: String,
    pub x_token: Option<String>,
    pub active_subscription: Arc<AtomicBool>,
}

impl YellowstoneGrpc {
    /// 创建简化的客户端 - 直接回调，无队列
    pub fn new(endpoint: String, x_token: Option<String>) -> AnyResult<Self> {
        Ok(Self {
            endpoint,
            x_token,
            active_subscription: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 订阅交易事件 - 直接回调解析后的事件，无队列
    pub async fn subscribe_transactions(
        &self,
        transaction_filter: TransactionFilter,
        callback: EventCallback,
    ) -> AnyResult<()> {
        // 设置订阅状态
        self.active_subscription.store(true, Ordering::SeqCst);

        // 创建 gRPC 客户端
        let mut client = GeyserGrpcClient::connect(
            self.endpoint.clone(),
            self.x_token.clone(),
            None,
        )?;

        // 构建订阅请求
        let mut accounts = std::collections::HashMap::new();
        let mut transactions = std::collections::HashMap::new();

        // 添加交易过滤器
        if !transaction_filter.account_include.is_empty() ||
           !transaction_filter.account_exclude.is_empty() ||
           !transaction_filter.account_required.is_empty() {
            transactions.insert(
                "transactions".to_string(),
                SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: Some(false),
                    signature: None,
                    account_include: transaction_filter.account_include,
                    account_exclude: transaction_filter.account_exclude,
                    account_required: transaction_filter.account_required,
                },
            );
        }

        let request = SubscribeRequest {
            accounts,
            slots: std::collections::HashMap::new(),
            transactions,
            entry: std::collections::HashMap::new(),
            blocks: std::collections::HashMap::new(),
            blocks_meta: std::collections::HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed as i32),
            accounts_data_slice: Vec::new(),
            ping: None,
        };

        // 开始订阅
        let mut stream = client.subscribe_once(request).await?;

        info!("✅ gRPC 订阅启动，开始接收事件...");

        // 处理消息流
        while let Some(message) = stream.next().await {
            // 检查是否应该停止
            if !self.active_subscription.load(Ordering::SeqCst) {
                break;
            }

            match message {
                Ok(update) => {
                    if let Err(e) = self.handle_update(update, &callback).await {
                        error!("处理更新失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("gRPC 流错误: {}", e);
                    break;
                }
            }
        }

        info!("gRPC 订阅结束");
        Ok(())
    }

    /// 处理单个更新消息
    async fn handle_update(
        &self,
        update: SubscribeUpdate,
        callback: &EventCallback,
    ) -> AnyResult<()> {
        if let Some(update_oneof) = update.update_oneof {
            match update_oneof {
                UpdateOneof::Transaction(tx_update) => {
                    if let Some(transaction) = tx_update.transaction {
                        if let Some(meta) = transaction.meta {
                            // 解析交易日志
                            let logs = meta.log_messages;
                            let signature = if !transaction.signatures.is_empty() {
                                Signature::try_from(transaction.signatures[0].as_slice())
                                    .unwrap_or_default()
                            } else {
                                Signature::default()
                            };

                            let slot = tx_update.slot;
                            let block_time = tx_update.block_time;

                            // 使用简单解析器解析事件
                            let events = SimpleEventParser::parse_all_events_from_logs(
                                &logs,
                                signature,
                                slot,
                                block_time,
                            );

                            // 直接回调每个解析出的事件
                            for event in events {
                                callback(&event);
                            }
                        }
                    }
                }
                _ => {
                    // 其他类型的更新暂时忽略
                }
            }
        }
        Ok(())
    }

    /// 停止订阅
    pub fn stop_subscription(&self) {
        self.active_subscription.store(false, Ordering::SeqCst);
        info!("📛 请求停止 gRPC 订阅");
    }
}

/// 使用示例
pub async fn example_usage() -> AnyResult<()> {
    // 创建简化的 gRPC 客户端
    let client = YellowstoneGrpc::new(
        "https://api.mainnet-beta.solana.com".to_string(),
        None,
    )?;

    // 设置交易过滤器 - 只监听 PumpFun 相关的交易
    let filter = TransactionFilter {
        account_include: vec![
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string(), // PumpFun program
        ],
        account_exclude: vec![],
        account_required: vec![],
    };

    // 设置直接回调 - 无队列，直接处理解析后的事件
    let callback = Box::new(|event: &ParsedEvent| {
        match event {
            ParsedEvent::PumpFunTrade(trade) => {
                let action = if trade.is_buy { "买入" } else { "卖出" };
                let sol_amount = trade.sol_amount as f64 / 1e9;
                println!("🔥 PumpFun 交易: {} {:.4} SOL", action, sol_amount);
                println!("   代币: {}", trade.mint);
                println!("   用户: {}", trade.user);

                if sol_amount > 1.0 {
                    println!("   🚨 大额交易警告!");
                }
            }
            ParsedEvent::PumpFunCreate(create) => {
                println!("🎉 新代币创建: {} ({})", create.name, create.symbol);
                println!("   铸造地址: {}", create.mint);
                println!("   创建者: {}", create.creator);
            }
            ParsedEvent::BonkTrade(bonk) => {
                println!("🪙 Bonk 交易: 输入 {} / 输出 {}", bonk.amount_in, bonk.amount_out);
            }
            _ => {
                println!("📝 其他事件: {:?}", event);
            }
        }
    });

    // 开始订阅 - 直接回调，无队列处理
    client.subscribe_transactions(filter, callback).await?;

    Ok(())
}
