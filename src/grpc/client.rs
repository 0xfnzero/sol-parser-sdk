//! Yellowstone gRPC 客户端 - 超低延迟 DEX 事件订阅
//!
//! 支持多种事件输出模式：
//! - Unordered: 10-20μs 极低延迟
//! - MicroBatch: 50-200μs 微批次有序
//! - StreamingOrdered: 0.1-5ms 流式有序
//! - Ordered: 1-50ms 完全有序

use super::buffers::{MicroBatchBuffer, SlotBuffer};
use super::subscribe_builder::build_subscribe_request_with_event_filter;
use super::types::*;
use crate::core::{now_micros, EventMetadata}; // 导入高性能时钟
use crate::instr::read_pubkey_fast;
use crate::logs::timestamp_to_microseconds;
use crate::DexEvent;
use crossbeam_queue::ArrayQueue;
use futures::{SinkExt, StreamExt};
use log::error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
// Note: ClientTlsConfig moved to yellowstone_grpc_client in newer versions
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::prelude::*;

static GRPC_DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);

#[inline]
fn push_queue(queue: &ArrayQueue<DexEvent>, event: DexEvent) -> bool {
    if queue.push(event).is_err() {
        let dropped = GRPC_DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
        if dropped <= 10 || dropped.is_power_of_two() {
            log::warn!(
                target: "sol_parser_sdk::grpc",
                "gRPC event queue is full; dropped event count={dropped}"
            );
        }
        false
    } else {
        true
    }
}

/// Counters are local to this client and shared by its clones. A changed revision
/// means cached state may have gaps; discard queued updates and rebuild state.
/// This reports transport continuity, not fork finality or snapshot completeness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrpcSubscriptionStatus {
    pub connected: bool,
    pub generation: u64,
    pub continuity_revision: u64,
    pub disconnects: u64,
    pub dropped_events: u64,
}

