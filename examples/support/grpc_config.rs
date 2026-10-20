//! Shared online-example configuration; never embeds or prints credentials.
pub fn endpoint() -> anyhow::Result<String> {
    let value = std::env::var("GRPC_ENDPOINT")
        .or_else(|_| std::env::var("GRPC_URL"))
        .map_err(|_| anyhow::anyhow!("set GRPC_ENDPOINT (GRPC_URL is also accepted)"))?;
    anyhow::ensure!(!value.trim().is_empty(), "GRPC_ENDPOINT is empty");
    Ok(value)
}

pub fn auth_token() -> Option<String> {
    std::env::var("GRPC_AUTH_TOKEN")
        .or_else(|_| std::env::var("GRPC_TOKEN"))
        .ok()
        .filter(|value| !value.trim().is_empty())
}
