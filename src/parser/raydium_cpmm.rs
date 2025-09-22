//! Raydium CPMM DEX 解析器 - 函数式设计
//!
//! 专门解析 Raydium Constant Product Market Maker 相关的事件，包括：
//! - 交换事件
//! - 池创建事件
//! - 流动性添加事件
//! - 流动性移除事件

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use prost_types::Timestamp;
use crate::parser::events::*;

/// Raydium CPMM discriminator 常量
pub mod discriminators {
    pub const SWAP: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const CREATE_POOL: [u8; 8] = [9, 10, 11, 12, 13, 14, 15, 16];
    pub const ADD_LIQUIDITY: [u8; 8] = [17, 18, 19, 20, 21, 22, 23, 24];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [25, 26, 27, 28, 29, 30, 31, 32];
}

/// Raydium CPMM 程序 ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// 原始 Raydium CPMM 交换事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumCpmmSwapEvent {
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_token_0_in: bool,
    pub fee_amount: u64,
    pub pool_token_0_amount: u64,
    pub pool_token_1_amount: u64,
    pub price: u64,
}

/// 原始 Raydium CPMM 池创建事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumCpmmCreatePoolEvent {
    pub pool_id: Pubkey,
    pub creator: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub initial_token_0_amount: u64,
    pub initial_token_1_amount: u64,
    pub fee_rate: u16,
}

/// 原始 Raydium CPMM 流动性添加事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumCpmmAddLiquidityEvent {
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_0_amount: u64,
    pub token_1_amount: u64,
    pub lp_token_amount: u64,
    pub pool_token_0_amount: u64,
    pub pool_token_1_amount: u64,
}

/// 原始 Raydium CPMM 流动性移除事件数据结构
#[derive(BorshDeserialize)]
pub struct RawRaydiumCpmmRemoveLiquidityEvent {
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_0_amount: u64,
    pub token_1_amount: u64,
    pub lp_token_amount: u64,
    pub pool_token_0_amount: u64,
    pub pool_token_1_amount: u64,
}

/// 检查日志是否来自 Raydium CPMM 程序
pub fn is_raydium_cpmm_program(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", RAYDIUM_CPMM_PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", RAYDIUM_CPMM_PROGRAM_ID))
}

/// 从日志中提取 Program data
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    if let Some(data_start) = log.find("Program data: ") {
        let data_part = &log[data_start + 14..];
        base64::decode(data_part.trim()).ok()
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
    block_time: Option<Timestamp>,
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
        block_time_ms: block_time.as_ref().map(|ts| ts.seconds * 1000 + ts.nanos as i64 / 1_000_000),
        program_id,
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: current_time,
        handle_us: current_time,
    }
}

/// 解析 Raydium CPMM 交换事件 - 纯函数
pub fn parse_swap_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<RaydiumCpmmSwapEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmSwapEvent>(&data, discriminators::SWAP))
        .map(|raw| convert_to_swap_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 解析 Raydium CPMM 池创建事件 - 纯函数
pub fn parse_create_pool_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<RaydiumCpmmCreatePoolEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmCreatePoolEvent>(&data, discriminators::CREATE_POOL))
        .map(|raw| convert_to_create_pool_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 解析 Raydium CPMM 流动性添加事件 - 纯函数
pub fn parse_add_liquidity_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<RaydiumCpmmAddLiquidityEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmAddLiquidityEvent>(&data, discriminators::ADD_LIQUIDITY))
        .map(|raw| convert_to_add_liquidity_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 解析 Raydium CPMM 流动性移除事件 - 纯函数
pub fn parse_remove_liquidity_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<RaydiumCpmmRemoveLiquidityEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmRemoveLiquidityEvent>(&data, discriminators::REMOVE_LIQUIDITY))
        .map(|raw| convert_to_remove_liquidity_event(raw, signature, slot, block_time.clone()))
        .collect()
}

