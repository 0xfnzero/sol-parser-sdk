use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use super::constants::*;

/// Event type enumeration
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Transaction = 0,
    Account = 1,
    BlockMeta = 2,
}

/// Compatibility alias
pub type MetricsEventType = EventType;

impl EventType {
    #[inline]
    const fn as_index(self) -> usize {
        self as usize
    }

    const fn name(self) -> &'static str {
        match self {
            EventType::Transaction => "TX",
            EventType::Account => "Account",
            EventType::BlockMeta => "Block Meta",
        }
    }

    // Compatibility constants
    pub const TX: EventType = EventType::Transaction;
}

/// High-performance atomic event metrics
#[derive(Debug)]
struct AtomicEventMetrics {
    process_count: AtomicU64,
    events_processed: AtomicU64,
    events_in_window: AtomicU64,
    window_start_nanos: AtomicU64,
    // Processing time statistics per event type
    processing_stats: AtomicProcessingTimeStats,
}

impl AtomicEventMetrics {
    fn new(now_nanos: u64) -> Self {
        Self {
            process_count: AtomicU64::new(0),
            events_processed: AtomicU64::new(0),
            events_in_window: AtomicU64::new(0),
            window_start_nanos: AtomicU64::new(now_nanos),
            processing_stats: AtomicProcessingTimeStats::new(),
        }
    }

    /// Atomically increment process count
    #[inline]
    fn add_process_count(&self) {
        self.process_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Atomically increment event processing count
    #[inline]
    fn add_events_processed(&self, count: u64) {
        self.events_processed.fetch_add(count, Ordering::Relaxed);
        self.events_in_window.fetch_add(count, Ordering::Relaxed);
    }

    /// Get current count (non-blocking)
    #[inline]
    fn get_counts(&self) -> (u64, u64, u64) {
        (
            self.process_count.load(Ordering::Relaxed),
            self.events_processed.load(Ordering::Relaxed),
            self.events_in_window.load(Ordering::Relaxed),
        )
    }

    /// Reset window count
    #[inline]
    fn reset_window(&self, new_start_nanos: u64) {
        self.events_in_window.store(0, Ordering::Relaxed);
        self.window_start_nanos.store(new_start_nanos, Ordering::Relaxed);
    }

    #[inline]
    fn get_window_start(&self) -> u64 {
        self.window_start_nanos.load(Ordering::Relaxed)
    }

    /// Get processing time statistics for this event type
    #[inline]
    fn get_processing_stats(&self) -> ProcessingTimeStats {
        self.processing_stats.get_stats()
    }

    /// Update processing time statistics for this event type
    #[inline]
    fn update_processing_stats(&self, time_us: f64, event_count: u64) {
        self.processing_stats.update(time_us, event_count);
    }
}

/// High-performance atomic processing time statistics
#[derive(Debug)]
struct AtomicProcessingTimeStats {
    min_time_bits: AtomicU64,
    max_time_bits: AtomicU64,
    min_time_timestamp_nanos: AtomicU64, // Timestamp of min value update (nanoseconds)
    max_time_timestamp_nanos: AtomicU64, // Timestamp of max value update (nanoseconds)
    total_time_us: AtomicU64,            // Store integer part of microseconds
    total_events: AtomicU64,
}

impl AtomicProcessingTimeStats {
    fn new() -> Self {
        let now_nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                as u64;

        Self {
            min_time_bits: AtomicU64::new(f64::INFINITY.to_bits()),
            max_time_bits: AtomicU64::new(0),
            min_time_timestamp_nanos: AtomicU64::new(now_nanos),
            max_time_timestamp_nanos: AtomicU64::new(now_nanos),
            total_time_us: AtomicU64::new(0),
            total_events: AtomicU64::new(0),
        }
    }

    /// Atomically update processing time statistics
    #[inline]
    fn update(&self, time_us: f64, event_count: u64) {
        let time_bits = time_us.to_bits();
        let now_nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                as u64;

        // Update minimum value, check time difference and reset if over 10 seconds
        let mut current_min = self.min_time_bits.load(Ordering::Relaxed);
        let min_timestamp = self.min_time_timestamp_nanos.load(Ordering::Relaxed);

        // Check if min value timestamp exceeds 10 seconds (10_000_000_000 nanoseconds)
        let min_time_diff_nanos = now_nanos.saturating_sub(min_timestamp);
        if min_time_diff_nanos > 10_000_000_000 {
            // Over 10 seconds, reset min value
            self.min_time_bits.store(f64::INFINITY.to_bits(), Ordering::Relaxed);
            self.min_time_timestamp_nanos.store(now_nanos, Ordering::Relaxed);
            current_min = f64::INFINITY.to_bits();
        }

        // If current time is less than min value, update min value and timestamp
        while time_bits < current_min {
            match self.min_time_bits.compare_exchange_weak(
                current_min,
                time_bits,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Successfully updated min value, also update timestamp
                    self.min_time_timestamp_nanos.store(now_nanos, Ordering::Relaxed);
                    break;
                }
                Err(x) => current_min = x,
            }
        }

