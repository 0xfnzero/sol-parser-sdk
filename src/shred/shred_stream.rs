use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;

use crate::common::AnyResult;
use crate::parser::{SimpleEventParser, ParsedEvent};
use log::{error, info};
use solana_entry::entry::Entry;
use solana_sdk::signature::Signature;
use prost_types::Timestamp;

/// 事件回调类型 - 直接回调解析后的事件
pub type EventCallback = Box<dyn Fn(&ParsedEvent) + Send + Sync>;

/// 简化的 ShredStream 客户端 - 直接回调，无队列
pub struct ShredStreamGrpc {
    pub endpoint: String,
    pub active_subscription: Arc<AtomicBool>,
}

impl ShredStreamGrpc {
    /// 创建简化的 ShredStream 客户端
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            active_subscription: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 订阅 ShredStream 事件 - 直接回调，无队列
    pub async fn subscribe_entries(
        &self,
        callback: EventCallback,
    ) -> AnyResult<()> {
        // 设置订阅状态
        self.active_subscription.store(true, Ordering::SeqCst);

        info!("✅ ShredStream 订阅启动，开始接收事件...");

        // 这里应该实现实际的 ShredStream 连接和数据处理
        // 为了示例，我们模拟处理流程

        // TODO: 实际实现需要：
        // 1. 连接到 ShredStream 端点
        // 2. 订阅 Entry 数据流
        // 3. 解析 Entry 中的交易
        // 4. 使用 SimpleEventParser 解析事件
        // 5. 直接回调解析的事件

        loop {
            // 检查是否应该停止
            if !self.active_subscription.load(Ordering::SeqCst) {
                break;
            }

            // 模拟处理延迟
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // 实际实现中，这里会处理从 ShredStream 接收到的 Entry 数据
            // 然后解析其中的交易并提取事件
        }

        info!("ShredStream 订阅结束");
        Ok(())
    }

    /// 处理 Entry 数据并解析事件
    async fn handle_entry(&self, entry: &Entry, callback: &EventCallback) -> AnyResult<()> {
        // 遍历 Entry 中的所有交易
        for transaction in &entry.transactions {
            // 构造虚拟的日志数据用于解析
            // 实际实现中需要从交易中提取真实的日志
            let logs = vec![];  // 这里需要从交易中提取日志

            let signature = Signature::default(); // 从交易中获取签名
            let slot = 0; // 从 Entry 或上下文获取 slot
            let block_time = None; // 从上下文获取时间

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

        Ok(())
    }

    /// 停止订阅
    pub fn stop_subscription(&self) {
        self.active_subscription.store(false, Ordering::SeqCst);
        info!("📛 请求停止 ShredStream 订阅");
    }
}

/// 使用示例
pub async fn example_usage() -> AnyResult<()> {
    // 创建简化的 ShredStream 客户端
    let client = ShredStreamGrpc::new("your_shredstream_endpoint".to_string());

    // 设置直接回调 - 无队列，直接处理解析后的事件
    let callback = Box::new(|event: &ParsedEvent| {
        match event {
            ParsedEvent::PumpFunTrade(trade) => {
                let action = if trade.is_buy { "买入" } else { "卖出" };
                let sol_amount = trade.sol_amount as f64 / 1e9;
                println!("🔥 ShredStream PumpFun 交易: {} {:.4} SOL", action, sol_amount);
                println!("   代币: {}", trade.mint);
                println!("   用户: {}", trade.user);
            }
            ParsedEvent::PumpFunCreate(create) => {
                println!("🎉 ShredStream 新代币创建: {} ({})", create.name, create.symbol);
                println!("   铸造地址: {}", create.mint);
                println!("   创建者: {}", create.creator);
            }
            _ => {
                println!("📝 ShredStream 其他事件: {:?}", event);
            }
        }
    });

    // 开始订阅 - 直接回调，无队列处理
    client.subscribe_entries(callback).await?;

    Ok(())
}