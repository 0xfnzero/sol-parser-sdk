//! 高性能时钟模块
//!
//! 提供微秒级精度的时间戳获取，避免频繁的系统调用开销。
//!
//! ## 设计原理
//!
//! 传统方式使用 `chrono::Utc::now()` 每次都需要系统调用（微秒级开销），
//! 高性能时钟使用单调时钟 `Instant` + 基准时间戳，将开销降低到纳秒级。
//!
//! ## 性能优势
//!
//! - **减少 90%+ 开销**: 从系统调用（1-2μs）降低到内存计算（10-50ns）
//! - **自动校准**: 每 5 分钟自动校准一次，防止时钟漂移
//! - **线程安全**: 使用 `OnceCell` 实现全局单例
//!
//! ## 使用示例
//!
//! ```rust
//! use sol_parser_sdk::core::clock::{now_micros, elapsed_micros_since};
//!
//! // 获取当前时间戳（微秒）
//! let start = now_micros();
//!
//! // ... 执行解析操作 ...
//!
//! // 计算耗时
//! let elapsed = elapsed_micros_since(start);
//! println!("解析耗时: {} μs", elapsed);
//! ```

use std::time::Instant;

/// 高性能时钟管理器
///
/// 使用单调时钟 + 基准时间戳，避免频繁的系统调用
#[derive(Debug)]
pub struct HighPerformanceClock {
    /// 基准时间点（程序启动时的单调时钟时间）
    base_instant: Instant,
    /// 基准时间点对应的 UTC 时间戳（微秒）
    base_timestamp_us: i64,
    /// 上次校准时间（用于检测是否需要重新校准）
    last_calibration: Instant,
    /// 校准间隔（秒）
    calibration_interval_secs: u64,
}

impl HighPerformanceClock {
    /// 创建新的高性能时钟（默认 5 分钟校准一次）
    pub fn new() -> Self {
        Self::new_with_calibration_interval(300)
    }

    /// 创建带自定义校准间隔的高性能时钟
    ///
    /// # 参数
    /// - `calibration_interval_secs`: 校准间隔（秒）
    ///
    /// # 实现细节
    /// 通过多次采样来减少初始化误差，选择延迟最小的样本
    pub fn new_with_calibration_interval(calibration_interval_secs: u64) -> Self {
        // 通过多次采样来减少初始化误差
        let mut best_offset = i64::MAX;
        let mut best_instant = Instant::now();
        let mut best_timestamp = chrono::Utc::now().timestamp_micros();

        // 进行 3 次采样，选择延迟最小的
        for _ in 0..3 {
            let instant_before = Instant::now();
            let timestamp = chrono::Utc::now().timestamp_micros();
            let instant_after = Instant::now();

            let sample_latency = instant_after.duration_since(instant_before).as_nanos() as i64;

            if sample_latency < best_offset {
                best_offset = sample_latency;
                best_instant = instant_before;
                best_timestamp = timestamp;
            }
        }

        Self {
            base_instant: best_instant,
            base_timestamp_us: best_timestamp,
            last_calibration: best_instant,
            calibration_interval_secs,
        }
    }

    /// 获取当前时间戳（微秒）
    ///
    /// 使用单调时钟计算，避免系统调用
    ///
    /// # 性能
    /// - 仅需 10-50ns（纳秒级）
    /// - 相比 `chrono::Utc::now()` 快 20-100 倍
    #[inline(always)]
    pub fn now_micros(&self) -> i64 {
        let elapsed = self.base_instant.elapsed();
        self.base_timestamp_us + elapsed.as_micros() as i64
    }

    /// 获取高精度当前时间戳（微秒），在必要时进行校准
    ///
    /// # 校准机制
    /// 每隔 `calibration_interval_secs` 秒自动校准一次，防止时钟漂移
    pub fn now_micros_with_calibration(&mut self) -> i64 {
        // 检查是否需要重新校准
        if self.last_calibration.elapsed().as_secs() >= self.calibration_interval_secs {
            self.recalibrate();
        }
        self.now_micros()
    }

    /// 重新校准时钟，减少累积漂移
    ///
    /// # 校准策略
    /// - 计算预期 UTC 时间戳（基于单调时钟）
    /// - 与实际 UTC 时间戳对比
    /// - 如果漂移超过 1ms，重新设置基准
    fn recalibrate(&mut self) {
        let current_monotonic = Instant::now();
        let current_utc = chrono::Utc::now().timestamp_micros();

        // 计算预期的 UTC 时间戳（基于单调时钟）
        let expected_utc = self.base_timestamp_us
            + current_monotonic.duration_since(self.base_instant).as_micros() as i64;

        // 计算漂移量
        let drift_us = current_utc - expected_utc;

        // 如果漂移超过 1 毫秒，进行校准
        if drift_us.abs() > 1000 {
            self.base_instant = current_monotonic;
            self.base_timestamp_us = current_utc;
        }

        self.last_calibration = current_monotonic;
    }

    /// 计算从指定时间戳到现在的消耗时间（微秒）
    ///
    /// # 参数
    /// - `start_timestamp_us`: 起始时间戳（微秒）
    ///
    /// # 返回
    /// 消耗时间（微秒）
    #[inline(always)]
    pub fn elapsed_micros_since(&self, start_timestamp_us: i64) -> i64 {
        self.now_micros() - start_timestamp_us
    }