        // Update maximum value, check time difference and reset if over 10 seconds
        let mut current_max = self.max_time_bits.load(Ordering::Relaxed);
        let max_timestamp = self.max_time_timestamp_nanos.load(Ordering::Relaxed);

        // Check if max value timestamp exceeds 10 seconds (10_000_000_000 nanoseconds)
        let time_diff_nanos = now_nanos.saturating_sub(max_timestamp);
        if time_diff_nanos > 10_000_000_000 {
            // Over 10 seconds, reset max value
            self.max_time_bits.store(0, Ordering::Relaxed);
            self.max_time_timestamp_nanos.store(now_nanos, Ordering::Relaxed);
            current_max = 0;
        }

        // If current time is greater than max value, update max value and timestamp
        while time_bits > current_max {
            match self.max_time_bits.compare_exchange_weak(
                current_max,
                time_bits,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Successfully updated max value, also update timestamp
                    self.max_time_timestamp_nanos.store(now_nanos, Ordering::Relaxed);
                    break;
                }
                Err(x) => current_max = x,
            }
        }

        // Update cumulative values (convert microseconds to integers to avoid floating point accumulation issues)
        let total_time_us_int = (time_us * event_count as f64) as u64;
        self.total_time_us.fetch_add(total_time_us_int, Ordering::Relaxed);
        self.total_events.fetch_add(event_count, Ordering::Relaxed);
    }

    /// Get statistics (non-blocking)
    #[inline]
    fn get_stats(&self) -> ProcessingTimeStats {
        let min_bits = self.min_time_bits.load(Ordering::Relaxed);
        let max_bits = self.max_time_bits.load(Ordering::Relaxed);
        let total_time_us_int = self.total_time_us.load(Ordering::Relaxed);
        let total_events = self.total_events.load(Ordering::Relaxed);

        let min_time = f64::from_bits(min_bits);
        let max_time = f64::from_bits(max_bits);
        let avg_time =
            if total_events > 0 { total_time_us_int as f64 / total_events as f64 } else { 0.0 };

        ProcessingTimeStats {
            min_us: if min_time == f64::INFINITY { 0.0 } else { min_time },
            max_us: max_time,
            avg_us: avg_time,
        }
    }
}

/// Processing time statistics result
#[derive(Debug, Clone)]
pub struct ProcessingTimeStats {
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
}

/// Event metrics snapshot
#[derive(Debug, Clone)]
pub struct EventMetricsSnapshot {
    pub process_count: u64,
    pub events_processed: u64,
    pub processing_stats: ProcessingTimeStats,
}

/// Compatibility structure - complete performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub uptime: std::time::Duration,
    pub tx_metrics: EventMetricsSnapshot,
    pub account_metrics: EventMetricsSnapshot,
    pub block_meta_metrics: EventMetricsSnapshot,
    pub processing_stats: ProcessingTimeStats,
    pub dropped_events_count: u64,
}

impl PerformanceMetrics {
    /// Create default performance metrics (compatibility method)
    pub fn new() -> Self {
        let default_stats = ProcessingTimeStats { min_us: 0.0, max_us: 0.0, avg_us: 0.0 };
        let default_metrics = EventMetricsSnapshot {
            process_count: 0,
            events_processed: 0,
            processing_stats: default_stats.clone(),
        };

        Self {
            uptime: std::time::Duration::ZERO,
            tx_metrics: default_metrics.clone(),
            account_metrics: default_metrics.clone(),
            block_meta_metrics: default_metrics,
            processing_stats: default_stats,
            dropped_events_count: 0,
        }
    }
}

/// High-performance metrics system
#[derive(Debug)]
pub struct HighPerformanceMetrics {
    start_nanos: u64,
    event_metrics: [AtomicEventMetrics; 3],
    processing_stats: AtomicProcessingTimeStats,
    // 丢弃事件指标
    dropped_events_count: AtomicU64,
}

impl HighPerformanceMetrics {
    fn new() -> Self {
        let now_nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                as u64;

        Self {
            start_nanos: now_nanos,
            event_metrics: [
                AtomicEventMetrics::new(now_nanos),
                AtomicEventMetrics::new(now_nanos),
                AtomicEventMetrics::new(now_nanos),
            ],
            processing_stats: AtomicProcessingTimeStats::new(),
            // 初始化丢弃事件指标
            dropped_events_count: AtomicU64::new(0),
        }
    }

