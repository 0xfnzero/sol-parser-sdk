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
use crate::parser::events::*;

/// Raydium CPMM discriminator 常量
pub mod discriminators {
    pub const SWAP: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const INITIALIZE: [u8; 8] = [9, 10, 11, 12, 13, 14, 15, 16];
    pub const DEPOSIT: [u8; 8] = [17, 18, 19, 20, 21, 22, 23, 24];
    pub const WITHDRAW: [u8; 8] = [25, 26, 27, 28, 29, 30, 31, 32];
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
pub struct RawRaydiumCpmmInitializeEvent {
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
pub struct RawRaydiumCpmmDepositEvent {
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
pub struct RawRaydiumCpmmWithdrawEvent {
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
        block_time_ms: block_time,
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
    block_time: Option<i64>,
) -> Vec<RaydiumCpmmSwapEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmSwapEvent>(&data, discriminators::SWAP))
        .map(|raw| convert_to_swap_event(raw, signature, slot, block_time))
        .collect()
}

/// 解析 Raydium CPMM 池创建事件 - 纯函数
pub fn parse_initialize_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumCpmmInitializeEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmInitializeEvent>(&data, discriminators::INITIALIZE))
        .map(|raw| convert_to_initialize_event(raw, signature, slot, block_time))
        .collect()
}

/// 解析 Raydium CPMM 流动性添加事件 - 纯函数
pub fn parse_deposit_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumCpmmDepositEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmDepositEvent>(&data, discriminators::DEPOSIT))
        .map(|raw| convert_to_deposit_event(raw, signature, slot, block_time))
        .collect()
}

/// 解析 Raydium CPMM 流动性移除事件 - 纯函数
pub fn parse_withdraw_event(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<RaydiumCpmmWithdrawEvent> {
    logs.iter()
        .filter_map(|log| extract_program_data(log))
        .filter_map(|data| parse_raw_event::<RawRaydiumCpmmWithdrawEvent>(&data, discriminators::WITHDRAW))
        .map(|raw| convert_to_withdraw_event(raw, signature, slot, block_time))
        .collect()
}

/// 转换为 Raydium CPMM 交换事件 - 纯函数
pub fn convert_to_swap_event(
    raw: RawRaydiumCpmmSwapEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumCpmmSwapEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmSwapEvent {
        metadata,
        pool: raw.pool_id,
        user: raw.user,
        amount_in: raw.amount_in,
        amount_out: raw.amount_out,
        is_base_input: raw.is_token_0_in,
    }
}

/// 转换为 Raydium CPMM 池创建事件 - 纯函数
pub fn convert_to_initialize_event(
    raw: RawRaydiumCpmmInitializeEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumCpmmInitializeEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmInitializeEvent {
        metadata,
        pool: raw.pool_id,
        creator: raw.creator,
        init_amount0: raw.initial_token_0_amount,
        init_amount1: raw.initial_token_1_amount,
    }
}

/// 转换为 Raydium CPMM 流动性添加事件 - 纯函数
pub fn convert_to_deposit_event(
    raw: RawRaydiumCpmmDepositEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumCpmmDepositEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmDepositEvent {
        metadata,
        pool: raw.pool_id,
        user: raw.user,
        lp_token_amount: raw.lp_token_amount,
        token0_amount: raw.token_0_amount,
        token1_amount: raw.token_1_amount,
    }
}

/// 转换为 Raydium CPMM 流动性移除事件 - 纯函数
pub fn convert_to_withdraw_event(
    raw: RawRaydiumCpmmWithdrawEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> RaydiumCpmmWithdrawEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    RaydiumCpmmWithdrawEvent {
        metadata,
        pool: raw.pool_id,
        user: raw.user,
        lp_token_amount: raw.lp_token_amount,
        token0_amount: raw.token_0_amount,
        token1_amount: raw.token_1_amount,
    }
}

/// 解析所有 Raydium CPMM 事件 - 组合函数
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> (Vec<RaydiumCpmmSwapEvent>, Vec<RaydiumCpmmInitializeEvent>, Vec<RaydiumCpmmDepositEvent>, Vec<RaydiumCpmmWithdrawEvent>) {
    // 只处理 Raydium CPMM 相关的日志
    let cpmm_logs: Vec<_> = logs.iter()
        .filter(|log| is_raydium_cpmm_program(log))
        .cloned()
        .collect();

    if cpmm_logs.is_empty() {
        return (vec![], vec![], vec![], vec![]);
    }

    let swap_events = parse_swap_event(&cpmm_logs, signature, slot, block_time);
    let initialize_events = parse_initialize_event(&cpmm_logs, signature, slot, block_time);
    let deposit_events = parse_deposit_event(&cpmm_logs, signature, slot, block_time);
    let withdraw_events = parse_withdraw_event(&cpmm_logs, signature, slot, block_time);

    (swap_events, initialize_events, deposit_events, withdraw_events)
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
        let (swaps, initializes, deposits, withdraws) = parse_all_events(
            &empty_logs,
            Signature::default(),
            0,
            None,
        );

        assert!(swaps.is_empty());
        assert!(initializes.is_empty());
        assert!(deposits.is_empty());
        assert!(withdraws.is_empty());
    }
}

/// 检查是否是 Raydium CPMM 日志
#[inline(always)]
pub fn is_raydium_cpmm_log(log: &str) -> bool {
    log.contains("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C") || log.contains("Program data:")
}

/// 从日志字符串解析 Raydium CPMM 事件
pub fn parse_raydium_cpmm_from_log_string(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_raydium_cpmm_log(log) {
        return None;
    }

    // TODO: 实现完整的 Raydium CPMM 日志解析
    // 这里需要根据实际的 Raydium CPMM 合约日志格式来解析
    None
}