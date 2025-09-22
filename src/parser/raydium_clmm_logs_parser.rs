//! Raydium CLMM DEX 解析器 - 函数式设计
//!
//! 专门解析 Raydium Concentrated Liquidity Market Maker 相关的事件，包括：
//! - 交换事件
//! - 头寸开仓事件
//! - 头寸平仓事件
//! - 流动性添加/移除事件

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
// use prost_types::Timestamp;
use crate::parser::events::*;

/// Raydium CLMM discriminator 常量
pub mod discriminators {
    pub const SWAP: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const POSITION_OPEN: [u8; 8] = [135, 128, 47, 77, 15, 152, 240, 49];
    pub const POSITION_CLOSE: [u8; 8] = [67, 89, 12, 145, 89, 121, 45, 203];
    pub const COLLECT_FEE: [u8; 8] = [156, 45, 78, 123, 89, 67, 34, 12];
}

/// Raydium CLMM 程序 ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// 原始 Raydium CLMM 交换事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumClmmSwapEvent {
    pub pool_state: Pubkey,
    pub user_wallet: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_a_to_b: bool,
    pub tick_current: i32,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub fee_amount: u64,
}

/// 原始 Raydium CLMM 头寸开仓事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumClmmOpenPositionEvent {
    pub pool_state: Pubkey,
    pub position_nft_mint: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
}

/// 原始 Raydium CLMM 头寸平仓事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumClmmClosePositionEvent {
    pub pool_state: Pubkey,
    pub position_nft_mint: Pubkey,
    pub owner: Pubkey,
    pub liquidity_removed: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub fee_amount_0: u64,
    pub fee_amount_1: u64,
}

/// 检查日志是否来自 Raydium CLMM 程序
pub fn is_raydium_clmm_program(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", RAYDIUM_CLMM_PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", RAYDIUM_CLMM_PROGRAM_ID))
}

/// 从日志中提取 Program data
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    if let Some(data_start) = log.find("Program data: ") {
        let data_part = &log[data_start + 14..];
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(data_part.trim()).ok()
    } else {
        None
    }
}

/// 解析原始事件数据 - 纯函数
pub fn parse_raw_event<T: BorshDeserialize>(data: &[u8], expected_discriminator: [u8; 8]) -> Option<T> {
    if data.len() < 8 {
        return None;
    }

    // 检查 discriminator
    let discriminator: [u8; 8] = data[0..8].try_into().ok()?;
    if discriminator != expected_discriminator {
        return None;
    }

    // 反序列化剩余数据
    let mut event_data = &data[8..];
    T::deserialize(&mut event_data).ok()
}

/// 创建事件元数据 - 纯函数
pub fn create_event_metadata(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    program_id: Pubkey,
) -> EventMetadata {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time.map(|ts| ts * 1000),
        program_id,
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: current_time,
        handle_us: current_time,
    }
}

/// 解析 Raydium CLMM 交换事件 - 纯函数
pub fn parse_swap_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumClmmSwapEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumClmmSwapEvent>(&data, discriminators::SWAP))
        .map(|raw| convert_to_swap_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 解析 Raydium CLMM 头寸开仓事件 - 纯函数
pub fn parse_position_open_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumClmmOpenPositionEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumClmmOpenPositionEvent>(&data, discriminators::POSITION_OPEN))
        .map(|raw| convert_to_position_open_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 解析 Raydium CLMM 头寸平仓事件 - 纯函数
pub fn parse_position_close_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumClmmClosePositionEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumClmmClosePositionEvent>(&data, discriminators::POSITION_CLOSE))
        .map(|raw| convert_to_position_close_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 转换为 Raydium CLMM 交换事件 - 纯函数
pub fn convert_to_swap_event(
    raw: RawRaydiumClmmSwapEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumClmmSwapEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_state);

    RaydiumClmmSwapEvent {
        metadata,
        pool: raw.pool_state,
        user: raw.user_wallet,
        amount: raw.amount_in,
        other_amount_threshold: raw.amount_out,
        sqrt_price_limit_x64: raw.sqrt_price_x64,
        is_base_input: raw.is_a_to_b,
    }
}

/// 转换为 Raydium CLMM 头寸开仓事件 - 纯函数
pub fn convert_to_position_open_event(
    raw: RawRaydiumClmmOpenPositionEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumClmmOpenPositionEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_state);

    RaydiumClmmOpenPositionEvent {
        metadata,
        pool: raw.pool_state,
        user: raw.owner,
        position_nft_mint: raw.position_nft_mint,
        tick_lower_index: raw.tick_lower,
        tick_upper_index: raw.tick_upper,
        liquidity: raw.liquidity,
    }
}

