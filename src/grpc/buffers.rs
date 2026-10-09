//! 事件缓冲区模块 - 用于有序模式下的事件排序和批次处理
//!
//! 提供多种缓冲策略：
//! - `SlotBuffer`: 按 slot 缓冲，支持 Ordered 和 StreamingOrdered 模式
//! - `MicroBatchBuffer`: 微秒级时间窗口批次，用于 MicroBatch 模式

use crate::DexEvent;
use std::collections::{BTreeMap, HashMap, HashSet};
use tokio::time::Instant;

// ==================== SlotBuffer ====================

/// Slot 缓冲区，用于有序模式下缓存同一 slot 的事件
#[derive(Default)]
pub struct SlotBuffer {
    /// slot -> Vec<(tx_index, event)>
    slots: BTreeMap<u64, Vec<(u64, DexEvent)>>,
    /// 当前处理的最大 slot
    current_slot: u64,
    /// 上次输出时间
    last_flush_time: Option<Instant>,
    /// 流式模式：每个 slot 已释放的最大连续 tx_index
    streaming_watermarks: HashMap<u64, u64>,
    streaming_pending_indexes: HashSet<u64>,
    ordered_watermark: Option<(u64, u64)>,
    ordered_late_events: u64,
}

impl SlotBuffer {
    #[inline]
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            current_slot: 0,
            last_flush_time: Some(Instant::now()),
            streaming_watermarks: HashMap::new(),
            streaming_pending_indexes: HashSet::new(),
            ordered_watermark: None,
            ordered_late_events: 0,
        }
    }

    /// 添加事件到缓冲区
    #[inline]
    pub fn push(&mut self, slot: u64, tx_index: u64, event: DexEvent) {
        if slot < self.current_slot || self.ordered_watermark.is_some_and(|last| (slot, tx_index) <= last) {
            self.ordered_late_events = self.ordered_late_events.saturating_add(1);
            // Preserve exact drop accounting without synchronous log I/O on every replay.
            let dropped = self.ordered_late_events;
            if dropped <= 10 || dropped.is_power_of_two() {
                log::warn!("Ordered continuity break: dropped late event ({slot},{tx_index}); total={dropped}");
            }
            return;
        }
        self.slots.entry(slot).or_default().push((tx_index, event));
        if slot > self.current_slot {
            self.current_slot = slot;
        }
    }

    /// 输出所有小于 current_slot 的事件
    pub fn flush_before(&mut self, current_slot: u64) -> Vec<DexEvent> {
        self.current_slot = self.current_slot.max(current_slot);
        let slots_to_flush: Vec<u64> =
            self.slots.keys().filter(|&&s| s < current_slot).copied().collect();

        let mut result = Vec::with_capacity(slots_to_flush.len() * 4);
        for slot in slots_to_flush {
            if let Some(mut events) = self.slots.remove(&slot) {
                events.sort_by_key(|(idx, _)| *idx);
                if let Some((index, _)) = events.last() {
                    self.ordered_watermark = Some((slot, *index));
                }
                result.extend(events.into_iter().map(|(_, e)| e));
            }
        }

        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }

    /// 超时强制输出所有缓冲事件
    pub fn flush_all(&mut self) -> Vec<DexEvent> {
        let all_slots: Vec<u64> = self.slots.keys().copied().collect();
        let mut result = Vec::with_capacity(all_slots.len() * 4);

        for slot in all_slots {
            if let Some(mut events) = self.slots.remove(&slot) {
                events.sort_by_key(|(idx, _)| *idx);
                if let Some((index, _)) = events.last() {
                    self.ordered_watermark = Some((slot, *index));
                }
                result.extend(events.into_iter().map(|(_, e)| e));
            }
        }

        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }

    /// Late Ordered events are dropped with a warning to preserve monotonic output.
    pub fn ordered_late_events(&self) -> u64 { self.ordered_late_events }

    /// 检查是否超时
    #[inline]
    pub fn should_timeout(&self, timeout_ms: u64) -> bool {
        self.last_flush_time
            .map(|t| !self.slots.is_empty() && t.elapsed().as_millis() as u64 > timeout_ms)
            .unwrap_or(false)
    }

    /// Single-event compatibility API. Multi-event transactions must use
    /// `push_streaming_batch` so their index advances only once.
    pub fn push_streaming(&mut self, slot: u64, tx_index: u64, event: DexEvent) -> Vec<DexEvent> {
        self.push_streaming_batch(slot, tx_index, [event])
    }

    /// Release complete transaction batches in contiguous transaction-index order.
    /// Filtered streams with index gaps should use MicroBatch or the timeout fallback.
    /// Once a slot is flushed by a newer slot, late batches for it are discarded.
    pub fn push_streaming_batch(
        &mut self,
        slot: u64,
        tx_index: u64,
        events: impl IntoIterator<Item = DexEvent>,
    ) -> Vec<DexEvent> {
        // A streaming watermark must represent the next index without u64 overflow.
        if slot < self.current_slot || tx_index == u64::MAX {
            return Vec::new();
        }
        let mut events = events.into_iter().peekable();
        if events.peek().is_none() {
            return Vec::new();
        }
        let mut result = Vec::new();
        if slot > self.current_slot {
            result = self.flush_before(slot);
            // Immediately emitted slots have no buffer entry, but still own a watermark.
            self.streaming_watermarks.retain(|old_slot, _| *old_slot >= slot);
            self.streaming_pending_indexes.clear();
            self.current_slot = slot;
        }

        let next_expected = *self.streaming_watermarks.get(&slot).unwrap_or(&0);
        if tx_index == next_expected {
            result.extend(events);
            let mut watermark = next_expected + 1;
            if let Some(buffered) = self.slots.get_mut(&slot) {
                buffered.sort_by_key(|(idx, _)| *idx);
                let mut released = 0;
                while released < buffered.len() && buffered[released].0 == watermark {
                    while released < buffered.len() && buffered[released].0 == watermark {
                        released += 1;
                    }
                    self.streaming_pending_indexes.remove(&watermark);
                    watermark += 1;
                }
                result.extend(buffered.drain(..released).map(|(_, event)| event));
                if buffered.is_empty() {
                    self.slots.remove(&slot);
                }
            }
            self.streaming_watermarks.insert(slot, watermark);
        } else if tx_index > next_expected && self.streaming_pending_indexes.insert(tx_index) {
            self.slots.entry(slot).or_default().extend(events.map(|event| (tx_index, event)));
        }
        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }

    /// 流式模式超时释放
    pub fn flush_streaming_timeout(&mut self) -> Vec<DexEvent> {
        let mut result = Vec::new();
        for (slot, mut events) in std::mem::take(&mut self.slots) {
            events.sort_by_key(|(idx, _)| *idx);
            if slot == self.current_slot {
                if let Some((index, _)) = events.last() {
                    let next = index.saturating_add(1);
                    let watermark = self.streaming_watermarks.entry(slot).or_default();
                    *watermark = (*watermark).max(next);
                }
            }
            result.extend(events.into_iter().map(|(_, e)| e));
        }
        self.streaming_pending_indexes.clear();
        self.streaming_watermarks.retain(|slot, _| *slot == self.current_slot);
        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }
}

