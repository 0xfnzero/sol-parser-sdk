//! Meteora Pools 日志解析器
//!
//! 解析 Meteora Pools 程序的日志事件

use solana_sdk::signature::Signature;
use crate::core::events::*;
use super::utils::*;

/// Meteora Pools 事件 discriminator 常量
pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [81, 108, 227, 190, 205, 208, 10, 196];
    pub const ADD_LIQUIDITY_EVENT: [u8; 8] = [31, 94, 125, 90, 227, 52, 61, 186];
    pub const REMOVE_LIQUIDITY_EVENT: [u8; 8] = [116, 244, 97, 232, 103, 31, 152, 58];
    pub const BOOTSTRAP_LIQUIDITY_EVENT: [u8; 8] = [121, 127, 38, 136, 92, 55, 14, 247];
    pub const POOL_CREATED_EVENT: [u8; 8] = [202, 44, 41, 88, 104, 220, 157, 82];
    pub const SET_POOL_FEES_EVENT: [u8; 8] = [245, 26, 198, 164, 88, 18, 75, 9];
}

/// 主要的 Meteora Pools 日志解析函数
pub fn parse_log(log: &str, signature: Signature, slot: u64, block_time: Option<i64>, grpc_recv_us: i64) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, block_time, grpc_recv_us)
}

/// 解析结构化日志（基于 discriminator）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;

    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::SWAP_EVENT => {
            parse_swap_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::ADD_LIQUIDITY_EVENT => {
            parse_add_liquidity_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::REMOVE_LIQUIDITY_EVENT => {
            parse_remove_liquidity_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::BOOTSTRAP_LIQUIDITY_EVENT => {
            parse_bootstrap_liquidity_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::POOL_CREATED_EVENT => {
            parse_pool_created_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::SET_POOL_FEES_EVENT => {
            parse_set_pool_fees_event(data, signature, slot, block_time, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析 Swap 事件
fn parse_swap_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let in_amount = read_u64_le(data, offset)?;
    offset += 8;

    let out_amount = read_u64_le(data, offset)?;
    offset += 8;

    let trade_fee = read_u64_le(data, offset)?;
    offset += 8;

    let admin_fee = read_u64_le(data, offset)?;
    offset += 8;

    let host_fee = read_u64_le(data, offset)?;

    // 使用默认的程序 ID，实际应该从上下文获取
    let metadata = create_metadata_default(signature, slot, block_time);

    Some(DexEvent::MeteoraPoolsSwap(MeteoraPoolsSwapEvent {
        metadata,
        in_amount,
        out_amount,
        trade_fee,
        admin_fee,
        host_fee,
    }))
}

/// 解析 Add Liquidity 事件
fn parse_add_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_mint_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_default(signature, slot, block_time);

    Some(DexEvent::MeteoraPoolsAddLiquidity(MeteoraPoolsAddLiquidityEvent {
        metadata,
        lp_mint_amount,
        token_a_amount,
        token_b_amount,
    }))
}

/// 解析 Remove Liquidity 事件
fn parse_remove_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_unmint_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_out_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_out_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_default(signature, slot, block_time);

    Some(DexEvent::MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent {
        metadata,
        lp_unmint_amount,
        token_a_out_amount,
        token_b_out_amount,
    }))
}

/// 解析 Bootstrap Liquidity 事件
fn parse_bootstrap_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_mint_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;

    let pool = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool, grpc_recv_us);

    Some(DexEvent::MeteoraPoolsBootstrapLiquidity(MeteoraPoolsBootstrapLiquidityEvent {
        metadata,
        lp_mint_amount,
        token_a_amount,
        token_b_amount,
        pool,
    }))
}

/// 解析 Pool Created 事件
fn parse_pool_created_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_a_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_b_mint = read_pubkey(data, offset)?;
    offset += 32;

    let pool_type = read_u8(data, offset)?;
    offset += 1;

    let pool = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool, grpc_recv_us);

    Some(DexEvent::MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent {
        metadata,
        lp_mint,
        token_a_mint,
        token_b_mint,
        pool_type,
        pool,
    }))
}

/// 解析 Set Pool Fees 事件
fn parse_set_pool_fees_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let trade_fee_numerator = read_u64_le(data, offset)?;
    offset += 8;

    let trade_fee_denominator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_trade_fee_numerator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_trade_fee_denominator = read_u64_le(data, offset)?;
    offset += 8;

    let pool = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool, grpc_recv_us);

    Some(DexEvent::MeteoraPoolsSetPoolFees(MeteoraPoolsSetPoolFeesEvent {
        metadata,
        trade_fee_numerator,
        trade_fee_denominator,
        owner_trade_fee_numerator,
        owner_trade_fee_denominator,
        pool,
    }))
}

/// 解析文本格式日志
fn parse_text_log(
    _log: &str,
    _signature: Signature,
    _slot: u64,
    _block_time: Option<i64>,
) -> Option<DexEvent> {
    // 目前暂不实现文本解析，主要依赖结构化解析
    None
}