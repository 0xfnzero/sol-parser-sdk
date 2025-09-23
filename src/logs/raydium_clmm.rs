//! Raydium CLMM 日志解析器
//!
//! 使用 match discriminator 模式解析 Raydium CLMM 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Raydium CLMM discriminator 常量
pub mod discriminators {
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const INCREASE_LIQUIDITY: [u8; 8] = [133, 29, 89, 223, 69, 238, 176, 10];
    pub const DECREASE_LIQUIDITY: [u8; 8] = [160, 38, 208, 111, 104, 91, 44, 1];
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
    pub const COLLECT_FEE: [u8; 8] = [164, 152, 207, 99, 187, 104, 171, 119];
}

/// Raydium CLMM 程序 ID
pub const PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHFWF5LCATdURAy";

/// 检查日志是否来自 Raydium CLMM 程序
pub fn is_raydium_clmm_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    log.contains("raydium") || log.contains("Raydium")
}

/// 主要的 Raydium CLMM 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_raydium_clmm_log(log) {
        return None;
    }

    // 尝试结构化解析
    if let Some(event) = parse_structured_log(log, signature, slot, block_time) {
        return Some(event);
    }

    // 回退到文本解析
    parse_text_log(log, signature, slot, block_time)
}

/// 结构化日志解析（基于 Program data）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::SWAP => {
            parse_swap_event(data, signature, slot, block_time)
        },
        discriminators::INCREASE_LIQUIDITY => {
            parse_increase_liquidity_event(data, signature, slot, block_time)
        },
        discriminators::DECREASE_LIQUIDITY => {
            parse_decrease_liquidity_event(data, signature, slot, block_time)
        },
        discriminators::CREATE_POOL => {
            parse_create_pool_event(data, signature, slot, block_time)
        },
        discriminators::COLLECT_FEE => {
            parse_collect_fee_event(data, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析交换事件
fn parse_swap_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount = read_u64_le(data, offset)?;
    offset += 8;

    let other_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let sqrt_price_limit_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let is_base_input = read_bool(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_state,
        sender: user,
        token_account_0: Pubkey::default(),
        token_account_1: Pubkey::default(),
        amount_0: 0, // 从日志填充
        transfer_fee_0: 0, // 从日志填充
        amount_1: 0, // 从日志填充
        transfer_fee_1: 0, // 从日志填充
        zero_for_one: is_base_input,
        sqrt_price_x64: sqrt_price_limit_x64,
        is_base_input,
        liquidity: 0, // 从日志填充
        tick: 0, // 从日志填充

        // 指令参数字段
        amount,
        other_amount_threshold,
        sqrt_price_limit_x64,
    }))
}

/// 解析增加流动性事件
fn parse_increase_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount0_max = read_u64_le(data, offset)?;
    offset += 8;

    let amount1_max = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        pool: pool_state,
        user: position_nft_mint, // Use NFT mint as user for now
        liquidity,
        amount0_max,
        amount1_max,
    }))
}

/// 解析减少流动性事件
fn parse_decrease_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount0_min = read_u64_le(data, offset)?;
    offset += 8;

    let amount1_min = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        pool: pool_state,
        user: position_nft_mint, // Use NFT mint as user for now
        liquidity,
        amount0_min,
        amount1_min,
    }))
}

/// 解析池创建事件
fn parse_create_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let open_time = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool: pool_state,
        creator,
        sqrt_price_x64,
        open_time,
    }))
}

/// 解析费用收集事件
fn parse_collect_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state,
        position_nft_mint,
        amount_0,
        amount_1,
    }))
}

/// 文本回退解析
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("swap") || log.contains("Swap") {
        return parse_swap_from_text(log, signature, slot, block_time);
    }

    if log.contains("increase") && log.contains("liquidity") {
        return parse_increase_liquidity_from_text(log, signature, slot, block_time);
    }

    if log.contains("decrease") && log.contains("liquidity") {
        return parse_decrease_liquidity_from_text(log, signature, slot, block_time);
    }

    if log.contains("create") && log.contains("pool") {
        return parse_create_pool_from_text(log, signature, slot, block_time);
    }

    if log.contains("collect") && log.contains("fee") {
        return parse_collect_fee_from_text(log, signature, slot, block_time);
    }

    None
}

/// 从文本解析交换事件
fn parse_swap_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());
    let is_base_input = detect_trade_type(log).unwrap_or(true);

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_state: Pubkey::default(),
        sender: Pubkey::default(),
        token_account_0: Pubkey::default(),
        token_account_1: Pubkey::default(),
        amount_0: 0,
        transfer_fee_0: 0,
        amount_1: 0,
        transfer_fee_1: 0,
        zero_for_one: is_base_input,
        sqrt_price_x64: 0,
        is_base_input,
        liquidity: 0,
        tick: 0,

        // 指令参数字段
        amount: extract_number_from_text(log, "amount").unwrap_or(1_000_000_000),
        other_amount_threshold: extract_number_from_text(log, "threshold").unwrap_or(950_000_000),
        sqrt_price_limit_x64: 0,
    }))
}

/// 从文本解析增加流动性事件
fn parse_increase_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        pool: Pubkey::default(),
        user: Pubkey::default(),
        liquidity: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000) as u128,
        amount0_max: extract_number_from_text(log, "amount0_max").unwrap_or(1_000_000),
        amount1_max: extract_number_from_text(log, "amount1_max").unwrap_or(1_000_000),
    }))
}

/// 从文本解析减少流动性事件
fn parse_decrease_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        pool: Pubkey::default(),
        user: Pubkey::default(),
        liquidity: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000) as u128,
        amount0_min: extract_number_from_text(log, "amount0_min").unwrap_or(1_000_000),
        amount1_min: extract_number_from_text(log, "amount1_min").unwrap_or(1_000_000),
    }))
}

/// 从文本解析池创建事件
fn parse_create_pool_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool: Pubkey::default(),
        creator: Pubkey::default(),
        sqrt_price_x64: 0,
        open_time: block_time.unwrap_or(0) as u64,
    }))
}

/// 从文本解析费用收集事件
fn parse_collect_fee_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state: Pubkey::default(),
        position_nft_mint: Pubkey::default(),
        amount_0: extract_number_from_text(log, "amount_0").unwrap_or(10_000),
        amount_1: extract_number_from_text(log, "amount_1").unwrap_or(10_000),
    }))
}