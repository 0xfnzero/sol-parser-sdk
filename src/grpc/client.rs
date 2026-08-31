//! Yellowstone gRPC 客户端 - 超低延迟 DEX 事件订阅
//!
//! 支持多种事件输出模式：
//! - Unordered: 10-20μs 极低延迟
//! - MicroBatch: 50-200μs 微批次有序
//! - StreamingOrdered: 0.1-5ms 流式有序
//! - Ordered: 1-50ms 完全有序

use super::buffers::{MicroBatchBuffer, SlotBuffer};
use super::subscribe_builder::{
    build_subscribe_request, build_subscribe_request_with_event_filter,
};
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
fn push_queue(queue: &ArrayQueue<DexEvent>, event: DexEvent) {
    if queue.push(event).is_err() {
        let dropped = GRPC_DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
        if dropped <= 10 || dropped.is_power_of_two() {
            log::warn!(
                target: "sol_parser_sdk::grpc",
                "gRPC event queue is full; dropped event count={dropped}"
            );
        }
    }
}

// ==================== YellowstoneGrpc 客户端 ====================

#[derive(Clone)]
pub struct YellowstoneGrpc {
    endpoint: String,
    token: Option<String>,
    config: ClientConfig,
    control_tx: Arc<Mutex<Option<mpsc::Sender<SubscribeRequest>>>>,
    subscription_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    subscription_lifecycle: Arc<Mutex<()>>,
    stop_signal: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl YellowstoneGrpc {
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

        let queue = Arc::new(ArrayQueue::new(self.config.buffer_size.max(1)));
        let queue_clone = Arc::clone(&queue);
        let self_clone = self.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        *self.stop_signal.lock().await = Some(Arc::clone(&stop_signal));

        let handle = tokio::spawn(async move {
            let mut delay = 1u64;
            loop {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }

                match self_clone
                    .stream_events(
                        &transaction_filters,
                        &account_filters,
                        &event_type_filter,
                        &queue_clone,
                    )
                    .await
                {
                    Ok(_) => delay = 1,
                    Err(e) => {
                        if stop_signal.load(Ordering::SeqCst) {
                            break;
                        }
                        error!("Grpc error: {} - retry in {}s", e, delay);
                    }
                }

                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(delay)).await;
                delay = (delay * 2).min(60);
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
        let sender = self.control_tx.lock().await.as_ref().ok_or("No active subscription")?.clone();

        let request = build_subscribe_request(&transaction_filters, &account_filters);
        sender.send(request).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn stop(&self) {
        let _lifecycle = self.subscription_lifecycle.lock().await;
        self.stop_without_lifecycle_lock().await;
    }

    async fn stop_without_lifecycle_lock(&self) {
        if let Some(stop_signal) = self.stop_signal.lock().await.take() {
            stop_signal.store(true, Ordering::SeqCst);
        }
        self.control_tx.lock().await.take();
        let handle = self.subscription_handle.lock().await.take();
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
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
                _ = tokio::time::sleep_until(next_check) => {
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
                        push_queue(queue, e);
                    }
                }
            }
            OrderMode::StreamingOrdered => {
                if slot_buf.should_timeout(timeout_ms) {
                    for e in slot_buf.flush_streaming_timeout() {
                        push_queue(queue, e);
                    }
                }
            }
            OrderMode::MicroBatch => {
                // Periodic flush for MicroBatch mode
                let now_us = get_timestamp_us();
                if micro_buf.should_flush(now_us, batch_us) {
                    for e in micro_buf.flush() {
                        push_queue(queue, e);
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
            push_queue(queue, event);
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
                Self::handle_account(acc, filter, queue, grpc_recv_us, block_time_us);
            }
            subscribe_update::UpdateOneof::BlockMeta(block_meta) => {
                Self::handle_block_meta(block_meta, filter, queue, grpc_recv_us, block_time_us);
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
                    push_queue(queue, e);
                }
            }
            OrderMode::Ordered => {
                if slot > *last_slot && *last_slot > 0 {
                    for e in slot_buf.flush_before(slot) {
                        push_queue(queue, e);
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
                        push_queue(queue, evt);
                    }
                }
            }
            OrderMode::MicroBatch => {
                for (idx, e) in
                    parse_transaction_to_vec(&tx, grpc_us, Some(block_us), filter.as_ref())
                {
                    if micro_buf.push(slot, idx, e, grpc_us, batch_us) {
                        for evt in micro_buf.flush() {
                            push_queue(queue, evt);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn handle_account(
        acc: SubscribeUpdateAccount,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        grpc_us: i64,
        block_us: i64,
    ) {
        let Some(info) = acc.account else { return };
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
        if let Some(e) = crate::accounts::parse_account_unified(&data, meta, filter.as_ref()) {
            push_queue(queue, e);
        }
    }

    #[inline]
    fn handle_block_meta(
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
            push_queue(queue, event);
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

        let stop_signal = Arc::new(AtomicBool::new(false));
        *grpc.control_tx.lock().await = Some(tx);
        *grpc.subscription_handle.lock().await = Some(handle);
        *grpc.stop_signal.lock().await = Some(Arc::clone(&stop_signal));

        grpc.stop().await;

        assert!(stop_signal.load(Ordering::SeqCst));
        assert!(grpc.stop_signal.lock().await.is_none());
        assert!(grpc.control_tx.lock().await.is_none());
        assert!(grpc.subscription_handle.lock().await.is_none());
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
        assert_eq!(GRPC_DROPPED_EVENTS.load(Ordering::Relaxed), before + 1);
    }
}
