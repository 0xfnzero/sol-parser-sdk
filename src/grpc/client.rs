use super::types::*;
use crate::DexEvent;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use futures::StreamExt;
use log::error;
use tonic::transport::ClientTlsConfig;
use crossbeam_queue::ArrayQueue;
use memchr::memmem;
use std::sync::Arc;
use once_cell::sync::Lazy;

static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: "));


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

    /// 订阅DEX事件（零拷贝无锁队列）
    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
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

    pub async fn stop(&self) {
        println!("🛑 Stopping gRPC subscription...");
    }
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
                            Self::parse_transaction(&transaction_update, grpc_recv_us, &queue, event_type_filter.as_ref()).await;
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

    /// 解析交易事件
    async fn parse_transaction(
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

                                Self::parse_events(
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

    /// 解析日志事件到队列
    #[inline]
    fn parse_events(
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
            let has_create = event_type_filter
                .map(|f| f.includes_pumpfun())
                .unwrap_or(true)
                && crate::logs::optimized_matcher::detect_pumpfun_create(logs);

            for log in logs.iter() {
                let log_bytes = log.as_bytes();

                if PROGRAM_DATA_FINDER.find(log_bytes).is_none() {
                    continue;
                }

                if let Some(log_event) = crate::logs::parse_log(log, signature, slot, block_time, grpc_recv_us, event_type_filter, has_create) {
                    let _ = queue.push(log_event);
                    *log_events_parsed = true;
                    return;
                }
            }

            *log_events_parsed = true;
        }
    }
}
