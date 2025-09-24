use super::types::*;
use crate::DexEvent;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use futures::StreamExt;
use log::error;
use tonic::transport::ClientTlsConfig;
use crossbeam_channel::{unbounded, Sender, Receiver};
use crossbeam_queue::ArrayQueue;
use memchr::memmem;
use std::sync::Arc;


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

    /// 订阅DEX事件（无锁队列版本）
    pub async fn subscribe_dex_events_with_channel(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Receiver<DexEvent>, Box<dyn std::error::Error>> {
        let (tx, rx) = unbounded();

        let self_clone = self.clone();
        tokio::spawn(async move {
            let _ = self_clone.stream_to_channel(
                transaction_filters,
                account_filters,
                event_type_filter,
                tx,
            ).await;
        });

        Ok(rx)
    }

    /// 订阅DEX事件（零拷贝无锁队列版本 - 极致性能）
    pub async fn subscribe_dex_events_zero_copy(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
        // 使用无锁环形队列，容量10万事件
        let queue = Arc::new(ArrayQueue::new(100_000));
        let queue_clone = Arc::clone(&queue);

        let self_clone = self.clone();
        tokio::spawn(async move {
            let _ = self_clone.stream_to_queue(
                transaction_filters,
                account_filters,
                event_type_filter,
                queue_clone,
            ).await;
        });

        Ok(queue)
    }

    /// 订阅DEX事件（回调版本 - 兼容旧接口）
    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
        callback: impl Fn(DexEvent) + Send + Sync + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rx = self.subscribe_dex_events_with_channel(
            transaction_filters,
            account_filters,
            event_type_filter,
        ).await?;

        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                // 计算从gRPC接收到队列接收的耗时
                let queue_recv_us = unsafe {
                    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                    libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
                    (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
                };

                let grpc_recv_us = match &event {
                    DexEvent::PumpFunTrade(e) => e.metadata.grpc_recv_us,
                    DexEvent::PumpFunCreate(e) => e.metadata.grpc_recv_us,
                    DexEvent::PumpFunMigrate(e) => e.metadata.grpc_recv_us,
                    DexEvent::RaydiumAmmV4Swap(e) => e.metadata.grpc_recv_us,
                    DexEvent::RaydiumClmmSwap(e) => e.metadata.grpc_recv_us,
                    DexEvent::RaydiumCpmmSwap(e) => e.metadata.grpc_recv_us,
                    _ => 0,
                };

                let latency_us = queue_recv_us - grpc_recv_us;
                println!("⏱️  队列接收耗时: {}μs", latency_us);

                callback(event);
            }
        });

        Ok(())
    }

    /// 流式订阅到 Channel
    async fn stream_to_channel(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
        tx: Sender<DexEvent>,
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

        // 处理订阅响应 - 使用无锁队列
        tokio::spawn(async move {
            let mut event_count = 0;
            let start_time = std::time::Instant::now();

            while let Some(response) = subscribe_rx.next().await {
                match response {
                    Ok(msg) => {
                        event_count += 1;

                        match msg.update_oneof {
                            Some(subscribe_update::UpdateOneof::Transaction(transaction_update)) => {
                                // 记录gRPC接收时间（微秒）
                                let grpc_recv_us = unsafe {
                                    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                                    libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
                                    (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
                                };
                                Self::parse_transaction_to_events_streaming_queue(&transaction_update, grpc_recv_us, &tx, event_type_filter.as_ref()).await;
                            },
                            Some(subscribe_update::UpdateOneof::Account(_account_update)) => {
                            },
                            Some(subscribe_update::UpdateOneof::Slot(_slot_update)) => {
                            },
                            _ => {
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
                        // 解析账户密钥（预分配容量以减少重新分配）
                        let mut accounts = Vec::with_capacity(message.account_keys.len());
                        for key in &message.account_keys {
                            if key.len() == 32 {
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(key);
                                accounts.push(Pubkey::new_from_array(pubkey_bytes));
                            }
                        }

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

    /// 流式解析交易为 DEX 事件 - 队列版本（直接发送到无锁队列）
    async fn parse_transaction_to_events_streaming_queue(
        transaction_update: &SubscribeUpdateTransaction,
        grpc_recv_us: i64,
        tx: &Sender<DexEvent>,
        event_type_filter: Option<&EventTypeFilter>,
    ) {
        // 从 transaction_update 中提取数据
        if let Some(transaction_info) = &transaction_update.transaction {
            let tx_index = transaction_info.index;

            if let Some(meta) = &transaction_info.meta {
                let logs = &meta.log_messages;

                if let Some(tx_msg) = &transaction_info.transaction {
                    if let Some(message) = &tx_msg.message {
                        // 解析账户密钥
                        let mut accounts = Vec::with_capacity(message.account_keys.len());
                        for key in &message.account_keys {
                            if key.len() == 32 {
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(key);
                                accounts.push(Pubkey::new_from_array(pubkey_bytes));
                            }
                        }

                        // 解析签名
                        let signature = if let Some(sig) = tx_msg.signatures.first() {
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

                        let block_time = Some(chrono::Utc::now().timestamp());
                        let mut log_events_parsed = false;

                        // 流式解析所有指令
                        for instruction in &message.instructions {
                            let program_id_index = instruction.program_id_index as usize;
                            if program_id_index < accounts.len() {
                                let program_id = accounts[program_id_index];

                                Self::parse_transaction_events_streaming_with_queue(
                                    &instruction.data,
                                    &accounts,
                                    &logs,
                                    signature,
                                    transaction_update.slot,
                                    tx_index,
                                    block_time,
                                    &program_id,
                                    grpc_recv_us,
                                    tx,
                                    &mut log_events_parsed,
                                    event_type_filter,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// 流式解析交易为 DEX 事件 - 每解析出一个事件立即回调
    async fn parse_transaction_to_events_streaming<F>(
        transaction_update: &SubscribeUpdateTransaction,
        grpc_recv_time: std::time::Instant,
        callback: &mut F
    ) where
        F: FnMut(DexEvent)
    {
        // 从 transaction_update 中提取数据
        if let Some(transaction_info) = &transaction_update.transaction {
            // 提取 transaction index (SubscribeUpdateTransactionInfo 有 index 字段)
            let tx_index = transaction_info.index;

            if let Some(meta) = &transaction_info.meta {
                // 使用引用避免 clone，提升性能
                let logs = &meta.log_messages;

                // 从交易中提取指令数据
                if let Some(tx) = &transaction_info.transaction {
                    if let Some(message) = &tx.message {
                        // 解析账户密钥（预分配容量以减少重新分配）
                        let mut accounts = Vec::with_capacity(message.account_keys.len());
                        for key in &message.account_keys {
                            if key.len() == 32 {
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(key);
                                accounts.push(Pubkey::new_from_array(pubkey_bytes));
                            }
                        }

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

                        // 预先计算时间戳（只调用一次），避免重复系统调用
                        let block_time = Some(chrono::Utc::now().timestamp());

                        // 优化：日志只解析一次，所有指令共享解析结果
                        let mut log_events_parsed = false;

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
                                    tx_index,
                                    block_time,
                                    &program_id,
                                    grpc_recv_time,
                                    callback,
                                    &mut log_events_parsed,  // 传递标志，避免重复解析日志
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// 流式解析交易事件 - 队列版本（直接发送到队列）
    #[inline]
    fn parse_transaction_events_streaming_with_queue(
        instruction_data: &[u8],
        accounts: &[Pubkey],
        logs: &[String],
        signature: solana_sdk::signature::Signature,
        slot: u64,
        tx_index: u64,
        block_time: Option<i64>,
        program_id: &Pubkey,
        grpc_recv_us: i64,
        tx: &Sender<DexEvent>,
        log_events_parsed: &mut bool,
        event_type_filter: Option<&EventTypeFilter>,
    ) {
        let mut has_log_event = false;

        // 流式处理日志事件
        if !*log_events_parsed {
            for log in logs.iter() {
                let log_bytes = log.as_bytes();

                // 使用 SIMD 快速检查是否包含 "Program data: "
                if memmem::find(log_bytes, b"Program data: ").is_none() {
                    continue;
                }

                if let Some(log_event) = crate::logs::parse_log_unified_with_grpc_time(log, signature, slot, block_time, grpc_recv_us, event_type_filter) {
                    // 直接发送到队列
                    let _ = tx.send(log_event);
                    has_log_event = true;

                    // 早期退出：找到事件后立即返回，不继续遍历
                    *log_events_parsed = true;
                    return;
                }
            }

            *log_events_parsed = true;
        }

        // 3. 如果有日志事件则返回
        if has_log_event {
            return;
        }

        // // 4. 如果没有日志事件，输出指令事件
        // if !*log_events_parsed {
        //     if let Some(instr_event) = instr_event {
        //         let _ = tx.send(instr_event);
        //         *log_events_parsed = true;
        //     }
        // }
    }

    /// 流式解析交易事件 - 优先解析日志，指令补充缺失字段后合并
    fn parse_transaction_events_streaming<F>(
        instruction_data: &[u8],
        accounts: &[Pubkey],
        logs: &[String],
        signature: solana_sdk::signature::Signature,
        slot: u64,
        tx_index: u64,
        block_time: Option<i64>,
        program_id: &Pubkey,
        grpc_recv_time: std::time::Instant,
        callback: &mut F,
        log_events_parsed: &mut bool,  // 标志：日志是否已解析
    ) where
        F: FnMut(DexEvent)
    {
        let total_start = std::time::Instant::now();
        let instruction_accounts = accounts;
        let mut event_count = 0u32;

        // 1. 先检查是否有指令事件，用于后续去重判断
        let instr_start = std::time::Instant::now();
        let instr_event = crate::instr::parse_instruction_unified(
            instruction_data, accounts, signature, slot, Some(tx_index), block_time, program_id
        );
        let instr_time = instr_start.elapsed().as_micros();

        // 2. 流式处理日志事件：快速过滤 + 解析 + 回调（只处理一次）
        let loop_start = std::time::Instant::now();
        let mut has_log_event = false;
        let mut total_parse_time = 0u128;
        let mut total_fill_time = 0u128;
        let mut total_callback_time = 0u128;
        let mut log_count = 0u32;
        let mut matched_count = 0u32;
        let mut filtered_count = 0u32;

        // 优化：日志只在第一个指令时解析，后续指令跳过
        if !*log_events_parsed {
            for log in logs.iter() {
                log_count += 1;

                // 快速过滤：只处理 "Program data:" 日志
                if !log.contains("Program data:") {
                    filtered_count += 1;
                    continue;
                }

                let parse_start = std::time::Instant::now();
                if let Some(mut log_event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
                    let parse_time = parse_start.elapsed().as_micros();
                    total_parse_time += parse_time;
                    matched_count += 1;

                    // 填充账户信息
                    let fill_start = std::time::Instant::now();
                    crate::core::account_filler::fill_accounts_from_instruction_data(&mut log_event, instruction_accounts);
                    let fill_time = fill_start.elapsed().as_micros();
                    total_fill_time += fill_time;

                    // 发送到队列并统计端到端耗时
                    let send_start = std::time::Instant::now();
                    callback(log_event);
                    let send_time = send_start.elapsed().as_micros();
                    total_callback_time += send_time;

                    // 计算从接收gRPC到发送队列的总耗时
                    let end_to_end_time = grpc_recv_time.elapsed().as_micros();

                    event_count += 1;
                    has_log_event = true;
                } else {
                    // 未匹配的日志也要计入解析时间
                    total_parse_time += parse_start.elapsed().as_micros();
                }
            }
            *log_events_parsed = true;  // 标记已解析
        }
        let loop_time = loop_start.elapsed().as_micros();

        // 3. 如果日志有事件，提前返回（已解析过的指令不再处理）
        if has_log_event {
            let total_time = total_start.elapsed().as_micros();
            let overhead = total_time.saturating_sub(instr_time + loop_time);
            let end_to_end_time = grpc_recv_time.elapsed().as_micros();

            println!("📊 解析统计 | 总:{} 指令:{} 循环:{} (解析:{} 填充:{} 队列:{}) 开销:{} | 过滤:{} 匹配:{}/{} 事件:{} | 🔄端到端:{}μs",
                total_time, instr_time, loop_time, total_parse_time, total_fill_time, total_callback_time,
                overhead, filtered_count, matched_count, log_count, event_count, end_to_end_time);
            println!("────────────────────────────────────────");
            return;
        }

        // 4. 如果没有日志事件，则输出指令事件（仅第一个指令）
        if !*log_events_parsed {
            if let Some(instr_event) = instr_event {
                let callback_start = std::time::Instant::now();
                callback(instr_event);
                total_callback_time += callback_start.elapsed().as_micros();
                event_count += 1;

                let total_time = total_start.elapsed().as_micros();
                let overhead = total_time.saturating_sub(instr_time + loop_time);

                println!("📊 解析统计 | 总:{} 指令:{} 循环:{} (解析:{} 填充:{} 回调:{}) 开销:{} | 过滤:{} 匹配:{}/{} 事件:{}",
                    total_time, instr_time, loop_time, total_parse_time, total_fill_time, total_callback_time,
                    overhead, filtered_count, matched_count, log_count, event_count);
                println!("────────────────────────────────────────");
            }
            *log_events_parsed = true;  // 标记已解析，后续指令跳过
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

    /// 流式订阅到无锁队列
    async fn stream_to_queue(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
        queue: Arc<ArrayQueue<DexEvent>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting Zero-Copy DEX event subscription...");

        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .x_token(self.token.clone())?
            .max_decoding_message_size(1024 * 1024 * 1024);

        if self.config.connection_timeout_ms > 0 {
            builder = builder.connect_timeout(std::time::Duration::from_millis(self.config.connection_timeout_ms));
        }

        // 添加 TLS 配置
        if self.config.enable_tls {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            builder = builder.tls_config(tls_config)?;
        }

        println!("🔗 Connecting to gRPC endpoint: {}", self.endpoint);
        println!("⏱️  Connection timeout: {}ms", self.config.connection_timeout_ms);

        let mut client = match builder.connect().await {
            Ok(c) => {
                println!("✅ Connection established");
                c
            },
            Err(e) => {
                println!("❌ Connection failed: {:?}", e);
                return Err(e.into());
            }
        };
        println!("✅ Connected to Yellowstone gRPC");

        println!("📝 Building subscription filters...");
        let mut accounts: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();
        for (i, filter) in account_filters.iter().enumerate() {
            let key = format!("account_filter_{}", i);
            accounts.insert(key, SubscribeRequestFilterAccounts {
                account: filter.account.clone(),
                owner: filter.owner.clone(),
                filters: vec![],
                nonempty_txn_signature: None,
            });
        }

        let mut transactions: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();
        for (i, filter) in transaction_filters.iter().enumerate() {
            let key = format!("transaction_filter_{}", i);
            transactions.insert(key, SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: filter.account_include.clone(),
                account_exclude: filter.account_exclude.clone(),
                account_required: filter.account_required.clone(),
            });
        }

        let request = SubscribeRequest {
            slots: HashMap::new(),
            accounts,
            transactions,
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(CommitmentLevel::Processed as i32),
            accounts_data_slice: Vec::new(),
            ping: None,
            from_slot: None,
        };

        println!("📡 Subscribing to stream...");
        let (_subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
        println!("✅ Subscribed successfully - Zero Copy Mode");
        println!("👂 Listening for events...");

        let mut msg_count = 0u64;
        while let Some(message) = stream.next().await {
            match message {
                Ok(update_msg) => {
                    msg_count += 1;
                    if msg_count % 100 == 0 {
                        println!("📨 Received {} messages", msg_count);
                    }

                    if let Some(update) = update_msg.update_oneof {
                        if let subscribe_update::UpdateOneof::Transaction(transaction_update) = update {
                            let grpc_recv_us = unsafe {
                                let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                                libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
                                (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
                            };
                            Self::parse_transaction_to_events_zero_copy(&transaction_update, grpc_recv_us, &queue, event_type_filter.as_ref()).await;
                        }
                    }
                },
                Err(e) => {
                    error!("Stream error: {:?}", e);
                    println!("❌ Stream error: {:?}", e);
                },
            }
        }

        println!("⚠️  Stream ended");

        Ok(())
    }

    /// 零拷贝事件解析（ArrayQueue）
    async fn parse_transaction_to_events_zero_copy(
        transaction_update: &SubscribeUpdateTransaction,
        grpc_recv_us: i64,
        queue: &Arc<ArrayQueue<DexEvent>>,
        event_type_filter: Option<&EventTypeFilter>,
    ) {
        if let Some(transaction_info) = &transaction_update.transaction {
            if let Some(meta) = &transaction_info.meta {
                let logs = &meta.log_messages;

                if let Some(tx_msg) = &transaction_info.transaction {
                    if let Some(message) = &tx_msg.message {
                        let mut accounts = Vec::with_capacity(message.account_keys.len());
                        for key in &message.account_keys {
                            if key.len() == 32 {
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(key);
                                accounts.push(Pubkey::new_from_array(pubkey_bytes));
                            }
                        }

                        let signature = if let Some(sig) = tx_msg.signatures.first() {
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

                        let block_time = Some(chrono::Utc::now().timestamp());
                        let mut log_events_parsed = false;

                        for instruction in &message.instructions {
                            let program_id_index = instruction.program_id_index as usize;
                            if program_id_index < accounts.len() {
                                let _program_id = accounts[program_id_index];

                                Self::parse_events_zero_copy_queue(
                                    &accounts,
                                    logs,
                                    signature,
                                    transaction_update.slot,
                                    block_time,
                                    grpc_recv_us,
                                    queue,
                                    &mut log_events_parsed,
                                    event_type_filter,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// 零拷贝解析事件到 ArrayQueue
    #[inline]
    fn parse_events_zero_copy_queue(
        _accounts: &[Pubkey],
        logs: &[String],
        signature: solana_sdk::signature::Signature,
        slot: u64,
        block_time: Option<i64>,
        grpc_recv_us: i64,
        queue: &Arc<ArrayQueue<DexEvent>>,
        log_events_parsed: &mut bool,
        event_type_filter: Option<&EventTypeFilter>,
    ) {
        if !*log_events_parsed {
            for log in logs.iter() {
                let log_bytes = log.as_bytes();

                if memmem::find(log_bytes, b"Program data: ").is_none() {
                    continue;
                }

                if let Some(log_event) = crate::logs::parse_log_unified_with_grpc_time(log, signature, slot, block_time, grpc_recv_us, event_type_filter) {
                    // 无锁推送到队列，如果队列满了就丢弃（背压处理）
                    let _ = queue.push(log_event);
                    *log_events_parsed = true;
                    return;
                }
            }

            *log_events_parsed = true;
        }
    }
}