/// 转换为 Raydium CPMM 交换事件 - 纯函数
pub fn convert_to_swap_event(
    raw: RawRaydiumCpmmSwapEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> RaydiumCpmmSwapEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmSwapEvent {
        metadata,
        pool_id: raw.pool_id,
        user: raw.user,
        token_0_mint: raw.token_0_mint,
        token_1_mint: raw.token_1_mint,
        amount_in: raw.amount_in,
        amount_out: raw.amount_out,
        is_token_0_in: raw.is_token_0_in,
        fee_amount: raw.fee_amount,
        pool_token_0_amount: raw.pool_token_0_amount,
        pool_token_1_amount: raw.pool_token_1_amount,
        price: raw.price,
    }
}

/// 转换为 Raydium CPMM 池创建事件 - 纯函数
pub fn convert_to_create_pool_event(
    raw: RawRaydiumCpmmCreatePoolEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> RaydiumCpmmCreatePoolEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmCreatePoolEvent {
        metadata,
        pool_id: raw.pool_id,
        creator: raw.creator,
        token_0_mint: raw.token_0_mint,
        token_1_mint: raw.token_1_mint,
        token_0_vault: raw.token_0_vault,
        token_1_vault: raw.token_1_vault,
        lp_mint: raw.lp_mint,
        initial_token_0_amount: raw.initial_token_0_amount,
        initial_token_1_amount: raw.initial_token_1_amount,
        fee_rate: raw.fee_rate,
    }
}

/// 转换为 Raydium CPMM 流动性添加事件 - 纯函数
pub fn convert_to_add_liquidity_event(
    raw: RawRaydiumCpmmAddLiquidityEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> RaydiumCpmmAddLiquidityEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmAddLiquidityEvent {
        metadata,
        pool_id: raw.pool_id,
        user: raw.user,
        token_0_amount: raw.token_0_amount,
        token_1_amount: raw.token_1_amount,
        lp_token_amount: raw.lp_token_amount,
        pool_token_0_amount: raw.pool_token_0_amount,
        pool_token_1_amount: raw.pool_token_1_amount,
    }
}

/// 转换为 Raydium CPMM 流动性移除事件 - 纯函数
pub fn convert_to_remove_liquidity_event(
    raw: RawRaydiumCpmmRemoveLiquidityEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> RaydiumCpmmRemoveLiquidityEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmRemoveLiquidityEvent {
        metadata,
        pool_id: raw.pool_id,
        user: raw.user,
        token_0_amount: raw.token_0_amount,
        token_1_amount: raw.token_1_amount,
        lp_token_amount: raw.lp_token_amount,
        pool_token_0_amount: raw.pool_token_0_amount,
        pool_token_1_amount: raw.pool_token_1_amount,
    }
}

/// 解析所有 Raydium CPMM 事件 - 组合函数
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> (Vec<RaydiumCpmmSwapEvent>, Vec<RaydiumCpmmCreatePoolEvent>, Vec<RaydiumCpmmAddLiquidityEvent>, Vec<RaydiumCpmmRemoveLiquidityEvent>) {
    // 只处理 Raydium CPMM 相关的日志
    let cpmm_logs: Vec<_> = logs.iter()
        .filter(|log| is_raydium_cpmm_program(log))
        .cloned()
        .collect();

    if cpmm_logs.is_empty() {
        return (vec![], vec![], vec![], vec![]);
    }

    let swap_events = parse_swap_event(&cpmm_logs, signature, slot, block_time.clone());
    let create_pool_events = parse_create_pool_event(&cpmm_logs, signature, slot, block_time.clone());
    let add_liquidity_events = parse_add_liquidity_event(&cpmm_logs, signature, slot, block_time.clone());
    let remove_liquidity_events = parse_remove_liquidity_event(&cpmm_logs, signature, slot, block_time);

    (swap_events, create_pool_events, add_liquidity_events, remove_liquidity_events)
}