    /// 获取运行时长（秒）
    #[inline]
    pub fn get_uptime_seconds(&self) -> f64 {
        let now_nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                as u64;
        (now_nanos - self.start_nanos) as f64 / 1_000_000_000.0
    }

    /// 获取事件指标快照
    #[inline]
    pub fn get_event_metrics(&self, event_type: EventType) -> EventMetricsSnapshot {
        let index = event_type.as_index();
        let (process_count, events_processed, _) = self.event_metrics[index].get_counts();
        let processing_stats = self.event_metrics[index].get_processing_stats();

        EventMetricsSnapshot { process_count, events_processed, processing_stats }
    }

    /// 获取处理时间统计
    #[inline]
    pub fn get_processing_stats(&self) -> ProcessingTimeStats {
        self.processing_stats.get_stats()
    }

    /// 获取丢弃事件计数
    #[inline]
    pub fn get_dropped_events_count(&self) -> u64 {
        self.dropped_events_count.load(Ordering::Relaxed)
    }

    /// 更新窗口指标（后台任务调用）
    fn update_window_metrics(&self, event_type: EventType, window_duration_nanos: u64) {
        let now_nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                as u64;

        let index = event_type.as_index();
        let event_metric = &self.event_metrics[index];

        let window_start = event_metric.get_window_start();
        if now_nanos.saturating_sub(window_start) >= window_duration_nanos {
            event_metric.reset_window(now_nanos);
        }
    }
}

/// 高性能指标管理器
pub struct MetricsManager {
    metrics: Arc<HighPerformanceMetrics>,
    enable_metrics: bool,
    stream_name: String,
    background_task_running: AtomicBool,
}

impl MetricsManager {
    /// 创建新的指标管理器
    pub fn new(enable_metrics: bool, stream_name: String) -> Self {
        let manager = Self {
            metrics: Arc::new(HighPerformanceMetrics::new()),
            enable_metrics,
            stream_name,
            background_task_running: AtomicBool::new(false),
        };

        // 启动后台任务
        manager.start_background_tasks();
        manager
    }

