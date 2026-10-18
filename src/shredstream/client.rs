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
use solana_sdk::message::v0::LoadedAddresses;
use solana_sdk::transaction::VersionedTransaction;
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

/// Synchronous ALT lookup from a prewarmed, slot-aware local cache. Called on
/// the receive task; never perform RPC or blocking I/O here. Address identity
/// and table activation at the requested slot are the caller's responsibility.
pub type AddressLookupResolver =
    dyn Fn(&VersionedTransaction, u64) -> crate::common::AnyResult<LoadedAddresses> + Send + Sync;

/// ShredStream 客户端
#[derive(Clone)]
pub struct ShredStreamClient {
    endpoint: String,
    address_lookup_resolver: Option<Arc<AddressLookupResolver>>,
    config: ShredStreamConfig,
    subscription_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    subscription_lifecycle: Arc<Mutex<()>>,
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

        Ok(Self {
            endpoint,
            address_lookup_resolver: None,
            config,
            subscription_handle: Arc::new(Mutex::new(None)),
            subscription_lifecycle: Arc::new(Mutex::new(())),
        })
    }

    /// Resolve ALT keys from a caller-owned cache before parsing. A cache miss
    /// returns an error and skips the transaction, never emits placeholder keys.
    pub fn with_address_lookup_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&VersionedTransaction, u64) -> crate::common::AnyResult<LoadedAddresses>
            + Send
            + Sync
            + 'static,
    {
        self.address_lookup_resolver = Some(Arc::new(resolver));
        self
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
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;

        let queue = Arc::new(ArrayQueue::new(100_000));
        let queue_clone = Arc::clone(&queue);

        let endpoint = self.endpoint.clone();
        let config = self.config.clone();
        let resolver = self.address_lookup_resolver.clone();

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
                    resolver.as_deref(),
                )
                .await
                {
                    Ok(_) => {
                        log::warn!("ShredStream ended cleanly - reconnecting in {}ms", delay);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay.max(1))).await;
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
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;

        let endpoint = self.endpoint.clone();
        let config = self.config.clone();
        let resolver = self.address_lookup_resolver.clone();
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
                    resolver.as_deref(),
                )
                .await
                {
                    Ok(_) => {
                        log::warn!("ShredStream ended cleanly - reconnecting in {}ms", delay);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay.max(1))).await;
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
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;
    }

    async fn stop_without_lifecycle_lock(&self) {
        if let Some(handle) = self.subscription_handle.lock().await.take() {
            handle.abort();
            let _ = handle.await;
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
        resolver: Option<&AddressLookupResolver>,
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
                    Self::process_entry_with_resolver(
                        entry,
                        event_type_filter,
                        &sink,
                        &mut events,
                        resolver,
                    );
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
        resolver: Option<&AddressLookupResolver>,
    ) -> Result<(), String> {
        Self::stream_events(
            endpoint,
            config,
            event_type_filter,
            EventSink::Callback(callback.as_ref()),
            resolver,
        )
        .await
    }

    #[cfg(test)]
    fn process_entry(
        entry: Entry,
        filter: Option<&EventTypeFilter>,
        sink: &EventSink<'_>,
        events: &mut Vec<DexEvent>,
    ) {
        Self::process_entry_with_resolver(entry, filter, sink, events, None);
    }

    /// 处理单个 Entry 消息
    #[inline]
    fn process_entry_with_resolver(
        entry: Entry,
        event_type_filter: Option<&EventTypeFilter>,
        sink: &EventSink<'_>,
        events: &mut Vec<DexEvent>,
        resolver: Option<&AddressLookupResolver>,
    ) {
        let slot = entry.slot;
        let recv_us = now_micros();

        // 反序列化 Entry 数据
        let entries = match wincode::deserialize::<Vec<SolanaEntry>>(&entry.entries) {
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
                if transaction.sanitize().is_err() {
                    log::debug!("Ignoring unsanitized ShredStream transaction");
                    tx_index += 1;
                    continue;
                }
                events.clear();
                Self::process_transaction(
                    transaction,
                    slot,
                    recv_us,
                    tx_index,
                    event_type_filter,
                    events,
                    sink,
                    resolver,
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
        resolver: Option<&AddressLookupResolver>,
    ) {
        Self::parse_transaction_events(
            transaction,
            slot,
            recv_us,
            tx_index,
            event_type_filter,
            events,
            resolver,
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
        resolver: Option<&AddressLookupResolver>,
    ) {
        let Some(&signature) = transaction.signatures.first() else {
            return;
        };
        if transaction.message.address_table_lookups().is_some_and(|lookups| !lookups.is_empty()) {
            if let Some(resolver) = resolver {
                let result = resolver(transaction, slot).and_then(|loaded| {
                    super::pump_ix::parse_transaction_dex_events_with_loaded_addresses(
                        transaction,
                        &loaded.writable,
                        &loaded.readonly,
                        signature,
                        slot,
                        tx_index,
                        recv_us,
                        event_type_filter,
                        events,
                    )?;
                    Ok(())
                });
                if let Err(error) = result {
                    log::debug!(
                        "Skipping unresolved ALT transaction {signature} at {slot}: {error}"
                    );
                }
                return;
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
                    num_readonly_unsigned_accounts: 1,
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
        let entry = Entry { slot: 42, entries: wincode::serialize(&entries).unwrap() };
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

    #[test]
    fn wincode_decodes_solana4_v1_entries() {
        let entries = vec![SolanaEntry {
            num_hashes: 1,
            hash: Hash::new_unique(),
            transactions: vec![VersionedTransaction {
                signatures: vec![Signature::from([9; 64])],
                message: VersionedMessage::V1(solana_sdk::message::v1::Message {
                    header: MessageHeader { num_required_signatures: 1, ..Default::default() },
                    config: solana_sdk::message::v1::TransactionConfig::empty()
                        .with_compute_unit_limit(250_000)
                        .with_priority_fee(1_500),
                    lifetime_specifier: Hash::new_unique(),
                    account_keys: vec![Pubkey::new_unique()],
                    instructions: Vec::new(),
                }),
            }],
        }];
        let bytes = wincode::serialize(&entries).expect("serialize V1 Entry");
        let decoded: Vec<SolanaEntry> = wincode::deserialize(&bytes).expect("decode V1 Entry");

        assert_eq!(decoded, entries);
    }

    #[test]
    fn process_entry_rejects_unsanitized_transactions() {
        let mut transaction = pumpfun_create_tx();
        let VersionedMessage::V0(message) = &mut transaction.message else {
            panic!("expected V0 fixture");
        };
        message.instructions[0].program_id_index = u8::MAX;
        let entries = vec![SolanaEntry {
            num_hashes: 1,
            hash: Hash::default(),
            transactions: vec![transaction],
        }];
        let entry = Entry { slot: 42, entries: wincode::serialize(&entries).unwrap() };
        let count = AtomicUsize::new(0);
        let mut events = Vec::with_capacity(4);

        ShredStreamClient::process_entry(
            entry,
            None,
            &EventSink::Callback(&|_| {
                count.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            &mut events,
        );

        assert_eq!(count.load(AtomicOrdering::Relaxed), 0);
    }
    #[test]
    fn cached_alt_routes_reach_queue_and_callback_without_network() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/shred_routes/reported.json"))
                .unwrap();
        let f = fixtures
            .as_array()
            .unwrap()
            .iter()
            .find(|f| !f["loaded_writable"].as_array().unwrap().is_empty())
            .unwrap();
        let tx: VersionedTransaction =
            wincode::deserialize_exact(&STANDARD.decode(f["wire"].as_str().unwrap()).unwrap())
                .unwrap();
        let loaded = LoadedAddresses {
            writable: f["loaded_writable"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().parse().unwrap())
                .collect(),
            readonly: f["loaded_readonly"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().parse().unwrap())
                .collect(),
        };
        let slot = f["slot"].as_u64().unwrap();
        let signature = tx.signatures[0];
        let calls = Arc::new(AtomicUsize::new(0));
        let resolver_calls = Arc::clone(&calls);
        let resolver = move |transaction: &VersionedTransaction,
                             at: u64|
              -> crate::common::AnyResult<LoadedAddresses> {
            assert_eq!(transaction.signatures[0], signature);
            assert_eq!(at, slot);
            resolver_calls.fetch_add(1, AtomicOrdering::Relaxed);
            Ok(loaded.clone())
        };
        let entry = || Entry {
            slot,
            entries: wincode::serialize(&vec![SolanaEntry {
                num_hashes: 1,
                hash: Hash::default(),
                transactions: vec![tx.clone()],
            }])
            .unwrap(),
        };
        let filter = EventTypeFilter::include_only(vec![crate::grpc::EventType::PumpFunTrade]);
        let queue = Arc::new(ArrayQueue::new(4));
        let mut events = Vec::new();
        ShredStreamClient::process_entry_with_resolver(
            entry(),
            Some(&filter),
            &EventSink::Queue(&queue),
            &mut events,
            Some(&resolver),
        );
        assert!(matches!(queue.pop(), Some(DexEvent::PumpRouteIntent(_))));
        let callback_count = AtomicUsize::new(0);
        ShredStreamClient::process_entry_with_resolver(
            entry(),
            Some(&filter),
            &EventSink::Callback(&|event| {
                assert!(matches!(event, DexEvent::PumpRouteIntent(_)));
                assert_eq!(event.metadata().slot, slot);
                callback_count.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            &mut events,
            Some(&resolver),
        );
        assert_eq!(callback_count.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(calls.load(AtomicOrdering::Relaxed), 2);
        // Without a cache, skip the unresolved route rather than emit zero pubkeys.
        ShredStreamClient::process_entry_with_resolver(
            entry(),
            None,
            &EventSink::Queue(&queue),
            &mut events,
            None,
        );
        assert!(queue.pop().is_none());
        let miss =
            |_: &VersionedTransaction, _: u64| -> crate::common::AnyResult<LoadedAddresses> {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "ALT cache miss").into())
            };
        ShredStreamClient::process_entry_with_resolver(
            entry(),
            None,
            &EventSink::Queue(&queue),
            &mut events,
            Some(&miss),
        );
        assert!(queue.pop().is_none());
        let wrong_count =
            |_: &VersionedTransaction, _: u64| -> crate::common::AnyResult<LoadedAddresses> {
                Ok(LoadedAddresses::default())
            };
        ShredStreamClient::process_entry_with_resolver(
            entry(),
            None,
            &EventSink::Queue(&queue),
            &mut events,
            Some(&wrong_count),
        );
        assert!(queue.pop().is_none());
    }
}
