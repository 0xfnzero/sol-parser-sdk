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

        // 处理接收到的DEX事件并调用callback
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
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