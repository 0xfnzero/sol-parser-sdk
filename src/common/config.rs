use super::constants::*;

/// Backpressure handling strategy
#[derive(Debug, Clone, Copy)]
pub enum BackpressureStrategy {
    /// Block and wait (default)
    Block,
    /// Drop messages
    Drop,
}

impl Default for BackpressureStrategy {
    fn default() -> Self {
        Self::Block
    }
}

/// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Channel size (default: 1000)
    pub permits: usize,
    /// Backpressure handling strategy (default: Block)
    pub strategy: BackpressureStrategy,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self { permits: 3000, strategy: BackpressureStrategy::default() }
    }
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Connection timeout in seconds (default: 10)
    pub connect_timeout: u64,
    /// Request timeout in seconds (default: 60)
    pub request_timeout: u64,
    /// Maximum decoding message size in bytes (default: 10MB)
    pub max_decoding_message_size: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_decoding_message_size: DEFAULT_MAX_DECODING_MESSAGE_SIZE,
        }
    }
}

/// Common client configuration
#[derive(Debug, Clone)]
pub struct StreamClientConfig {
    /// Connection configuration
    pub connection: ConnectionConfig,
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
    /// Whether performance monitoring is enabled (default: false)
    pub enable_metrics: bool,
}

impl Default for StreamClientConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig::default(),
            backpressure: BackpressureConfig::default(),
            enable_metrics: false,
        }
    }
}

impl StreamClientConfig {
    /// Creates a high-throughput configuration optimized for high-concurrency scenarios.
    ///
    /// This configuration prioritizes throughput over latency by:
    /// - Implementing a drop strategy for backpressure to avoid blocking
    /// - Setting a large permit buffer (5,000) to handle burst traffic
    ///
    /// Ideal for scenarios where you need to process large volumes of data
    /// and can tolerate occasional message drops during peak loads.
    pub fn high_throughput() -> Self {
        Self {
            connection: ConnectionConfig::default(),
            backpressure: BackpressureConfig {
                permits: 20000,
                strategy: BackpressureStrategy::Drop,
            },
            enable_metrics: false,
        }
    }

    /// Creates a low-latency configuration optimized for real-time scenarios.
    ///
    /// This configuration prioritizes latency over throughput by:
    /// - Processing events immediately without buffering
    /// - Implementing a blocking backpressure strategy to ensure no data loss
    /// - Setting optimal permits (4000) for balanced throughput and latency
    ///
    /// Ideal for scenarios where every millisecond counts and you cannot
    /// afford to lose any events, such as trading applications or real-time monitoring.
    pub fn low_latency() -> Self {
        Self {
            connection: ConnectionConfig::default(),
            backpressure: BackpressureConfig { permits: 4000, strategy: BackpressureStrategy::Block },
            enable_metrics: false,
        }
    }

}
