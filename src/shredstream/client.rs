//! ShredStream 客户端
//!
//! `solana_entry::entry::Entry` 在 Agave SDK 中带 `deprecated`（需显式启用不稳定 feature 才消除）；
//! 本模块仍依赖其 bincode 布局解码 Shred 侧 `entries` 负载。
#![allow(deprecated)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_queue::ArrayQueue;
use futures::StreamExt;
use solana_entry::entry::Entry as SolanaEntry;
use solana_sdk::message::VersionedMessage;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tonic::transport::{Channel, Endpoint};

use crate::core::now_micros;
use crate::grpc::types::EventTypeFilter;
use crate::shredstream::config::ShredStreamConfig;
use crate::shredstream::proto::{Entry, ShredstreamProxyClient, SubscribeEntriesRequest};
use crate::DexEvent;

static SHREDSTREAM_DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);

enum EventSink<'a> {
    Queue(&'a Arc<ArrayQueue<DexEvent>>),
    Callback(&'a (dyn Fn(DexEvent) + Send + Sync)),
}

impl EventSink<'_> {
    #[inline]
    fn deliver(&self, event: DexEvent) {
        match self {
            EventSink::Queue(queue) => {
                if queue.push(event).is_err() {
                    record_shredstream_dropped_event();
                }
            }
            EventSink::Callback(callback) => callback(event),
        }
    }
}

#[inline]
fn record_shredstream_dropped_event() -> u64 {
    let dropped = SHREDSTREAM_DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if dropped <= 10 || dropped.is_power_of_two() {
        log::warn!(
            target: "sol_parser_sdk::shredstream",
            "ShredStream event queue is full; dropped event count={}",
            dropped
        );
    }
    dropped
}

/// ShredStream 客户端
#[derive(Clone)]
pub struct ShredStreamClient {
    endpoint: String,
    config: ShredStreamConfig,
    subscription_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ShredStreamClient {
    /// 创建新客户端
    pub async fn new(endpoint: impl Into<String>) -> crate::common::AnyResult<Self> {
        Self::new_with_config(endpoint, ShredStreamConfig::default()).await
    }

    /// 使用自定义配置创建客户端
    pub async fn new_with_config(
        endpoint: impl Into<String>,
        config: ShredStreamConfig,
    ) -> crate::common::AnyResult<Self> {
        let endpoint = endpoint.into();
        // 测试连接
        let _ = Self::connect_client(&endpoint, &config).await?;

        Ok(Self { endpoint, config, subscription_handle: Arc::new(Mutex::new(None)) })
    }

    /// 订阅 DEX 事件（自动重连）
    ///
    /// 返回一个队列，事件会被推送到该队列中
    pub async fn subscribe(&self) -> crate::common::AnyResult<Arc<ArrayQueue<DexEvent>>> {
        self.subscribe_with_filter(None).await
    }

    /// 订阅 DEX 事件，并在 ShredStream 热路径中按 SDK 事件类型提前过滤。
    ///
    /// 过滤发生在解析分发前，用于低延迟场景避免解析不需要的协议/事件。
    pub async fn subscribe_with_filter(
        &self,
        event_type_filter: Option<EventTypeFilter>,
    ) -> crate::common::AnyResult<Arc<ArrayQueue<DexEvent>>> {
        // 停止现有订阅
        self.stop().await;

        let queue = Arc::new(ArrayQueue::new(100_000));
        let queue_clone = Arc::clone(&queue);

        let endpoint = self.endpoint.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut delay = config.reconnect_delay_ms;
            let mut attempts = 0u32;

            loop {
                if config.max_reconnect_attempts > 0 && attempts >= config.max_reconnect_attempts {
                    log::error!("Max reconnection attempts reached, giving up");
                    break;
                }
                attempts += 1;

                match Self::stream_events(
                    &endpoint,
                    &config,
                    event_type_filter.as_ref(),
                    EventSink::Queue(&queue_clone),
                )
                .await
                {
                    Ok(_) => {
                        delay = config.reconnect_delay_ms;
                        attempts = 0;
                    }
                    Err(e) => {
                        log::error!("ShredStream error: {} - retry in {}ms", e, delay);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        delay = (delay * 2).min(60_000);
                    }
                }
            }
        });