// ==================== MicroBatchBuffer ====================

/// 微批次缓冲区，用于 MicroBatch 模式
pub struct MicroBatchBuffer {
    /// 当前窗口内的事件: (slot, tx_index, event)
    events: Vec<(u64, u64, DexEvent)>,
    /// 窗口开始时间（微秒）
    window_start_us: i64,
}

impl MicroBatchBuffer {
    #[inline]
    pub fn new() -> Self {
        Self { events: Vec::with_capacity(64), window_start_us: 0 }
    }

    /// 添加事件到窗口，返回是否需要刷新
    #[inline]
    pub fn push(
        &mut self,
        slot: u64,
        tx_index: u64,
        event: DexEvent,
        now_us: i64,
        window_us: u64,
    ) -> bool {
        if self.events.is_empty() {
            self.window_start_us = now_us;
        }
        self.events.push((slot, tx_index, event));
        (now_us - self.window_start_us) as u64 >= window_us
    }

    /// 刷新窗口，返回排序后的事件
    #[inline]
    pub fn flush(&mut self) -> Vec<DexEvent> {
        if self.events.is_empty() {
            return Vec::new();
        }

        // Stable sort preserves parser order for multiple events from one transaction.
        self.events.sort_by_key(|(slot, tx_index, _)| (*slot, *tx_index));

        let mut result = Vec::with_capacity(self.events.len());
        result.extend(self.events.drain(..).map(|(_, _, event)| event));

        self.window_start_us = 0;
        result
    }

