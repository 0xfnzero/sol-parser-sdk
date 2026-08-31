//! 事件缓冲区模块 - 用于有序模式下的事件排序和批次处理
//!
//! 提供多种缓冲策略：
//! - `SlotBuffer`: 按 slot 缓冲，支持 Ordered 和 StreamingOrdered 模式
//! - `MicroBatchBuffer`: 微秒级时间窗口批次，用于 MicroBatch 模式

use crate::DexEvent;
use std::collections::{BTreeMap, HashMap};
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
}

impl SlotBuffer {
    #[inline]
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            current_slot: 0,
            last_flush_time: Some(Instant::now()),
            streaming_watermarks: HashMap::new(),
        }
    }

    /// 添加事件到缓冲区
    #[inline]
    pub fn push(&mut self, slot: u64, tx_index: u64, event: DexEvent) {
        self.slots.entry(slot).or_default().push((tx_index, event));
        if slot > self.current_slot {
            self.current_slot = slot;
        }
    }

    /// 输出所有小于 current_slot 的事件
    pub fn flush_before(&mut self, current_slot: u64) -> Vec<DexEvent> {
        let slots_to_flush: Vec<u64> =
            self.slots.keys().filter(|&&s| s < current_slot).copied().collect();

        let mut result = Vec::with_capacity(slots_to_flush.len() * 4);
        for slot in slots_to_flush {
            if let Some(mut events) = self.slots.remove(&slot) {
                events.sort_unstable_by_key(|(idx, _)| *idx);
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
                events.sort_unstable_by_key(|(idx, _)| *idx);
                result.extend(events.into_iter().map(|(_, e)| e));
            }
        }

        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }

    /// 检查是否超时
    #[inline]
    pub fn should_timeout(&self, timeout_ms: u64) -> bool {
        self.last_flush_time
            .map(|t| !self.slots.is_empty() && t.elapsed().as_millis() as u64 > timeout_ms)
            .unwrap_or(false)
    }

    /// Streaming release: add event and return releasable continuous sequence
    /// NOTE: This mode assumes tx_index is continuous (0,1,2,3...)
    /// For filtered event streams where tx_index may not be continuous, use MicroBatch mode instead
    pub fn push_streaming(&mut self, slot: u64, tx_index: u64, event: DexEvent) -> Vec<DexEvent> {
        let mut result = Vec::new();

        // When new slot arrives, release ALL events from previous slots (sorted)
        if slot > self.current_slot && self.current_slot > 0 {
            let old_slots: Vec<u64> = self.slots.keys().filter(|&&s| s < slot).copied().collect();
            for old_slot in old_slots {
                if let Some(mut events) = self.slots.remove(&old_slot) {
                    events.sort_unstable_by_key(|(idx, _)| *idx);
                    result.extend(events.into_iter().map(|(_, e)| e));
                }
                self.streaming_watermarks.remove(&old_slot);
            }
        }

        if slot > self.current_slot {
            self.current_slot = slot;
        }

        // Check if this is the expected tx_index (continuous sequence)
        let next_expected = *self.streaming_watermarks.get(&slot).unwrap_or(&0);

        if tx_index == next_expected {
            // Expected index: release immediately
            result.push(event);
            let mut watermark = next_expected + 1;

            // Release buffered consecutive events
            if let Some(buffered) = self.slots.get_mut(&slot) {
                buffered.sort_unstable_by_key(|(idx, _)| *idx);
                while let Some(pos) = buffered.iter().position(|(idx, _)| *idx == watermark) {
                    result.push(buffered.remove(pos).1);
                    watermark += 1;
                }
            }
            self.streaming_watermarks.insert(slot, watermark);
        } else if tx_index > next_expected {
            // Future index: buffer it
            self.slots.entry(slot).or_default().push((tx_index, event));
        }
        // tx_index < next_expected: duplicate event, ignore

        if !result.is_empty() {
            self.last_flush_time = Some(Instant::now());
        }
        result
    }

    /// 流式模式超时释放
    pub fn flush_streaming_timeout(&mut self) -> Vec<DexEvent> {
        let mut result = Vec::new();
        for (slot, mut events) in std::mem::take(&mut self.slots) {
            events.sort_unstable_by_key(|(idx, _)| *idx);
            result.extend(events.into_iter().map(|(_, e)| e));
            self.streaming_watermarks.remove(&slot);
        }
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