    /// 启动后台任务
    fn start_background_tasks(&self) {
        if self
            .background_task_running
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            if !self.enable_metrics {
                return;
            }

            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

                loop {
                    interval.tick().await;

                    let window_duration_nanos = DEFAULT_METRICS_WINDOW_SECONDS * 1_000_000_000;

                    // 更新所有事件类型的窗口指标
                    metrics.update_window_metrics(EventType::Transaction, window_duration_nanos);
                    metrics.update_window_metrics(EventType::Account, window_duration_nanos);
                    metrics.update_window_metrics(EventType::BlockMeta, window_duration_nanos);
                }
            });
        }
    }

    /// 记录处理次数（非阻塞）
    #[inline]
    pub fn record_process(&self, event_type: EventType) {
        if self.enable_metrics {
            self.metrics.event_metrics[event_type.as_index()].add_process_count();
        }
    }

    /// 记录事件处理（非阻塞）
    #[inline]
    pub fn record_events(&self, event_type: EventType, count: u64, processing_time_us: f64) {
        if !self.enable_metrics {
            return;
        }

        let index = event_type.as_index();

        // 原子更新事件计数
        self.metrics.event_metrics[index].add_events_processed(count);

        // 原子更新该事件类型的处理时间统计
        self.metrics.event_metrics[index].update_processing_stats(processing_time_us, count);

        // 保持全局处理时间统计的兼容性
        self.metrics.processing_stats.update(processing_time_us, count);
    }

    /// 记录慢处理操作
    #[inline]
    pub fn log_slow_processing(&self, processing_time_us: f64, event_count: usize) {
        if processing_time_us > SLOW_PROCESSING_THRESHOLD_US {
            log::debug!(
                "{} slow processing: {:.2}us for {} events",
                self.stream_name,
                processing_time_us,
                event_count,
            );
        }
    }

    /// 获取运行时长
    pub fn get_uptime(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.metrics.get_uptime_seconds())
    }

    /// 获取事件指标
    pub fn get_event_metrics(&self, event_type: EventType) -> EventMetricsSnapshot {
        self.metrics.get_event_metrics(event_type)
    }

    /// 获取处理时间统计
    pub fn get_processing_stats(&self) -> ProcessingTimeStats {
        self.metrics.get_processing_stats()
    }

    /// 获取丢弃事件计数
    pub fn get_dropped_events_count(&self) -> u64 {
        self.metrics.get_dropped_events_count()
    }

    /// 打印性能指标（非阻塞）
    pub fn print_metrics(&self) {
        println!("\n📊 {} Performance Metrics", self.stream_name);
        println!("   Run Time: {:?}", self.get_uptime());

        // 打印丢弃事件指标
        let dropped_count = self.get_dropped_events_count();
        if dropped_count > 0 {
            println!("\n⚠️  Dropped Events: {}", dropped_count);
        }

        // 打印事件指标表格（包含处理时间统计）
        println!("┌─────────────┬──────────────┬──────────────────┬─────────────┬─────────────┬─────────────┐");
        println!("│ Event Type  │ Process Count│ Events Processed │ Avg Time(μs)│ Min 10s(μs) │ Max 10s(μs) │");
        println!("├─────────────┼──────────────┼──────────────────┼─────────────┼─────────────┼─────────────┤");

        for event_type in [EventType::Transaction, EventType::Account, EventType::BlockMeta] {
            let metrics = self.get_event_metrics(event_type);
            println!(
                "│ {:11} │ {:12} │ {:16} │ {:9.2}   │ {:9.2}   │ {:9.2}   │",
                event_type.name(),
                metrics.process_count,
                metrics.events_processed,
                metrics.processing_stats.avg_us,
                metrics.processing_stats.min_us,
                metrics.processing_stats.max_us
            );
        }

        println!("└─────────────┴──────────────┴──────────────────┴─────────────┴─────────────┴─────────────┘");
        println!();
    }

    /// 启动自动性能监控任务
    pub async fn start_auto_monitoring(&self) -> Option<tokio::task::JoinHandle<()>> {
        if !self.enable_metrics {
            return None;
        }

        let manager = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                DEFAULT_METRICS_PRINT_INTERVAL_SECONDS,
            ));
            loop {
                interval.tick().await;
                manager.print_metrics();
            }
        });
        Some(handle)
    }

    // === 兼容性方法 ===

    /// 兼容性构造函数
    pub fn new_with_metrics(
        _metrics: Arc<std::sync::RwLock<PerformanceMetrics>>,
        enable_metrics: bool,
        stream_name: String,
    ) -> Self {
        Self::new(enable_metrics, stream_name)
    }

    /// 获取完整的性能指标（兼容性方法）
    pub fn get_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            uptime: self.get_uptime(),
            tx_metrics: self.get_event_metrics(EventType::Transaction),
            account_metrics: self.get_event_metrics(EventType::Account),
            block_meta_metrics: self.get_event_metrics(EventType::BlockMeta),
            processing_stats: self.get_processing_stats(),
            dropped_events_count: self.metrics.get_dropped_events_count(),
        }
    }

    /// 兼容性方法 - 添加交易处理计数
    #[inline]
    pub fn add_tx_process_count(&self) {
        self.record_process(EventType::Transaction);
    }

    /// 兼容性方法 - 添加账户处理计数
    #[inline]
    pub fn add_account_process_count(&self) {
        self.record_process(EventType::Account);
    }

    /// 兼容性方法 - 添加区块元数据处理计数
    #[inline]
    pub fn add_block_meta_process_count(&self) {
        self.record_process(EventType::BlockMeta);
    }

    /// 兼容性方法 - 更新指标
    #[inline]
    pub fn update_metrics(
        &self,
        event_type: MetricsEventType,
        events_processed: u64,
        processing_time_us: f64,
    ) {
        self.record_events(event_type, events_processed, processing_time_us);
        self.log_slow_processing(processing_time_us, events_processed as usize);
    }

    /// 增加丢弃事件计数
    #[inline]
    pub fn increment_dropped_events(&self) {
        if !self.enable_metrics {
            return;
        }

        // 原子地增加丢弃事件计数
        let new_count = self.metrics.dropped_events_count.fetch_add(1, Ordering::Relaxed) + 1;

        // 每丢弃1000个事件记录一次警告日志
        if new_count % 1000 == 0 {
            log::debug!("{} dropped events count reached: {}", self.stream_name, new_count);
        }
    }

    /// 批量增加丢弃事件计数
    #[inline]
    pub fn increment_dropped_events_by(&self, count: u64) {
        if !self.enable_metrics || count == 0 {
            return;
        }

        // 原子地增加丢弃事件计数
        let new_count =
            self.metrics.dropped_events_count.fetch_add(count, Ordering::Relaxed) + count;

        // 记录批量丢弃事件的日志
        if count > 1 {
            log::debug!(
                "{} dropped batch of {} events, total dropped: {}",
                self.stream_name,
                count,
                new_count
            );
        }

        // 每丢弃1000个事件记录一次警告日志
        if new_count % 1000 == 0 || (new_count / 1000) != ((new_count - count) / 1000) {
            log::debug!("{} dropped events count reached: {}", self.stream_name, new_count);
        }
    }
}

impl Clone for MetricsManager {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
            enable_metrics: self.enable_metrics,
            stream_name: self.stream_name.clone(),
            background_task_running: AtomicBool::new(false), // 新实例不自动启动后台任务
        }
    }
}
