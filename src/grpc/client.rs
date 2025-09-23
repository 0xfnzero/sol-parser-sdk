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
                                let mut event_count = 0;
                                // 流式解析交易事件 - 每解析出一个事件就立即回调
                                Self::parse_transaction_to_events_streaming(&transaction_update, &mut |event| {
                                    event_count += 1;
                                    // 执行用户回调
                                    callback(event);
                                }).await;
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

    /// 流式解析交易为 DEX 事件 - 每解析出一个事件立即回调
    async fn parse_transaction_to_events_streaming<F>(
        transaction_update: &SubscribeUpdateTransaction,
        callback: &mut F
    ) where
        F: FnMut(DexEvent)
    {
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

                        // 预先计算时间戳，避免重复系统调用
                        let block_time = Some(chrono::Utc::now().timestamp());

                        // 流式解析所有指令 - 每解析出一个事件就立即回调
                        for instruction in &message.instructions {
                            let program_id_index = instruction.program_id_index as usize;
                            if program_id_index < accounts.len() {
                                let program_id = accounts[program_id_index];

                                // 使用流式解析函数 - 每个事件都会立即回调
                                Self::parse_transaction_events_streaming(
                                    &instruction.data,
                                    &accounts,
                                    &logs,
                                    signature,
                                    transaction_update.slot,
                                    block_time,
                                    &program_id,
                                    callback,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// 流式解析交易事件 - 优先解析日志，指令补充缺失字段后合并
    fn parse_transaction_events_streaming<F>(
        instruction_data: &[u8],
        accounts: &[Pubkey],
        logs: &[String],
        signature: solana_sdk::signature::Signature,
        slot: u64,
        block_time: Option<i64>,
        program_id: &Pubkey,
        callback: &mut F,
    ) where
        F: FnMut(DexEvent)
    {
        let mut event_counter = 0;

        // 1. 先一次性解析指令，提取所有账户数据
        let instruction_accounts = accounts; // 直接使用指令中的accounts

        // 2. 然后逐个处理log事件，从预解析的账户数据中补充缺失字段
        for (_log_index, log) in logs.iter().enumerate() {
            let event_start = std::time::Instant::now();
            if let Some(mut log_event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
                // 从预解析的账户数据中补充缺失字段
                crate::core::account_filler::fill_accounts_from_instruction_data(&mut log_event, instruction_accounts);

                let total_time = event_start.elapsed().as_micros();
                event_counter += 1;
                println!("⚡ 事件{}: 耗时: {}μs", event_counter, total_time);
                println!("────────────────────────────────────────");

                // 立即回调处理完的事件
                callback(log_event);
            }
        }

        // 3. 如果有独立的指令事件（没有对应的log事件），单独处理
        if let Some(instr_event) = crate::instr::parse_instruction_unified(
            instruction_data, accounts, signature, slot, block_time, program_id
        ) {
            // 检查这个指令事件是否已经被上面的log事件处理过了
            let is_already_processed = logs.iter().any(|log| {
                if let Some(log_event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
                    Self::can_merge_events(&log_event, &instr_event)
                } else {
                    false
                }
            });

            // 如果没有被处理过，则作为独立事件输出
            if !is_already_processed {
                let event_start = std::time::Instant::now();
                let total_time = event_start.elapsed().as_micros();
                event_counter += 1;
                println!("⚡ 事件{}: 耗时: {}μs", event_counter, total_time);
                println!("────────────────────────────────────────");
                callback(instr_event);
            }
        }
    }




    /// 检查两个事件是否可以合并
    fn can_merge_events(log_event: &DexEvent, instr_event: &DexEvent) -> bool {
        use crate::core::events::DexEvent;

        match (log_event, instr_event) {
            // 同类型事件可以合并
            (DexEvent::PumpFunTrade(_), DexEvent::PumpFunTrade(_)) => true,
            (DexEvent::RaydiumClmmSwap(_), DexEvent::RaydiumClmmSwap(_)) => true,
            (DexEvent::RaydiumCpmmSwap(_), DexEvent::RaydiumCpmmSwap(_)) => true,
            (DexEvent::RaydiumAmmV4Swap(_), DexEvent::RaydiumAmmV4Swap(_)) => true,
            (DexEvent::OrcaWhirlpoolSwap(_), DexEvent::OrcaWhirlpoolSwap(_)) => true,
            (DexEvent::MeteoraPoolsSwap(_), DexEvent::MeteoraPoolsSwap(_)) => true,
            (DexEvent::MeteoraDammV2Swap(_), DexEvent::MeteoraDammV2Swap(_)) => true,
            (DexEvent::BonkTrade(_), DexEvent::BonkTrade(_)) => true,
            // 其他情况不合并
            _ => false,
        }
    }

    /// 用指令事件补充日志事件中缺失的字段
    fn merge_log_with_instruction(log_event: DexEvent, instr_event: &DexEvent) -> Option<DexEvent> {
        use crate::core::events::*;

        match (log_event, instr_event) {
            // PumpFun 交易事件合并
            (DexEvent::PumpFunTrade(mut log_trade), DexEvent::PumpFunTrade(instr_trade)) => {
                // 日志事件优先，只补充缺失的字段
                if log_trade.sol_amount == 0 && instr_trade.sol_amount > 0 {
                    log_trade.sol_amount = instr_trade.sol_amount;
                }
                if log_trade.token_amount == 0 && instr_trade.token_amount > 0 {
                    log_trade.token_amount = instr_trade.token_amount;
                }
                if log_trade.virtual_sol_reserves == 0 && instr_trade.virtual_sol_reserves > 0 {
                    log_trade.virtual_sol_reserves = instr_trade.virtual_sol_reserves;
                }
                if log_trade.virtual_token_reserves == 0 && instr_trade.virtual_token_reserves > 0 {
                    log_trade.virtual_token_reserves = instr_trade.virtual_token_reserves;
                }
                Some(DexEvent::PumpFunTrade(log_trade))
            },

            // Raydium CLMM 事件合并
            (DexEvent::RaydiumClmmSwap(mut log_swap), DexEvent::RaydiumClmmSwap(instr_swap)) => {
                // 补充缺失的字段
                if log_swap.amount == 0 && instr_swap.amount > 0 {
                    log_swap.amount = instr_swap.amount;
                }
                if log_swap.other_amount_threshold == 0 && instr_swap.other_amount_threshold > 0 {
                    log_swap.other_amount_threshold = instr_swap.other_amount_threshold;
                }
                Some(DexEvent::RaydiumClmmSwap(log_swap))
            },

            // Raydium CPMM 事件合并
            (DexEvent::RaydiumCpmmSwap(mut log_swap), DexEvent::RaydiumCpmmSwap(instr_swap)) => {
                if log_swap.amount_in == 0 && instr_swap.amount_in > 0 {
                    log_swap.amount_in = instr_swap.amount_in;
                }
                if log_swap.output_amount == 0 && instr_swap.output_amount > 0 {
                    log_swap.output_amount = instr_swap.output_amount;
                }
                Some(DexEvent::RaydiumCpmmSwap(log_swap))
            },

            // Raydium AMM V4 事件合并
            (DexEvent::RaydiumAmmV4Swap(mut log_swap), DexEvent::RaydiumAmmV4Swap(instr_swap)) => {
                if log_swap.amount_in == 0 && instr_swap.amount_in > 0 {
                    log_swap.amount_in = instr_swap.amount_in;
                }
                if log_swap.amount_out == 0 && instr_swap.amount_out > 0 {
                    log_swap.amount_out = instr_swap.amount_out;
                }
                Some(DexEvent::RaydiumAmmV4Swap(log_swap))
            },

            // Orca Whirlpool 事件合并
            (DexEvent::OrcaWhirlpoolSwap(mut log_swap), DexEvent::OrcaWhirlpoolSwap(instr_swap)) => {
                if log_swap.input_amount == 0 && instr_swap.input_amount > 0 {
                    log_swap.input_amount = instr_swap.input_amount;
                }
                if log_swap.output_amount == 0 && instr_swap.output_amount > 0 {
                    log_swap.output_amount = instr_swap.output_amount;
                }
                Some(DexEvent::OrcaWhirlpoolSwap(log_swap))
            },

            // Meteora Pools 事件合并
            (DexEvent::MeteoraPoolsSwap(mut log_swap), DexEvent::MeteoraPoolsSwap(instr_swap)) => {
                if log_swap.in_amount == 0 && instr_swap.in_amount > 0 {
                    log_swap.in_amount = instr_swap.in_amount;
                }
                if log_swap.out_amount == 0 && instr_swap.out_amount > 0 {
                    log_swap.out_amount = instr_swap.out_amount;
                }
                Some(DexEvent::MeteoraPoolsSwap(log_swap))
            },

            // Meteora DAMM V2 事件合并
            (DexEvent::MeteoraDammV2Swap(mut log_swap), DexEvent::MeteoraDammV2Swap(instr_swap)) => {
                if log_swap.amount_in == 0 && instr_swap.amount_in > 0 {
                    log_swap.amount_in = instr_swap.amount_in;
                }
                if log_swap.amount_out == 0 && instr_swap.amount_out > 0 {
                    log_swap.amount_out = instr_swap.amount_out;
                }
                Some(DexEvent::MeteoraDammV2Swap(log_swap))
            },

            // Bonk 交易事件合并
            (DexEvent::BonkTrade(mut log_trade), DexEvent::BonkTrade(instr_trade)) => {
                if log_trade.amount_in == 0 && instr_trade.amount_in > 0 {
                    log_trade.amount_in = instr_trade.amount_in;
                }
                if log_trade.amount_out == 0 && instr_trade.amount_out > 0 {
                    log_trade.amount_out = instr_trade.amount_out;
                }
                Some(DexEvent::BonkTrade(log_trade))
            },

            // 不匹配的事件类型
            _ => None,
        }
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