    /// 检查是否需要刷新（窗口超时）
    #[inline]
    pub fn should_flush(&self, now_us: i64, window_us: u64) -> bool {
        !self.events.is_empty() && (now_us - self.window_start_us) as u64 >= window_us
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for MicroBatchBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{BlockMetaEvent, EventMetadata};

    fn event(id: u64) -> DexEvent {
        DexEvent::BlockMeta(BlockMetaEvent {
            metadata: EventMetadata { slot: id, ..Default::default() },
        })
    }
    fn ids(events: Vec<DexEvent>) -> Vec<u64> {
        events.into_iter().map(|event| event.metadata().slot).collect()
    }

    #[test]
    fn streaming_releases_complete_immediate_and_buffered_transactions() {
        let mut buffer = SlotBuffer::new();
        assert!(buffer.push_streaming_batch(42, 1, [event(20), event(21)]).is_empty());
        assert_eq!(
            ids(buffer.push_streaming_batch(42, 0, [event(10), event(11)])),
            [10, 11, 20, 21]
        );
        assert_eq!(ids(buffer.push_streaming_batch(42, 2, [event(30), event(31)])), [30, 31]);
        assert!(buffer.push_streaming_batch(42, 0, [event(99)]).is_empty());
        assert!(buffer.slots.is_empty());
    }

    #[test]
    fn streaming_bounds_watermarks_and_rejects_late_or_replayed_batches() {
        let mut buffer = SlotBuffer::new();
        for slot in 1..=10_000 {
            assert_eq!(buffer.push_streaming_batch(slot, 0, [event(slot)]).len(), 1);
            assert_eq!(buffer.streaming_watermarks.len(), 1);
        }
        assert!(buffer.flush_streaming_timeout().is_empty());
        assert_eq!(buffer.streaming_watermarks.len(), 1);
        for old_slot in 1..10_000 {
            assert!(buffer.push_streaming_batch(old_slot, 0, [event(99)]).is_empty());
        }
        assert_eq!(buffer.streaming_watermarks.len(), 1);
        assert!(buffer.push_streaming_batch(10_000, 0, [event(99)]).is_empty());
    }

    #[test]
    fn streaming_slot_and_timeout_flush_preserve_event_order_within_a_transaction() {
        let mut buffer = SlotBuffer::new();
        assert!(buffer.push_streaming_batch(1, 5, [event(10), event(11)]).is_empty());
        assert_eq!(
            ids(buffer.push_streaming_batch(2, 0, [event(20), event(21)])),
            [10, 11, 20, 21]
        );
        assert!(buffer.push_streaming_batch(2, 3, [event(30), event(31)]).is_empty());
        assert_eq!(ids(buffer.flush_streaming_timeout()), [30, 31]);
        assert_eq!(buffer.streaming_watermarks.len(), 1);
        assert!(buffer.push_streaming_batch(2, 3, [event(99)]).is_empty());
        assert!(buffer.push_streaming_batch(2, 0, [event(99)]).is_empty());
        assert_eq!(ids(buffer.push_streaming_batch(2, 4, [event(40)])), [40]);
    }

    #[test]
    fn repeated_pending_and_old_indexes_remain_bounded() {
        let mut buffer = SlotBuffer::new();
        for _ in 0..10_000 {
            assert!(buffer.push_streaming_batch(42, 2, [event(2)]).is_empty());
        }
        assert_eq!(buffer.slots[&42].len(), 1);
        assert_eq!(buffer.streaming_pending_indexes.len(), 1);
        assert_eq!(ids(buffer.flush_streaming_timeout()), [2]);
        assert!(buffer.slots.is_empty());
        assert!(buffer.streaming_pending_indexes.is_empty());
        assert!(buffer.push_streaming_batch(42, u64::MAX, [event(99)]).is_empty());
        assert!(buffer.flush_streaming_timeout().is_empty());
        assert_eq!(ids(buffer.push_streaming_batch(43, 0, [event(3)])), [3]);
        for _ in 0..10_000 {
            assert!(buffer.push_streaming_batch(42, 2, [event(2)]).is_empty());
        }
        assert_eq!(buffer.streaming_watermarks.len(), 1);
        assert!(buffer.slots.is_empty());
    }

    #[test]
    fn duplicate_buffered_batch_is_not_emitted_twice() {
        let mut buffer = SlotBuffer::new();
        assert!(buffer.push_streaming_batch(42, 1, [event(20), event(21)]).is_empty());
        assert!(buffer.push_streaming_batch(42, 1, [event(20), event(21)]).is_empty());
        assert_eq!(ids(buffer.push_streaming_batch(42, 0, [event(10)])), [10, 20, 21]);
    }
    #[test]
    fn ordered_rejects_closed_slots_and_retains_timeout_watermark() {
        let mut buffer = SlotBuffer::new();
        buffer.push(10, 2, event(1));
        assert_eq!(ids(buffer.flush_before(11)), [1]);
        buffer.push(10, 1, event(99));
        buffer.push(10, 9, event(99));
        buffer.push(11, 0, event(2));
        assert_eq!(ids(buffer.flush_all()), [2]);
        assert_eq!(buffer.ordered_late_events(), 2);

        buffer.push(11, 3, event(3));
        buffer.push(11, 3, event(4));
        assert_eq!(ids(buffer.flush_all()), [3, 4]);
        for index in [0, 1, 2, 3] { buffer.push(11, index, event(99)); }
        buffer.push(11, 4, event(5));
        assert_eq!(ids(buffer.flush_all()), [5]);
        assert_eq!(buffer.ordered_late_events(), 6);
    }

    #[test]
    fn ordered_late_replay_bounds_diagnostics() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Capture;
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        static LOGGER: Capture = Capture;
        impl log::Log for Capture {
            fn enabled(&self, _: &log::Metadata) -> bool { true }
            fn log(&self, record: &log::Record) {
                // Other parallel tests are isolated by this unique slot/index.
                if record.args().to_string().contains("late event (300001,0)") {
                    COUNT.fetch_add(1, Ordering::Relaxed);
                }
            }
            fn flush(&self) {}
        }
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Warn);
        let mut buffer = SlotBuffer::new();
        buffer.push(300001, 1, event(1));
        assert_eq!(ids(buffer.flush_all()), [1]);
        for _ in 0..10000 { buffer.push(300001, 0, event(99)); }
        assert_eq!(buffer.ordered_late_events(), 10000);
        assert_eq!(COUNT.load(Ordering::Relaxed), 20);
        assert!(buffer.slots.is_empty());
        buffer.push(300001, 2, event(2));
        assert_eq!(ids(buffer.flush_all()), [2]);
    }

    #[test]
    fn ordered_watermark_accepts_max_index_without_overflow() {
        let mut buffer = SlotBuffer::new();
        buffer.push(u64::MAX, u64::MAX, event(1));
        assert_eq!(ids(buffer.flush_all()), [1]);
        buffer.push(u64::MAX, u64::MAX, event(99));
        assert!(buffer.flush_all().is_empty());
        assert_eq!(buffer.ordered_late_events(), 1);
    }

}