        *self.subscription_handle.lock().await = Some(handle);
        Ok(queue)
    }

    /// 订阅 DEX 事件，并在解析热路径中直接回调事件，避免跨任务队列调度。
    ///
    /// 这是最低延迟路径；回调会在 ShredStream 读流任务内执行，应避免阻塞 I/O 或重计算。
    pub async fn subscribe_with_filter_callback<F>(
        &self,
        event_type_filter: Option<EventTypeFilter>,
        callback: F,
    ) -> crate::common::AnyResult<()>
    where
        F: Fn(DexEvent) + Send + Sync + 'static,
    {
        self.stop().await;

        let endpoint = self.endpoint.clone();
        let config = self.config.clone();
        let callback = Arc::new(callback);

        let handle = tokio::spawn(async move {
            let mut delay = config.reconnect_delay_ms;
            let mut attempts = 0u32;

            loop {
                if config.max_reconnect_attempts > 0 && attempts >= config.max_reconnect_attempts {
                    log::error!("Max reconnection attempts reached, giving up");
                    break;
                }
                attempts += 1;

                match Self::stream_events_callback(
                    &endpoint,
                    &config,
                    event_type_filter.as_ref(),
                    callback.clone(),
                )
                .await
                {
                    Ok(_) => {
                        delay = config.reconnect_delay_ms;
                        attempts = 0;
                    }
                    Err(e) => {
                        log::error!("ShredStream error: {} - retry in {}ms", e, delay);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        delay = (delay * 2).min(60_000);
                    }
                }
            }
        });

        *self.subscription_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止订阅
    pub async fn stop(&self) {
        if let Some(handle) = self.subscription_handle.lock().await.take() {
            handle.abort();
        }
    }

    async fn connect_client(
        endpoint: &str,
        config: &ShredStreamConfig,
    ) -> crate::common::AnyResult<ShredstreamProxyClient<Channel>> {
        let mut builder = Endpoint::from_shared(endpoint.to_string())?;
        if config.connection_timeout_ms > 0 {
            builder = builder.connect_timeout(Duration::from_millis(config.connection_timeout_ms));
        }
        let channel = builder.connect().await?;
        Ok(ShredstreamProxyClient::new(channel)
            .max_decoding_message_size(config.max_decoding_message_size))
    }

    /// 核心事件流处理
    async fn stream_events(
        endpoint: &str,
        config: &ShredStreamConfig,
        event_type_filter: Option<&EventTypeFilter>,
        sink: EventSink<'_>,
    ) -> Result<(), String> {
        let mut client = Self::connect_client(endpoint, config).await.map_err(|e| e.to_string())?;
        let request = tonic::Request::new(SubscribeEntriesRequest {});
        let response = if config.request_timeout_ms > 0 {
            tokio::time::timeout(
                Duration::from_millis(config.request_timeout_ms),
                client.subscribe_entries(request),
            )
            .await
            .map_err(|_| {
                format!(
                    "ShredStream subscribe request timed out after {}ms",
                    config.request_timeout_ms
                )
            })?
            .map_err(|e| e.to_string())?
        } else {
            client.subscribe_entries(request).await.map_err(|e| e.to_string())?
        };
        let mut stream = response.into_inner();

        log::info!("ShredStream connected, receiving entries...");

        let mut events = Vec::with_capacity(4);
        while let Some(message) = stream.next().await {
            match message {
                Ok(entry) => {
                    Self::process_entry(entry, event_type_filter, &sink, &mut events);
                }
                Err(e) => {
                    log::error!("Stream error: {:?}", e);
                    return Err(e.to_string());
                }
            }
        }

        Ok(())
    }

    async fn stream_events_callback(
        endpoint: &str,
        config: &ShredStreamConfig,
        event_type_filter: Option<&EventTypeFilter>,
        callback: Arc<dyn Fn(DexEvent) + Send + Sync>,
    ) -> Result<(), String> {
        Self::stream_events(
            endpoint,
            config,
            event_type_filter,
            EventSink::Callback(callback.as_ref()),
        )
        .await
    }

    /// 处理单个 Entry 消息
    #[inline]
    fn process_entry(
        entry: Entry,
        event_type_filter: Option<&EventTypeFilter>,
        sink: &EventSink<'_>,
        events: &mut Vec<DexEvent>,
    ) {
        let slot = entry.slot;
        let recv_us = now_micros();

        // 反序列化 Entry 数据
        let entries = match bincode::deserialize::<Vec<SolanaEntry>>(&entry.entries) {
            Ok(e) => e,
            Err(e) => {
                log::debug!("Failed to deserialize entries: {}", e);
                return;
            }
        };

        // 处理每个 Entry 中的交易
        let mut tx_index = 0u64;
        for entry in entries {
            for transaction in entry.transactions.iter() {
                events.clear();
                Self::process_transaction(
                    transaction,
                    slot,
                    recv_us,
                    tx_index,
                    event_type_filter,
                    events,
                    sink,
                );
                tx_index += 1;
            }
        }
    }

    /// 处理单个交易
    #[inline]
    fn process_transaction(
        transaction: &solana_sdk::transaction::VersionedTransaction,
        slot: u64,
        recv_us: i64,
        tx_index: u64,
        event_type_filter: Option<&EventTypeFilter>,
        events: &mut Vec<DexEvent>,
        sink: &EventSink<'_>,
    ) {
        if transaction.signatures.is_empty() {
            return;
        }

        Self::parse_transaction_events(
            transaction,
            slot,
            recv_us,
            tx_index,
            event_type_filter,
            events,
        );

        for event in events.drain(..) {
            sink.deliver(event);
        }
    }

    #[inline]
    fn parse_transaction_events(
        transaction: &solana_sdk::transaction::VersionedTransaction,
        slot: u64,
        recv_us: i64,
        tx_index: u64,
        event_type_filter: Option<&EventTypeFilter>,
        events: &mut Vec<DexEvent>,
    ) {
        if transaction.signatures.is_empty() {
            return;
        }

        let signature = transaction.signatures[0];
        if let VersionedMessage::V0(m) = &transaction.message {
            if !m.address_table_lookups.is_empty() {
                log::trace!(
                    target: "sol_parser_sdk::shredstream",
                    "V0 tx uses address lookup tables; shred parser will use static accounts and default placeholders for ALT-loaded accounts"
                );
            }
        }
        // 热路径：`static_account_keys` 零拷贝、`pump_ix` 内不克隆 CompiledInstruction。
        super::pump_ix::parse_transaction_dex_events_with_filter(
            transaction,
            signature,
            slot,
            tx_index,
            recv_us,
            event_type_filter,
            events,
        );
        crate::core::pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge(events);

        for event in events.iter_mut() {
            if let Some(meta) = event.metadata_mut() {
                meta.grpc_recv_us = recv_us;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{EventMetadata, PumpFunCreateTokenEvent};
    use crate::instr::program_ids::PUMPFUN_PROGRAM_ID;
    use solana_sdk::hash::Hash;
    use solana_sdk::message::{
        compiled_instruction::CompiledInstruction, v0, MessageHeader, VersionedMessage,
    };
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signature;
    use solana_sdk::transaction::VersionedTransaction;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    fn push_string(data: &mut Vec<u8>, value: &str) {
        data.extend_from_slice(&(value.len() as u32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
    }

    fn pumpfun_create_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[24, 30, 200, 40, 5, 28, 7, 119]);
        push_string(&mut data, "Callback Test");
        push_string(&mut data, "CBT");
        push_string(&mut data, "https://example.invalid/callback.json");
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        data
    }

    fn pumpfun_create_tx() -> VersionedTransaction {
        let mut account_keys = (0..10).map(|_| Pubkey::new_unique()).collect::<Vec<_>>();
        account_keys.push(PUMPFUN_PROGRAM_ID);

        VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(v0::Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 0,
                },
                account_keys,
                recent_blockhash: Hash::default(),
                instructions: vec![CompiledInstruction::new_from_raw_parts(
                    10,
                    pumpfun_create_data(),
                    (0..10).collect(),
                )],
                address_table_lookups: Vec::new(),
            }),
        }
    }

    #[test]
    fn dropped_counter_increments_without_panicking() {
        let before = SHREDSTREAM_DROPPED_EVENTS.load(Ordering::Relaxed);
        let queue = ArrayQueue::new(1);

        queue
            .push(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: EventMetadata::default(),
                ..Default::default()
            }))
            .expect("first push fits");

        if queue
            .push(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: EventMetadata::default(),
                ..Default::default()
            }))
            .is_err()
        {
            record_shredstream_dropped_event();
        }

        assert!(SHREDSTREAM_DROPPED_EVENTS.load(Ordering::Relaxed) > before);
    }

    #[test]
    fn callback_path_delivers_events_without_queue() {
        let entries = vec![SolanaEntry {
            num_hashes: 1,
            hash: Hash::default(),
            transactions: vec![pumpfun_create_tx()],
        }];
        let entry = Entry { slot: 42, entries: bincode::serialize(&entries).unwrap() };
        let count = AtomicUsize::new(0);

        let mut events = Vec::with_capacity(4);
        ShredStreamClient::process_entry(
            entry,
            None,
            &EventSink::Callback(&|event| {
                assert!(matches!(event, DexEvent::PumpFunCreate(_)));
                assert_eq!(event.metadata().slot, 42);
                count.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            &mut events,
        );

        assert_eq!(count.load(AtomicOrdering::Relaxed), 1);
    }
}
