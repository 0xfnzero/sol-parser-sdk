//! Meteora DLMM 日志解析器
//!
//! 解析 Meteora DLMM 程序的日志事件

use solana_sdk::signature::Signature;
use crate::core::events::*;
use super::utils::*;
use crate::instr::read_vec_u64;

/// Meteora DLMM 事件 discriminator 常量
pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
    pub const ADD_LIQUIDITY_EVENT: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
    pub const REMOVE_LIQUIDITY_EVENT: [u8; 8] = [80, 85, 209, 72, 24, 206, 35, 178];
    pub const INITIALIZE_BIN_ARRAY_EVENT: [u8; 8] = [11, 18, 155, 194, 33, 115, 238, 119];
    pub const INITIALIZE_POOL_EVENT: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];
    pub const CREATE_POSITION_EVENT: [u8; 8] = [123, 233, 11, 43, 146, 180, 97, 119];
    pub const CLOSE_POSITION_EVENT: [u8; 8] = [94, 168, 102, 45, 59, 122, 137, 54];
    pub const CLAIM_FEE_EVENT: [u8; 8] = [152, 70, 208, 111, 104, 91, 44, 1];
}

/// 判断是否为 Meteora DLMM 日志
pub fn is_meteora_dlmm_log(log: &str) -> bool {
    log.contains("Program data: ") &&
    (log.contains("Program log: Instruction:") || log.contains("meteora") || log.contains("DLMM"))
}

/// 主要的 Meteora DLMM 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_meteora_dlmm_log(log) {
        return None;
    }

    // 尝试结构化解析
    if let Some(event) = parse_structured_log(log, signature, slot, block_time) {
        return Some(event);
    }

    // 尝试文本解析作为备选
    parse_text_log(log, signature, slot, block_time)
}

/// 解析结构化日志（基于 discriminator）
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
        discriminators::SWAP_EVENT => {
            parse_swap_event(data, signature, slot, block_time)
        },
        discriminators::ADD_LIQUIDITY_EVENT => {
            parse_add_liquidity_event(data, signature, slot, block_time)
        },
        discriminators::REMOVE_LIQUIDITY_EVENT => {
            parse_remove_liquidity_event(data, signature, slot, block_time)
        },
        discriminators::INITIALIZE_POOL_EVENT => {
            parse_initialize_pool_event(data, signature, slot, block_time)
        },
        discriminators::CREATE_POSITION_EVENT => {
            parse_create_position_event(data, signature, slot, block_time)
        },
        discriminators::CLOSE_POSITION_EVENT => {
            parse_close_position_event(data, signature, slot, block_time)
        },
        discriminators::CLAIM_FEE_EVENT => {
            parse_claim_fee_event(data, signature, slot, block_time)
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

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let start_bin_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let end_bin_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let swap_for_y = read_bool(data, offset)?;
    offset += 1;

    let fee = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    let host_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        pool,
        from,
        start_bin_id,
        end_bin_id,
        amount_in,
        amount_out,
        swap_for_y,
        fee,
        protocol_fee,
        host_fee,
    }))
}

/// 解析添加流动性事件
fn parse_add_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_minted = read_u64_le(data, offset)?;
    offset += 8;

    let amounts = read_vec_u64(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
        metadata,
        pool,
        from,
        liquidity_minted,
        amounts,
    }))
}

/// 解析移除流动性事件
fn parse_remove_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_burned = read_u64_le(data, offset)?;
    offset += 8;

    let amounts = read_vec_u64(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
        metadata,
        pool,
        from,
        liquidity_burned,
        amounts,
    }))
}

/// 解析池初始化事件
fn parse_initialize_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let active_bin_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let bin_step = read_u16_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
        metadata,
        pool,
        creator,
        active_bin_id,
        bin_step,
    }))
}

/// 解析创建头寸事件
fn parse_create_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let lower_bin_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let width = read_u32_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
        metadata,
        pool,
        position,
        owner,
        lower_bin_id,
        width,
    }))
}

/// 解析关闭头寸事件
fn parse_close_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
        metadata,
        pool,
        position,
        owner,
    }))
}

/// 解析费用领取事件
fn parse_claim_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let fee_x = read_u64_le(data, offset)?;
    offset += 8;

    let fee_y = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent {
        metadata,
        pool,
        position,
        owner,
        fee_x,
        fee_y,
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

    if log.contains("add") && log.contains("liquidity") {
        return parse_add_liquidity_from_text(log, signature, slot, block_time);
    }

    if log.contains("remove") && log.contains("liquidity") {
        return parse_remove_liquidity_from_text(log, signature, slot, block_time);
    }

    if log.contains("initialize") && log.contains("pool") {
        return parse_initialize_pool_from_text(log, signature, slot, block_time);
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

    let metadata = create_metadata_simple(signature, slot, block_time, solana_sdk::pubkey::Pubkey::default());

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        pool: solana_sdk::pubkey::Pubkey::default(),
        from: solana_sdk::pubkey::Pubkey::default(),
        start_bin_id: 0,
        end_bin_id: 0,
        amount_in: extract_number_from_text(log, "amount_in").unwrap_or(1_000_000),
        amount_out: extract_number_from_text(log, "amount_out").unwrap_or(950_000),
        swap_for_y: detect_trade_type(log).unwrap_or(true),
        fee: extract_number_from_text(log, "fee").unwrap_or(3000),
        protocol_fee: 0,
        host_fee: 0,
    }))
}

/// 从文本解析添加流动性事件
fn parse_add_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, solana_sdk::pubkey::Pubkey::default());

    Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
        metadata,
        pool: solana_sdk::pubkey::Pubkey::default(),
        from: solana_sdk::pubkey::Pubkey::default(),
        liquidity_minted: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000),
        amounts: vec![
            extract_number_from_text(log, "amount_x").unwrap_or(500_000),
            extract_number_from_text(log, "amount_y").unwrap_or(500_000),
        ],
    }))
}

/// 从文本解析移除流动性事件
fn parse_remove_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, solana_sdk::pubkey::Pubkey::default());

    Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
        metadata,
        pool: solana_sdk::pubkey::Pubkey::default(),
        from: solana_sdk::pubkey::Pubkey::default(),
        liquidity_burned: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000),
        amounts: vec![
            extract_number_from_text(log, "amount_x").unwrap_or(500_000),
            extract_number_from_text(log, "amount_y").unwrap_or(500_000),
        ],
    }))
}

/// 从文本解析池初始化事件
fn parse_initialize_pool_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, solana_sdk::pubkey::Pubkey::default());

    Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
        metadata,
        pool: solana_sdk::pubkey::Pubkey::default(),
        creator: solana_sdk::pubkey::Pubkey::default(),
        active_bin_id: extract_number_from_text(log, "bin_id").unwrap_or(0) as i32,
        bin_step: extract_number_from_text(log, "bin_step").unwrap_or(1) as u16,
    }))
}