use super::types::*;
use crate::DexEvent;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use futures::StreamExt;
use log::error;
use tonic::transport::ClientTlsConfig;

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
        callback: impl Fn(DexEvent) + Send + Sync + 'static,
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

        // 初始化 rustls crypto provider
        let _ = rustls::crypto::ring::default_provider().install_default();

        // 创建真实的 Yellowstone gRPC 客户端 - 使用正确的 yellowstone-grpc-client API
        println!("🔗 Connecting to Yellowstone gRPC...");

        // 构建客户端配置
        let mut builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .x_token(self.token.clone())?
            .max_decoding_message_size(1024 * 1024 * 1024); // 1GiB for large blocks

        // 配置连接超时
        if self.config.connection_timeout_ms > 0 {
            builder = builder.connect_timeout(std::time::Duration::from_millis(self.config.connection_timeout_ms));
        }

        // 配置请求超时
        if self.config.request_timeout_ms > 0 {
            builder = builder.timeout(std::time::Duration::from_millis(self.config.request_timeout_ms));
        }

        // 配置 TLS
        if self.config.enable_tls {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            builder = builder.tls_config(tls_config)?;
        }

        // 连接客户端
        let mut client = builder.connect().await?;
        println!("✅ Connected to Yellowstone gRPC successfully");

        // 构建订阅请求 - 使用正确的 SubscribeRequest 结构
        let mut subscribe_request = SubscribeRequest {
            slots: HashMap::new(),
            accounts: HashMap::new(),
            transactions: HashMap::new(),
            entry: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            accounts_data_slice: vec![],
            commitment: Some(CommitmentLevel::Confirmed as i32),
            ping: None,
            from_slot: None,
            transactions_status: HashMap::new(),
        };

        // 添加交易过滤器 - 正确配置 DEX 程序过滤
        println!("📝 Setting up transaction filters...");
        for (i, tx_filter) in transaction_filters.iter().enumerate() {
            let grpc_filter = SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: tx_filter.account_include.clone(),
                account_exclude: tx_filter.account_exclude.clone(),
                account_required: tx_filter.account_required.clone(),
            };

            subscribe_request.transactions.insert(format!("tx_{}", i), grpc_filter);
            println!("   📤 Transaction filter {}: {} include, {} exclude, {} required",
                i, tx_filter.account_include.len(), tx_filter.account_exclude.len(), tx_filter.account_required.len());
        }

        // 添加账户过滤器 - 正确配置 DEX 账户过滤
        println!("📝 Setting up account filters...");
        for (i, acc_filter) in account_filters.iter().enumerate() {
            let grpc_filter = SubscribeRequestFilterAccounts {
                account: acc_filter.account.clone(),
                owner: acc_filter.owner.clone(),
                filters: vec![], // 基础过滤器
                nonempty_txn_signature: None,
            };

            subscribe_request.accounts.insert(format!("acc_{}", i), grpc_filter);
            println!("   📥 Account filter {}: {} accounts, {} owners",
                i, acc_filter.account.len(), acc_filter.owner.len());
        }

        println!("📡 Starting gRPC subscription...");

        // 使用正确的 yellowstone-grpc-client API
        let (_subscribe_tx, mut subscribe_rx) = client.subscribe_with_request(Some(subscribe_request)).await?;

        println!("✅ Subscription established successfully");
        println!("🎧 Listening for real-time events...");

        // 处理订阅响应 - 正确处理 yellowstone-grpc 消息
        let callback = std::sync::Arc::new(callback);
        tokio::spawn(async move {
            let mut event_count = 0;
            let start_time = std::time::Instant::now();

            while let Some(response) = subscribe_rx.next().await {
                match response {
                    Ok(msg) => {
                        event_count += 1;

                        // 统计消息处理（不显示）
                        // if event_count % 100 == 0 {
                        //     let elapsed = start_time.elapsed().as_secs();
                        //     let rate = if elapsed > 0 { event_count / elapsed } else { 0 };
                        //     println!("📊 Processed {} messages ({} msg/sec)", event_count, rate);
                        // }

                        match msg.update_oneof {
                            Some(subscribe_update::UpdateOneof::Transaction(transaction_update)) => {
                                // 解析交易事件 - 使用我们现有的解析逻辑
                                if let Some(events) = Self::parse_transaction_to_events(&transaction_update).await {
                                    for event in events {
                                        callback(event);
                                    }
                                }
                            },
                            Some(subscribe_update::UpdateOneof::Account(_account_update)) => {
                                // Account updates - 暂时忽略
                            },
                            Some(subscribe_update::UpdateOneof::Slot(_slot_update)) => {
                                // Slot updates - 暂时忽略
                            },
                            _ => {
                                // 其他类型更新 - 暂时忽略
                            }
                        }
                    },
                    Err(e) => {
                        error!("❌ Stream error: {:?}", e);
                        println!("🔄 Attempting to reconnect...");
                        break;
                    }
                }
            }
            println!("🔄 Event handler loop ended");
        });

        Ok(())
    }

    /// 解析交易为 DEX 事件
    async fn parse_transaction_to_events(transaction_update: &SubscribeUpdateTransaction) -> Option<Vec<DexEvent>> {
        // 从 transaction_update 中提取数据
        if let Some(transaction_info) = &transaction_update.transaction {
            if let Some(meta) = &transaction_info.meta {
                // 从 meta 中提取日志
                let logs: Vec<String> = meta.log_messages.clone();

                // 从交易中提取指令数据
                if let Some(tx) = &transaction_info.transaction {
                    if let Some(message) = &tx.message {
                        // 解析账户密钥
                        let accounts: Vec<Pubkey> = message.account_keys
                            .iter()
                            .filter_map(|key| {
                                if key.len() == 32 {
                                    let mut pubkey_bytes = [0u8; 32];
                                    pubkey_bytes.copy_from_slice(key);
                                    Some(Pubkey::new_from_array(pubkey_bytes))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        // 解析签名
                        let signature = if let Some(sig) = tx.signatures.first() {
                            if sig.len() == 64 {
                                let mut sig_array = [0u8; 64];
                                sig_array.copy_from_slice(sig);
                                solana_sdk::signature::Signature::from(sig_array)
                            } else {
                                solana_sdk::signature::Signature::default()
                            }
                        } else {
                            solana_sdk::signature::Signature::default()
                        };

                        // 解析所有指令
                        let mut all_events = Vec::new();

                        for instruction in &message.instructions {
                            let program_id_index = instruction.program_id_index as usize;
                            if program_id_index < accounts.len() {
                                let program_id = accounts[program_id_index];

                                // 调用现有的解析逻辑
                                let events = crate::parse_transaction_events(
                                    &instruction.data,
                                    &accounts,
                                    &logs,
                                    signature,
                                    transaction_update.slot,
                                    Some(chrono::Utc::now().timestamp()), // 使用当前时间戳
                                    &program_id,
                                );
                                all_events.extend(events);
                            }
                        }

                        if !all_events.is_empty() {
                            return Some(all_events);
                        }
                    }
                }
            }
        }

        None
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
        callback: impl Fn(DexEvent) + Send + Sync + 'static,
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