    /// 获取高精度纳秒时间戳
    #[inline(always)]
    pub fn now_nanos(&self) -> i128 {
        let elapsed = self.base_instant.elapsed();
        (self.base_timestamp_us as i128 * 1000) + elapsed.as_nanos() as i128
    }

    /// 重置时钟（强制重新初始化）
    pub fn reset(&mut self) {
        *self = Self::new_with_calibration_interval(self.calibration_interval_secs);
    }
}

impl Default for HighPerformanceClock {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局高性能时钟实例
static HIGH_PERF_CLOCK: once_cell::sync::OnceCell<HighPerformanceClock> =
    once_cell::sync::OnceCell::new();

/// 获取当前时间戳（微秒）
///
/// 使用全局高性能时钟实例，避免系统调用开销
///
/// # 性能
/// - 首次调用：初始化时钟（约 100μs）
/// - 后续调用：仅需 10-50ns
///
/// # 示例
/// ```rust
/// use sol_parser_sdk::core::clock::now_micros;
///
/// let grpc_recv_us = now_micros();
/// println!("gRPC 接收时间: {} μs", grpc_recv_us);
/// ```
#[inline(always)]
pub fn now_micros() -> i64 {
    let clock = HIGH_PERF_CLOCK.get_or_init(HighPerformanceClock::new);
    clock.now_micros()
}

/// 计算从指定时间戳到现在的消耗时间（微秒）
///
/// # 参数
/// - `start_timestamp_us`: 起始时间戳（微秒）
///
/// # 返回
/// 消耗时间（微秒）
///
/// # 示例
/// ```rust
/// use sol_parser_sdk::core::clock::{now_micros, elapsed_micros_since};
///
/// let start = now_micros();
/// // ... 执行解析操作 ...
/// let tx_parser_us = elapsed_micros_since(start);
/// println!("解析耗时: {} μs", tx_parser_us);
/// ```
#[inline(always)]
pub fn elapsed_micros_since(start_timestamp_us: i64) -> i64 {
    now_micros() - start_timestamp_us
}

/// 获取高精度纳秒时间戳
///
/// 用于需要纳秒级精度的场景
#[inline(always)]
pub fn now_nanos() -> i128 {
    let clock = HIGH_PERF_CLOCK.get_or_init(HighPerformanceClock::new);
    clock.now_nanos()
}

// 平台差异：Windows 使用 now_micros()；Linux 使用 CLOCK_REALTIME_COARSE；其他使用 CLOCK_REALTIME
#[inline(always)]
pub fn now_us() -> i64 {
    #[cfg(target_os = "windows")]
    {
        now_micros()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let clock_id = {
            #[cfg(target_os = "linux")]
            {
                libc::CLOCK_REALTIME_COARSE
            }

            #[cfg(not(target_os = "linux"))]
            {
                libc::CLOCK_REALTIME
            }
        };

        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        unsafe {
            libc::clock_gettime(clock_id, &mut ts);
        }
        ts.tv_sec * 1_000_000 + ts.tv_nsec / 1_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_high_performance_clock_basic() {
        let clock = HighPerformanceClock::new();
        let t1 = clock.now_micros();
        thread::sleep(Duration::from_millis(10));
        let t2 = clock.now_micros();

        let elapsed = t2 - t1;
        assert!(elapsed >= 10_000, "elapsed: {} μs", elapsed); // 至少 10ms
        assert!(elapsed < 20_000, "elapsed: {} μs", elapsed); // 不超过 20ms
    }

    #[test]
    fn test_elapsed_micros_since() {
        let clock = HighPerformanceClock::new();
        let start = clock.now_micros();
        thread::sleep(Duration::from_millis(5));
        let elapsed = clock.elapsed_micros_since(start);

        assert!(elapsed >= 5_000, "elapsed: {} μs", elapsed);
        assert!(elapsed < 10_000, "elapsed: {} μs", elapsed);
    }

    #[test]
    fn test_global_clock() {
        let t1 = now_micros();
        thread::sleep(Duration::from_millis(1));
        let t2 = now_micros();

        assert!(t2 > t1);
        assert!(t2 - t1 >= 1_000); // 至少 1ms
    }

    #[test]
    fn test_elapsed_global() {
        let start = now_micros();
        thread::sleep(Duration::from_millis(2));
        let elapsed = elapsed_micros_since(start);

        assert!(elapsed >= 2_000, "elapsed: {} μs", elapsed);
        assert!(elapsed < 5_000, "elapsed: {} μs", elapsed);
    }

    #[test]
    fn test_clock_precision() {
        let clock = HighPerformanceClock::new();
        let mut timestamps = Vec::new();

        // 快速连续获取 100 个时间戳
        for _ in 0..100 {
            timestamps.push(clock.now_micros());
        }

        // 验证时间戳单调递增
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1], "时间戳应该单调递增");
        }
    }

    #[test]
    fn test_calibration() {
        let mut clock = HighPerformanceClock::new_with_calibration_interval(0); // 立即校准
        let t1 = clock.now_micros_with_calibration();
        thread::sleep(Duration::from_millis(10));
        let t2 = clock.now_micros_with_calibration();

        let elapsed = t2 - t1;
        assert!(elapsed >= 10_000, "elapsed: {} μs", elapsed);
    }
}