/// 转换为 Raydium CLMM 头寸平仓事件 - 纯函数
pub fn convert_to_position_close_event(
    raw: RawRaydiumClmmClosePositionEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumClmmClosePositionEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_state);

    RaydiumClmmClosePositionEvent {
        metadata,
        pool: raw.pool_state,
        user: raw.owner,
        position_nft_mint: raw.position_nft_mint,
    }
}

/// 解析所有 Raydium CLMM 事件 - 组合函数
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> (Vec<RaydiumClmmSwapEvent>, Vec<RaydiumClmmOpenPositionEvent>, Vec<RaydiumClmmClosePositionEvent>) {
    // 只处理 Raydium CLMM 相关的日志
    let clmm_logs: Vec<_> = logs.iter()
        .filter(|log| is_raydium_clmm_program(log))
        .cloned()
        .collect();

    if clmm_logs.is_empty() {
        return (vec![], vec![], vec![]);
    }

    let swap_events = parse_swap_event(&clmm_logs, signature, slot, block_time.clone());
    let position_open_events = parse_position_open_event(&clmm_logs, signature, slot, block_time.clone());
    let position_close_events = parse_position_close_event(&clmm_logs, signature, slot, block_time);

    (swap_events, position_open_events, position_close_events)
}

/// 计算 CLMM 价格 - 基于 sqrt_price_x64
pub fn calculate_price_from_sqrt_price(sqrt_price_x64: u128) -> f64 {
    let sqrt_price = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    sqrt_price * sqrt_price
}

/// 计算流动性范围价格
pub fn calculate_tick_price(tick: i32) -> f64 {
    let base: f64 = 1.0001;
    base.powf(tick as f64)
}

/// 判断是否是大额 CLMM 交易
pub fn is_large_clmm_trade(amount_in: u64, amount_out: u64) -> bool {
    let total_value = amount_in.max(amount_out);
    total_value >= 1_000_000_000 // 1 SOL 及以上或等值
}

/// 计算 CLMM 交易滑点
pub fn calculate_clmm_slippage(amount_in: u64, amount_out: u64, sqrt_price_x64: u128) -> f64 {
    if amount_in == 0 || amount_out == 0 {
        return 0.0;
    }

    let actual_price = amount_out as f64 / amount_in as f64;
    let market_price = calculate_price_from_sqrt_price(sqrt_price_x64);

    if market_price == 0.0 {
        return 0.0;
    }

    ((market_price - actual_price) / market_price).abs() * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_clmm_program_detection() {
        let log1 = format!("Program {} invoke [1]", RAYDIUM_CLMM_PROGRAM_ID);
        let log2 = format!("Program {} success", RAYDIUM_CLMM_PROGRAM_ID);
        let log3 = "Program other_program invoke [1]";

        assert!(is_raydium_clmm_program(&log1));
        assert!(is_raydium_clmm_program(&log2));
        assert!(!is_raydium_clmm_program(log3));
    }

    #[test]
    fn test_price_calculations() {
        // 测试 sqrt price 计算
        let sqrt_price_x64 = 1u128 << 32; // 例子: sqrt(1) * 2^64 / 2^32 = 2^32
        let price = calculate_price_from_sqrt_price(sqrt_price_x64);
        assert!(price > 0.0);

        // 测试 tick 价格计算
        let tick = 0;
        let tick_price = calculate_tick_price(tick);
        assert!((tick_price - 1.0).abs() < 0.0001); // tick 0 should be price 1.0
    }

    #[test]
    fn test_large_trade_detection() {
        assert!(is_large_clmm_trade(2_000_000_000, 1_500_000_000)); // 大额交易
        assert!(!is_large_clmm_trade(100_000_000, 95_000_000));     // 小额交易
    }

    #[test]
    fn test_slippage_calculation() {
        let amount_in = 1_000_000_000;
        let amount_out = 950_000_000;
        let sqrt_price_x64 = 1u128 << 63; // sqrt(0.5) * 2^64

        let slippage = calculate_clmm_slippage(amount_in, amount_out, sqrt_price_x64);
        assert!(slippage >= 0.0 && slippage <= 100.0);
    }

    #[test]
    fn test_empty_logs() {
        let empty_logs: Vec<String> = vec![];
        let (swaps, opens, closes) = parse_all_events(
            &empty_logs,
            Signature::default(),
            0,
            None,
        );

        assert!(swaps.is_empty());
        assert!(opens.is_empty());
        assert!(closes.is_empty());
    }
}

/// 检查是否是 Raydium CLMM 日志
#[inline(always)]
pub fn is_raydium_clmm_log(log: &str) -> bool {
    log.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") || log.contains("Program data:")
}

/// 从日志字符串解析 Raydium CLMM 事件
pub fn parse_raydium_clmm_from_log_string(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_raydium_clmm_log(log) {
        return None;
    }

    // TODO: 实现完整的 Raydium CLMM 日志解析
    // 这里需要根据实际的 Raydium CLMM 合约日志格式来解析
    None
}