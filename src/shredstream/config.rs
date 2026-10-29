//! ShredStream 配置

/// ShredStream 客户端配置
#[derive(Clone)]
pub struct ShredStreamConfig {
    /// 可选的服务商认证令牌，通过订阅请求的 `x-token` metadata 发送。
    /// 首尾空白会被移除，空字符串视为未配置。
    pub auth_token: Option<String>,
    /// 连接超时（毫秒）
    pub connection_timeout_ms: u64,
    /// 请求超时（毫秒）
    pub request_timeout_ms: u64,
    /// 最大解码消息大小（字节）
    pub max_decoding_message_size: usize,
    /// 自动重连延迟（毫秒）
    pub reconnect_delay_ms: u64,
    /// 最大重连次数（0 表示无限重连）
    pub max_reconnect_attempts: u32,
}

impl std::fmt::Debug for ShredStreamConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShredStreamConfig")
            .field("auth_token", &self.auth_token.as_ref().map(|_| "[REDACTED]"))
            .field("connection_timeout_ms", &self.connection_timeout_ms)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field("max_decoding_message_size", &self.max_decoding_message_size)
            .field("reconnect_delay_ms", &self.reconnect_delay_ms)
            .field("max_reconnect_attempts", &self.max_reconnect_attempts)
            .finish()
    }
}

impl Default for ShredStreamConfig {
    fn default() -> Self {
        Self {
            auth_token: None,
            connection_timeout_ms: 8000,
            request_timeout_ms: 15000,
            max_decoding_message_size: 1024 * 1024 * 100, // 100MB
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 3,
        }
    }
}

impl ShredStreamConfig {
    /// 低延迟配置 - 最小化处理延迟
    pub fn low_latency() -> Self {
        Self {
            auth_token: None,
            connection_timeout_ms: 5000,
            request_timeout_ms: 10000,
            max_decoding_message_size: 1024 * 1024 * 50,
            reconnect_delay_ms: 100,
            max_reconnect_attempts: 1,
        }
    }

    /// 高吞吐配置 - 优化批量处理
    pub fn high_throughput() -> Self {
        Self {
            auth_token: None,
            connection_timeout_ms: 10000,
            request_timeout_ms: 30000,
            max_decoding_message_size: 1024 * 1024 * 200,
            reconnect_delay_ms: 2000,
            max_reconnect_attempts: 5,
        }
    }
}
