//! Yellowstone Geyser gRPC 客户端连接（与 [`super::client::YellowstoneGrpc`] 共用 tonic / TLS 约定）。

use std::time::Duration;

use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient, Interceptor};

/// 连接 Geyser 的常用选项（与业务无关）。
#[derive(Debug, Clone)]
pub struct GeyserConnectConfig {
    pub connect_timeout: Duration,
    pub max_decoding_message_size: usize,
    pub x_token: Option<String>,
}

impl Default for GeyserConnectConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(8),
            max_decoding_message_size: 1024 * 1024 * 1024,
            x_token: None,
        }
    }
}

/// 建立一条 Geyser gRPC 连接（安装 rustls ring provider、`https` 时启用系统根 TLS）。
pub async fn connect_yellowstone_geyser(
    endpoint: &str,
    config: GeyserConnectConfig,
) -> Result<GeyserGrpcClient<impl Interceptor>, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut builder = GeyserGrpcClient::build_from_shared(endpoint.to_string())
        .map_err(|e| e.to_string())?
        .connect_timeout(config.connect_timeout)
        .max_decoding_message_size(config.max_decoding_message_size);

    if let Some(ref t) = config.x_token {
        builder = builder.x_token(Some(t.as_str())).map_err(|e| e.to_string())?;
    }

    if endpoint.starts_with("https://") {
        builder = builder
            .tls_config(ClientTlsConfig::new().with_native_roots())
            .map_err(|e| e.to_string())?;
    }

    builder.connect().await.map_err(|e| e.to_string())
}