#[derive(Default)]
struct SubscriptionHealth(std::sync::Mutex<GrpcSubscriptionStatus>);
impl SubscriptionHealth {
    fn update(&self, f: impl FnOnce(&mut GrpcSubscriptionStatus)) {
        let mut status = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut status);
    }
    fn connected(&self) {
        self.update(|s| {
            s.generation += 1;
            s.continuity_revision += 1;
            s.connected = true;
        });
    }
    fn disconnected(&self) {
        self.update(|s| {
            if s.connected {
                s.connected = false;
                s.disconnects += 1;
                s.continuity_revision += 1;
            }
        });
    }
    fn invalidate(&self) {
        self.update(|s| s.continuity_revision += 1);
    }
    fn dropped(&self) {
        self.update(|s| {
            s.dropped_events += 1;
            s.continuity_revision += 1;
        });
    }
    fn snapshot(&self) -> GrpcSubscriptionStatus {
        *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// ==================== YellowstoneGrpc 客户端 ====================

#[derive(Clone)]
struct SubscriptionFilters {
    transactions: Vec<TransactionFilter>,
    accounts: Vec<AccountFilter>,
    events: Option<EventTypeFilter>,
}

/// Back off failed connection attempts, but reset after any established stream.
struct ReconnectBackoff {
    base_ms: u64,
    next_ms: u64,
}
impl ReconnectBackoff {
    fn new(retry_delay_ms: u64) -> Self {
        let base_ms = retry_delay_ms.clamp(1, 60_000);
        Self { base_ms, next_ms: base_ms }
    }
    fn next_delay(&mut self, established_stream: bool) -> Duration {
        if established_stream {
            self.next_ms = self.base_ms;
        }
        let delay = self.next_ms;
        self.next_ms = self.next_ms.saturating_mul(2).min(60_000);
        Duration::from_millis(delay)
    }
}

#[derive(Clone)]
pub struct YellowstoneGrpc {
    endpoint: String,
    token: Option<String>,
    config: ClientConfig,
    control_tx: Arc<Mutex<Option<mpsc::Sender<SubscribeRequest>>>>,
    subscription_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    subscription_lifecycle: Arc<Mutex<()>>,
    stop_signal: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    health: Arc<SubscriptionHealth>,
    subscription_filters: Arc<Mutex<Option<SubscriptionFilters>>>,
}

impl YellowstoneGrpc {
    /// Sample before and after consuming a batch. A gap/reconnect/overflow must
    /// invalidate dependent account caches; reconnect does not backfill updates.
    pub fn subscription_status(&self) -> GrpcSubscriptionStatus {
        self.health.snapshot()
    }

    #[inline]
    fn push_queue(&self, queue: &ArrayQueue<DexEvent>, event: DexEvent) {
        if !push_queue(queue, event) {
            self.health.dropped();
        }
    }

    pub fn new(
        endpoint: String,
        token: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        crate::warmup::warmup_parser();
        Ok(Self {
            endpoint,
            token,
            config: ClientConfig::default(),
            control_tx: Arc::new(Mutex::new(None)),
            subscription_handle: Arc::new(Mutex::new(None)),
            subscription_lifecycle: Arc::new(Mutex::new(())),
            stop_signal: Arc::new(Mutex::new(None)),
            health: Arc::new(SubscriptionHealth::default()),
            subscription_filters: Arc::new(Mutex::new(None)),
        })
    }

    pub fn new_with_config(
        endpoint: String,
        token: Option<String>,
        config: ClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        crate::warmup::warmup_parser();
        Ok(Self {
            endpoint,
            token,
            config,
            control_tx: Arc::new(Mutex::new(None)),
            subscription_handle: Arc::new(Mutex::new(None)),
            subscription_lifecycle: Arc::new(Mutex::new(())),
            stop_signal: Arc::new(Mutex::new(None)),
            health: Arc::new(SubscriptionHealth::default()),
            subscription_filters: Arc::new(Mutex::new(None)),
        })
    }

    /// 订阅 DEX 事件（自动重连）
    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;

        *self.subscription_filters.lock().await = Some(SubscriptionFilters {
            transactions: transaction_filters,
            accounts: account_filters,
            events: event_type_filter,
        });
        let queue = Arc::new(ArrayQueue::new(self.config.buffer_size.max(1)));
        let queue_clone = Arc::clone(&queue);
        let self_clone = self.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        *self.stop_signal.lock().await = Some(Arc::clone(&stop_signal));

        let handle = tokio::spawn(async move {
            let mut backoff = ReconnectBackoff::new(self_clone.config.retry_delay_ms);
            loop {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }

                // Reconnect uses the latest accepted filters, not the initial ones.
                let filters = self_clone.subscription_filters.lock().await.clone();
                let Some(filters) = filters else { break };
                let generation = self_clone.subscription_status().generation;
                let result = self_clone
                    .stream_events(
                        &filters.transactions,
                        &filters.accounts,
                        &filters.events,
                        &queue_clone,
                    )
                    .await;
                self_clone.health.disconnected();
                let delay =
                    backoff.next_delay(self_clone.subscription_status().generation != generation);
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        if stop_signal.load(Ordering::SeqCst) {
                            break;
                        }
                        error!("Grpc error: {} - retry in {}ms", e, delay.as_millis());
                    }
                }

                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(delay).await;
            }
        });

        *self.subscription_handle.lock().await = Some(handle);
        Ok(queue)
    }

    /// 动态更新订阅过滤器
    pub async fn update_subscription(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Serialize updates with stop/resubscribe and retain the original event policy.
        let _lifecycle = self.subscription_lifecycle.lock().await;
        let sender = self.control_tx.lock().await.as_ref().ok_or("No active subscription")?.clone();
        let mut desired = self.subscription_filters.lock().await;
        let current = desired.as_ref().ok_or("No active subscription filters")?;
        let next = SubscriptionFilters {
            transactions: transaction_filters,
            accounts: account_filters,
            events: current.events.clone(),
        };
        let request = build_subscribe_request_with_event_filter(
            &next.transactions,
            &next.accounts,
            next.events.as_ref(),
            CommitmentLevel::Processed,
        );
        // Invalidate before the provider can start emitting the changed account set.
        self.health.invalidate();
        // Do not hold the lifecycle lock while waiting for a full control queue:
        // otherwise a stalled provider could prevent stop() from acquiring it.
        sender.try_send(request).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => "Subscription update queue is full; retry later",
            mpsc::error::TrySendError::Closed(_) => "No active subscription",
        })?;
        *desired = Some(next);
        Ok(())
    }

    pub async fn stop(&self) {
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;
    }

    async fn stop_without_lifecycle_lock(&self) {
        self.health.disconnected();
        if let Some(stop_signal) = self.stop_signal.lock().await.take() {
            stop_signal.store(true, Ordering::SeqCst);
        }
        self.control_tx.lock().await.take();
        let handle = self.subscription_handle.lock().await.take();
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
        // A connection may have completed while cancellation was in flight.
        self.health.disconnected();
        self.subscription_filters.lock().await.take();
    }

    // ==================== 核心事件流处理 ====================

    async fn stream_events(
        &self,
        tx_filters: &[TransactionFilter],
        acc_filters: &[AccountFilter],
        event_filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) -> Result<(), String> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        // 构建客户端
        let mut builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())
            .map_err(|e| e.to_string())?
            .x_token(self.token.clone())
            .map_err(|e| e.to_string())?
            .max_decoding_message_size(1024 * 1024 * 1024);

        if self.config.connection_timeout_ms > 0 {
            builder =
                builder.connect_timeout(Duration::from_millis(self.config.connection_timeout_ms));
        }
        if self.config.enable_tls {
            builder = builder
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| e.to_string())?;
        }

        let mut client = builder.connect().await.map_err(|e| e.to_string())?;
        let request = build_subscribe_request_with_event_filter(
            tx_filters,
            acc_filters,
            event_filter.as_ref(),
            CommitmentLevel::Processed,
        );

        let (subscribe_tx, mut stream) =
            client.subscribe_with_request(Some(request)).await.map_err(|e| e.to_string())?;

        self.health.connected();
        self.print_mode_info();

        // 设置控制通道
        let (control_tx, mut control_rx) = mpsc::channel::<SubscribeRequest>(100);
        *self.control_tx.lock().await = Some(control_tx);
        let subscribe_tx = Arc::new(Mutex::new(subscribe_tx));

        // 初始化缓冲区
        let mut slot_buffer = SlotBuffer::new();
        let mut micro_batch = MicroBatchBuffer::new();
        let mut last_slot = 0u64;

        let order_mode = self.config.order_mode;
        let timeout_ms = self.config.order_timeout_ms;
        let batch_us = self.config.micro_batch_us;
        let check_interval = match order_mode {
            OrderMode::MicroBatch => Duration::from_micros(batch_us.max(1)),
            _ => Duration::from_millis((timeout_ms / 2).max(1)),
        };
        let mut next_check = Instant::now() + check_interval;

        loop {
            tokio::select! {
                msg = stream.next() => {
                    match msg {
                        Some(Ok(update)) => {
                            // Geyser 会周期性下发 ping；必须在同一 subscribe 流上回写 SubscribeRequest.ping，否则公共节点 / LB 可能 RST_STREAM。
                            if matches!(
                                update.update_oneof.as_ref(),
                                Some(subscribe_update::UpdateOneof::Ping(_))
                            ) {
                                if let Err(e) = subscribe_tx
                                    .lock()
                                    .await
                                    .send(SubscribeRequest {
                                        ping: Some(SubscribeRequestPing { id: 1 }),
                                        ..Default::default()
                                    })
                                    .await
                                {
                                    self.control_tx.lock().await.take();
                                    return Err(e.to_string());
                                }
                                continue;
                            }
                            self.handle_update(
                                update, order_mode, event_filter, queue,
                                &mut slot_buffer, &mut micro_batch, &mut last_slot, batch_us
                            );
                        }
                        Some(Err(e)) => {
                            error!("Grpc Stream error: {:?}", e);
                            self.flush_on_disconnect(
                                order_mode,
                                &mut slot_buffer,
                                &mut micro_batch,
                                queue,
                            );
                            self.control_tx.lock().await.take();
                            return Err(e.to_string());
                        }
                        None => {
                            self.flush_on_disconnect(
                                order_mode,
                                &mut slot_buffer,
                                &mut micro_batch,
                                queue,
                            );
                            self.control_tx.lock().await.take();
                            return Ok(());
                        }
                    }
                }
                Some(req) = control_rx.recv() => {
                    if let Err(e) = subscribe_tx.lock().await.send(req).await {
                        self.control_tx.lock().await.take();
                        return Err(e.to_string());
                    }
                }
                // Unordered emits immediately and has no ordering buffer to flush.
                _ = tokio::time::sleep_until(next_check), if order_mode != OrderMode::Unordered => {
                    self.check_timeout(
                        order_mode,
                        &mut slot_buffer,
                        &mut micro_batch,
                        queue,
                        timeout_ms,
                        batch_us,
                        &mut next_check,
                        check_interval,
                    );
                }
            }
        }
    }

    fn print_mode_info(&self) {
        match self.config.order_mode {
            OrderMode::Unordered => println!("✅ Unordered Mode (10-20μs)"),
            OrderMode::Ordered => {
                println!("✅ Ordered Mode (timeout={}ms)", self.config.order_timeout_ms)
            }
            OrderMode::StreamingOrdered => {
                println!("✅ StreamingOrdered Mode (timeout={}ms)", self.config.order_timeout_ms)
            }
            OrderMode::MicroBatch => {
                println!("✅ MicroBatch Mode (window={}μs)", self.config.micro_batch_us)
            }
        }
    }

    #[inline]
    fn check_timeout(
        &self,
        mode: OrderMode,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        queue: &Arc<ArrayQueue<DexEvent>>,
        timeout_ms: u64,
        batch_us: u64,
        next_check: &mut Instant,
        interval: Duration,
    ) {
        if Instant::now() < *next_check {
            return;
        }
        *next_check = Instant::now() + interval;

        match mode {
            OrderMode::Ordered => {
                if slot_buf.should_timeout(timeout_ms) {
                    for e in slot_buf.flush_all() {
                        self.push_queue(queue, e);
                    }
                }
            }
            OrderMode::StreamingOrdered => {
                if slot_buf.should_timeout(timeout_ms) {
                    for e in slot_buf.flush_streaming_timeout() {
                        self.push_queue(queue, e);
                    }
                }
            }
            OrderMode::MicroBatch => {
                // Periodic flush for MicroBatch mode
                let now_us = get_timestamp_us();
                if micro_buf.should_flush(now_us, batch_us) {
                    for e in micro_buf.flush() {
                        self.push_queue(queue, e);
                    }
                }
            }
            OrderMode::Unordered => {}
        }
    }

    fn flush_on_disconnect(
        &self,
        mode: OrderMode,
        buffer: &mut SlotBuffer,
        micro_batch: &mut MicroBatchBuffer,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) {
        let events = match mode {
            OrderMode::Ordered => buffer.flush_all(),
            OrderMode::StreamingOrdered => buffer.flush_streaming_timeout(),
            OrderMode::MicroBatch => micro_batch.flush(),
            OrderMode::Unordered => Vec::new(),
        };
        for event in events {
            self.push_queue(queue, event);
        }
    }

    #[inline]
    fn handle_update(
        &self,
        update_msg: SubscribeUpdate,
        mode: OrderMode,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        last_slot: &mut u64,
        batch_us: u64,
    ) {
        let created_at = update_msg.created_at.unwrap_or_default();
        let block_time_us = timestamp_to_microseconds(created_at.seconds, created_at.nanos) as i64;
        let grpc_recv_us = get_timestamp_us();

        let Some(update) = update_msg.update_oneof else { return };

        match update {
            subscribe_update::UpdateOneof::Transaction(tx) => {
                self.handle_transaction(
                    tx,
                    mode,
                    filter,
                    queue,
                    slot_buf,
                    micro_buf,
                    last_slot,
                    batch_us,
                    grpc_recv_us,
                    block_time_us,
                );
            }
            subscribe_update::UpdateOneof::Account(acc) => {
                self.handle_account(acc, filter, queue, grpc_recv_us, block_time_us);
            }
            subscribe_update::UpdateOneof::BlockMeta(block_meta) => {
                self.handle_block_meta(block_meta, filter, queue, grpc_recv_us, block_time_us);
            }
            _ => {}
        }
    }

    #[inline]
    fn handle_transaction(
        &self,
        tx: SubscribeUpdateTransaction,
        mode: OrderMode,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        last_slot: &mut u64,
        batch_us: u64,
        grpc_us: i64,
        block_us: i64,
    ) {
        let slot = tx.slot;

        match mode {
            OrderMode::Unordered => {
                for e in crate::grpc::parse_subscribe_update_transaction_low_latency(
                    &tx,
                    grpc_us,
                    Some(block_us),
                    filter.as_ref(),
                ) {
                    self.push_queue(queue, e);
                }
            }
            OrderMode::Ordered => {
                if slot > *last_slot && *last_slot > 0 {
                    for e in slot_buf.flush_before(slot) {
                        self.push_queue(queue, e);
                    }
                }
                *last_slot = slot;
                for (idx, e) in
                    parse_transaction_to_vec(&tx, grpc_us, Some(block_us), filter.as_ref())
                {
                    slot_buf.push(slot, idx, e);
                }
            }
            OrderMode::StreamingOrdered => {
                for (idx, e) in
                    parse_transaction_to_vec(&tx, grpc_us, Some(block_us), filter.as_ref())
                {
                    for evt in slot_buf.push_streaming(slot, idx, e) {
                        self.push_queue(queue, evt);
                    }
                }
            }
            OrderMode::MicroBatch => {
                for (idx, e) in
                    parse_transaction_to_vec(&tx, grpc_us, Some(block_us), filter.as_ref())
                {
                    if micro_buf.push(slot, idx, e, grpc_us, batch_us) {
                        for evt in micro_buf.flush() {
                            self.push_queue(queue, evt);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn handle_account(
        &self,
        acc: SubscribeUpdateAccount,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        grpc_us: i64,
        block_us: i64,
    ) {
        let Some(info) = acc.account else { return };
        // Malformed identities must not become default keys in subscription caches.
        if info.pubkey.len() != 32 || info.owner.len() != 32 {
            self.health.dropped();
            return;
        }
        let data = crate::accounts::AccountData {
            pubkey: read_pubkey_fast(&info.pubkey),
            executable: info.executable,
            lamports: info.lamports,
            owner: read_pubkey_fast(&info.owner),
            rent_epoch: info.rent_epoch,
            data: info.data,
        };
        let meta = EventMetadata {
            signature: Default::default(),
            slot: acc.slot,
            tx_index: 0,
            block_time_us: block_us,
            grpc_recv_us: grpc_us,
            recent_blockhash: None,
        };
        // Parse while borrowing bytes, then move them into the opt-in raw event.
        // This also avoids a full account-data clone when both outputs are requested.
        let normalized = if data.lamports == 0 {
            // A closed account can retain its former bytes in a Geyser update.
            // Preserve the tombstone in raw output, never emit it as live state.
            None
        } else {
            crate::accounts::parse_account_unified(&data, meta.clone(), filter.as_ref())
        };
        if filter.as_ref().is_some_and(|f| {
            f.include_only
                .as_ref()
                .is_some_and(|types| types.contains(&crate::grpc::EventType::AccountRawSnapshot))
        }) {
            self.push_queue(
                queue,
                DexEvent::RawAccountSnapshot(Box::new(
                    crate::accounts::liquidity_snapshot::RawAccountSnapshotEvent {
                        metadata: meta,
                        account: data,
                        write_version: info.write_version,
                        is_startup: acc.is_startup,
                    },
                )),
            );
        }
        if let Some(e) = normalized {
            self.push_queue(queue, e);
        }
    }

    #[inline]
    fn handle_block_meta(
        &self,
        block_meta: SubscribeUpdateBlockMeta,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        grpc_us: i64,
        fallback_block_us: i64,
    ) {
        let block_time_us = block_meta
            .block_time
            .as_ref()
            .map(|t| t.timestamp.saturating_mul(1_000_000))
            .unwrap_or(fallback_block_us);
        let event = DexEvent::BlockMeta(crate::core::events::BlockMetaEvent {
            metadata: EventMetadata {
                signature: Default::default(),
                slot: block_meta.slot,
                tx_index: 0,
                block_time_us,
                grpc_recv_us: grpc_us,
                recent_blockhash: (!block_meta.blockhash.is_empty())
                    .then_some(block_meta.blockhash),
            },
        });
        if filter.as_ref().map(|f| f.should_include_dex_event(&event)).unwrap_or(true) {
            self.push_queue(queue, event);
        }
    }
}

// ==================== 辅助函数 ====================

/// 获取当前时间戳（微秒）
///
/// 使用高性能时钟，避免系统调用开销
///
/// # 性能优势
/// - 旧实现：使用 libc::clock_gettime，每次调用约 1-2μs
/// - 新实现：使用高性能时钟，每次调用约 10-50ns
/// - 性能提升：20-100 倍
#[inline(always)]
fn get_timestamp_us() -> i64 {
    now_micros()
}

// ==================== 交易解析 ====================

#[inline]
fn parse_transaction_to_vec(
    tx: &SubscribeUpdateTransaction,
    grpc_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
) -> Vec<(u64, DexEvent)> {
    let idx = tx.transaction.as_ref().map(|t| t.index).unwrap_or(0);
    crate::grpc::parse_subscribe_update_transaction_low_latency(tx, grpc_us, block_us, filter)
        .into_iter()
        .map(|event| (idx, event))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_share_retained_bytes_emit_only_raw_tombstone() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        let mut bytes = vec![0; 145];
        bytes[..8]
            .copy_from_slice(crate::accounts::raydium_cpmm::discriminators::CREATOR_FEE_SHARE);
        bytes[73..81].copy_from_slice(&300_000u64.to_le_bytes());
        for raw in [false, true] {
            let queue = Arc::new(ArrayQueue::new(4));
            let mut types = vec![EventType::AccountRaydiumCpmmCreatorFeeShare];
            if raw {
                types.push(EventType::AccountRawSnapshot);
            }
            let filter = Some(EventTypeFilter::include_only(types));
            let mut update = SubscribeUpdateAccount {
                account: Some(SubscribeUpdateAccountInfo {
                    pubkey: solana_sdk::pubkey::Pubkey::new_unique().to_bytes().to_vec(),
                    owner: crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID.to_bytes().to_vec(),
                    data: bytes.clone(),
                    lamports: 1,
                    write_version: 1,
                    ..Default::default()
                }),
                slot: 100,
                ..Default::default()
            };
            grpc.handle_account(update.clone(), &filter, &queue, 1, 2);
            if raw {
                assert!(matches!(queue.pop(), Some(DexEvent::RawAccountSnapshot(_))));
            }
            assert!(matches!(queue.pop(), Some(DexEvent::RaydiumCpmmCreatorFeeShareAccount(_))));
            update.slot = 101;
            let info = update.account.as_mut().unwrap();
            info.lamports = 0;
            info.write_version = 2;
            grpc.handle_account(update, &filter, &queue, 3, 4);
            if raw {
                let DexEvent::RawAccountSnapshot(closed) = queue.pop().unwrap() else {
                    panic!("tombstone")
                };
                assert_eq!(closed.account.lamports, 0);
                assert_eq!(closed.account.data, bytes);
                assert_eq!((closed.metadata.slot, closed.write_version), (101, 2));
            }
            assert!(queue.is_empty(), "closed account was decoded as live state");
            assert_eq!(grpc.subscription_status().dropped_events, 0);
        }
    }
    #[test]
    fn reconnect_backoff_honors_low_latency_config_and_resets_after_recovery() {
        let base = ClientConfig::low_latency().retry_delay_ms;
        let mut backoff = ReconnectBackoff::new(base);
        assert_eq!(backoff.next_delay(false), Duration::from_millis(base));
        assert_eq!(backoff.next_delay(false), Duration::from_millis(base * 2));
        for _ in 0..20 {
            backoff.next_delay(false);
        }
        assert_eq!(backoff.next_delay(false), Duration::from_secs(60));
        assert_eq!(backoff.next_delay(true), Duration::from_millis(base));
        assert_eq!(backoff.next_delay(true), Duration::from_millis(base));
        assert_eq!(ReconnectBackoff::new(0).next_delay(false), Duration::from_millis(1));
        assert_eq!(ReconnectBackoff::new(u64::MAX).next_delay(false), Duration::from_secs(60));
    }
    #[test]
    fn raw_snapshot_moves_original_buffer_and_can_coexist_with_normalized_output() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        for both in [false, true] {
            let mut data = vec![0; 236];
            data[..8].copy_from_slice(crate::accounts::raydium_cpmm::discriminators::AMM_CONFIG);
            let original = data.as_ptr();
            let mut types = vec![EventType::AccountRawSnapshot];
            if both {
                types.push(EventType::AccountRaydiumCpmmAmmConfig);
            }
            let queue = Arc::new(ArrayQueue::new(4));
            grpc.handle_account(
                SubscribeUpdateAccount {
                    account: Some(SubscribeUpdateAccountInfo {
                        pubkey: solana_sdk::pubkey::Pubkey::new_unique().to_bytes().to_vec(),
                        owner: crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID
                            .to_bytes()
                            .to_vec(),
                        data,
                        lamports: 1,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                &Some(EventTypeFilter::include_only(types)),
                &queue,
                1,
                2,
            );
            let DexEvent::RawAccountSnapshot(raw) = queue.pop().unwrap() else { panic!("raw") };
            assert_eq!(raw.account.data.as_ptr(), original);
            if both {
                assert!(matches!(queue.pop(), Some(DexEvent::RaydiumCpmmAmmConfigAccount(_))));
            }
            assert!(queue.is_empty());
        }
    }
    #[test]
    fn malformed_account_identities_invalidate_continuity_without_default_key_events() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        let queue = Arc::new(ArrayQueue::new(4));
        for (key_len, owner_len) in [(31, 32), (32, 33), (0, 32)] {
            grpc.handle_account(
                SubscribeUpdateAccount {
                    account: Some(SubscribeUpdateAccountInfo {
                        pubkey: vec![1; key_len],
                        owner: vec![1; owner_len],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                &Some(EventTypeFilter::include_only(vec![EventType::AccountRawSnapshot])),
                &queue,
                1,
                2,
            );
        }
        assert!(queue.is_empty());
        assert_eq!(grpc.subscription_status().dropped_events, 3);
        assert_eq!(grpc.subscription_status().continuity_revision, 3);
    }
    #[test]
    fn raw_snapshots_are_opt_in_and_preserve_version_and_closures() {
        let key = solana_sdk::pubkey::Pubkey::new_unique();
        let account = SubscribeUpdateAccount {
            account: Some(SubscribeUpdateAccountInfo {
                pubkey: key.to_bytes().to_vec(),
                owner: solana_sdk::pubkey::Pubkey::default().to_bytes().to_vec(),
                lamports: 0,
                data: Vec::new(),
                write_version: 42,
                ..Default::default()
            }),
            slot: 123,
            is_startup: true,
        };
        let queue = Arc::new(ArrayQueue::new(4));
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        grpc.handle_account(account.clone(), &None, &queue, 1, 2);
        assert!(queue.pop().is_none());
        let filter =
            Some(EventTypeFilter::include_only(vec![crate::grpc::EventType::AccountRawSnapshot]));
        grpc.handle_account(account, &filter, &queue, 1, 2);
        let DexEvent::RawAccountSnapshot(event) = queue.pop().unwrap() else {
            panic!("raw snapshot")
        };
        assert_eq!(event.write_version, 42);
        assert_eq!(event.metadata.slot, 123);
        assert_eq!(event.account.pubkey, key);
        assert_eq!(event.account.lamports, 0);
        assert!(event.account.data.is_empty() && event.is_startup);
        assert!(queue.pop().is_none());
    }

    fn test_event(slot: u64) -> DexEvent {
        DexEvent::BlockMeta(crate::core::events::BlockMetaEvent {
            metadata: EventMetadata { slot, ..Default::default() },
        })
    }

    #[tokio::test]
    async fn stop_clears_subscription_state_and_aborts_handle() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_string(), None).unwrap();
        let (tx, _rx) = mpsc::channel::<SubscribeRequest>(1);
        let handle = tokio::spawn(async {
            std::future::pending::<()>().await;
        });

        grpc.health.connected();
        let stop_signal = Arc::new(AtomicBool::new(false));
        *grpc.control_tx.lock().await = Some(tx);
        *grpc.subscription_handle.lock().await = Some(handle);
        *grpc.stop_signal.lock().await = Some(Arc::clone(&stop_signal));

        grpc.stop().await;

        assert!(stop_signal.load(Ordering::SeqCst));
        assert!(grpc.stop_signal.lock().await.is_none());
        assert!(grpc.control_tx.lock().await.is_none());
        assert!(grpc.subscription_handle.lock().await.is_none());
        assert!(!grpc.subscription_status().connected);
        assert_eq!(grpc.subscription_status().disconnects, 1);
    }

    #[test]
    fn micro_batch_is_flushed_on_disconnect() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_string(), None).unwrap();
        let queue = Arc::new(ArrayQueue::new(2));
        let mut slot_buffer = SlotBuffer::new();
        let mut micro_batch = MicroBatchBuffer::new();
        assert!(!micro_batch.push(7, 0, test_event(7), 10, 100));

        grpc.flush_on_disconnect(OrderMode::MicroBatch, &mut slot_buffer, &mut micro_batch, &queue);

        assert!(micro_batch.is_empty());
        assert!(matches!(queue.pop(), Some(DexEvent::BlockMeta(_))));
    }

    #[test]
    fn full_queue_increments_drop_counter() {
        let queue = ArrayQueue::new(1);
        push_queue(&queue, test_event(1));
        let before = GRPC_DROPPED_EVENTS.load(Ordering::Relaxed);
        push_queue(&queue, test_event(2));
        assert!(GRPC_DROPPED_EVENTS.load(Ordering::Relaxed) >= before + 1);
    }
    #[test]
    fn continuity_status_is_shared_by_clones_and_loss_is_client_local() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        let clone = grpc.clone();
        let other = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        grpc.health.connected();
        let initial = grpc.subscription_status();
        let queue = ArrayQueue::new(1);
        grpc.push_queue(&queue, test_event(1));
        assert_eq!(clone.subscription_status(), initial);
        grpc.push_queue(&queue, test_event(2));
        let loss = clone.subscription_status();
        assert_eq!(loss.dropped_events, 1);
        assert!(loss.continuity_revision > initial.continuity_revision);
        assert_eq!(other.subscription_status(), GrpcSubscriptionStatus::default());
        grpc.health.disconnected();
        let disconnected = clone.subscription_status();
        assert!(!disconnected.connected);
        assert_eq!(disconnected.disconnects, 1);
        grpc.health.disconnected();
        assert_eq!(clone.subscription_status(), disconnected);
        grpc.health.connected();
        let reconnected = clone.subscription_status();
        assert!(reconnected.connected);
        assert_eq!(reconnected.generation, 2);
        assert!(reconnected.continuity_revision > disconnected.continuity_revision);
    }

    #[test]
    fn raw_account_overflow_marks_subscription_state_incomplete() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        let queue = Arc::new(ArrayQueue::new(1));
        queue.push(test_event(1)).unwrap();
        let account = SubscribeUpdateAccount {
            account: Some(SubscribeUpdateAccountInfo {
                pubkey: solana_sdk::pubkey::Pubkey::new_unique().to_bytes().to_vec(),
                owner: solana_sdk::pubkey::Pubkey::default().to_bytes().to_vec(),
                ..Default::default()
            }),
            ..Default::default()
        };
        grpc.handle_account(
            account,
            &Some(EventTypeFilter::include_only(vec![EventType::AccountRawSnapshot])),
            &queue,
            0,
            0,
        );
        assert_eq!(grpc.subscription_status().dropped_events, 1);
        assert_eq!(queue.len(), 1);
    }
    #[tokio::test]
    async fn full_update_queue_does_not_block_stop_or_replace_reconnect_filters() {
        let grpc = YellowstoneGrpc::new("http://127.0.0.1:1".to_owned(), None).unwrap();
        *grpc.subscription_filters.lock().await = Some(SubscriptionFilters {
            transactions: vec![],
            accounts: vec![AccountFilter::new().add_account("old")],
            events: Some(EventTypeFilter::include_only(vec![EventType::BlockMeta])),
        });
        let (tx, _rx) = mpsc::channel(1);
        tx.try_send(SubscribeRequest::default()).unwrap();
        *grpc.control_tx.lock().await = Some(tx);
        let error = tokio::time::timeout(
            Duration::from_secs(1),
            grpc.update_subscription(vec![], vec![AccountFilter::new().add_account("new")]),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert!(error.to_string().contains("queue is full"));
        assert_eq!(
            grpc.subscription_filters.lock().await.as_ref().unwrap().accounts[0].account,
            vec!["old"]
        );
        tokio::time::timeout(Duration::from_secs(1), grpc.stop()).await.unwrap();
        assert!(grpc.subscription_filters.lock().await.is_none());
    }
}