/// 计算 CPMM 价格 - 基于储备比例
pub fn calculate_cpmm_price(pool_token_0_amount: u64, pool_token_1_amount: u64) -> f64 {
    if pool_token_0_amount == 0 {
        return 0.0;
    }
    pool_token_1_amount as f64 / pool_token_0_amount as f64
}

/// 计算 CPMM 流动性价值 (以token_0为基准)
pub fn calculate_liquidity_value(
    token_0_amount: u64,
    token_1_amount: u64,
    pool_token_0_amount: u64,
    pool_token_1_amount: u64,
) -> f64 {
    let price = calculate_cpmm_price(pool_token_0_amount, pool_token_1_amount);
    token_0_amount as f64 + (token_1_amount as f64 * price)
}

/// 判断是否是大额 CPMM 交易
pub fn is_large_cpmm_trade(amount_in: u64, amount_out: u64) -> bool {
    let total_value = amount_in.max(amount_out);
    total_value >= 1_000_000_000 // 1 SOL 及以上或等值
}

/// 计算 CPMM 交易滑点
pub fn calculate_cpmm_slippage(
    amount_in: u64,
    amount_out: u64,
    pool_token_0_amount: u64,
    pool_token_1_amount: u64,
    is_token_0_in: bool,
) -> f64 {
    if amount_in == 0 || amount_out == 0 {
        return 0.0;
    }

    let spot_price = if is_token_0_in {
        pool_token_1_amount as f64 / pool_token_0_amount as f64
    } else {
        pool_token_0_amount as f64 / pool_token_1_amount as f64
    };

    let execution_price = amount_out as f64 / amount_in as f64;

    if spot_price == 0.0 {
        return 0.0;
    }

    ((spot_price - execution_price) / spot_price).abs() * 100.0
}

/// 计算 LP 代币份额
pub fn calculate_lp_share(
    lp_token_amount: u64,
    total_lp_supply: u64,
) -> f64 {
    if total_lp_supply == 0 {
        return 0.0;
    }
    (lp_token_amount as f64 / total_lp_supply as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_cpmm_program_detection() {
        let log1 = format!("Program {} invoke [1]", RAYDIUM_CPMM_PROGRAM_ID);
        let log2 = format!("Program {} success", RAYDIUM_CPMM_PROGRAM_ID);
        let log3 = "Program other_program invoke [1]";

        assert!(is_raydium_cpmm_program(&log1));
        assert!(is_raydium_cpmm_program(&log2));
        assert!(!is_raydium_cpmm_program(log3));
    }

    #[test]
    fn test_price_calculation() {
        let price = calculate_cpmm_price(1000, 2000);
        assert_eq!(price, 2.0);

        let price_zero = calculate_cpmm_price(0, 1000);
        assert_eq!(price_zero, 0.0);
    }

    #[test]
    fn test_liquidity_value_calculation() {
        let value = calculate_liquidity_value(100, 200, 1000, 2000);
        assert!(value > 0.0);
    }

    #[test]
    fn test_large_trade_detection() {
        assert!(is_large_cpmm_trade(2_000_000_000, 1_500_000_000));
        assert!(!is_large_cpmm_trade(100_000_000, 95_000_000));
    }

    #[test]
    fn test_slippage_calculation() {
        let slippage = calculate_cpmm_slippage(
            1000, 1900, 100000, 200000, true
        );
        assert!(slippage >= 0.0 && slippage <= 100.0);
    }

    #[test]
    fn test_lp_share_calculation() {
        let share = calculate_lp_share(100, 1000);
        assert_eq!(share, 10.0);

        let share_zero = calculate_lp_share(100, 0);
        assert_eq!(share_zero, 0.0);
    }

    #[test]
    fn test_empty_logs() {
        let empty_logs: Vec<String> = vec![];
        let (swaps, creates, adds, removes) = parse_all_events(
            &empty_logs,
            Signature::default(),
            0,
            None,
        );

        assert!(swaps.is_empty());
        assert!(creates.is_empty());
        assert!(adds.is_empty());
        assert!(removes.is_empty());
    }
}