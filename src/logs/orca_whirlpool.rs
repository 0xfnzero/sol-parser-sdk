//! Orca Whirlpool 日志解析器
//!
//! 解析 Orca Whirlpool 程序的日志事件

use solana_sdk::signature::Signature;
use crate::core::events::*;
use super::utils::*;

/// Orca Whirlpool 事件 discriminator 常量
pub mod discriminators {
    pub const TRADED_EVENT: [u8; 8] = [225, 202, 73, 175, 147, 43, 160, 150];
    pub const LIQUIDITY_INCREASED_EVENT: [u8; 8] = [30, 7, 144, 181, 102, 254, 155, 161];
    pub const LIQUIDITY_DECREASED_EVENT: [u8; 8] = [166, 1, 36, 71, 112, 202, 181, 171];
    pub const POOL_INITIALIZED_EVENT: [u8; 8] = [100, 118, 173, 87, 12, 198, 254, 229];
}

/// 主要的 Orca Whirlpool 日志解析函数
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
        discriminators::TRADED_EVENT => {
            parse_traded_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::LIQUIDITY_INCREASED_EVENT => {
            parse_liquidity_increased_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::LIQUIDITY_DECREASED_EVENT => {
            parse_liquidity_decreased_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::POOL_INITIALIZED_EVENT => {
            parse_pool_initialized_event(data, signature, slot, block_time, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析 Traded 事件
fn parse_traded_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let whirlpool = read_pubkey(data, offset)?;
    offset += 32;

    let a_to_b = read_bool(data, offset)?;
    offset += 1;

    let pre_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    let post_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    let input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let input_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let output_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, whirlpool, grpc_recv_us);

    Some(DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        a_to_b,
        pre_sqrt_price,
        post_sqrt_price,
        input_amount,
        output_amount,
        input_transfer_fee,
        output_transfer_fee,
        lp_fee,
        protocol_fee,

        // 指令参数字段
        amount: input_amount,
        amount_specified_is_input: true,
        other_amount_threshold: output_amount,
        sqrt_price_limit: pre_sqrt_price,

        // 指令账户字段
        token_authority: solana_sdk::pubkey::Pubkey::default(),
        whirlpool,
        token_owner_account_a: solana_sdk::pubkey::Pubkey::default(),
        token_vault_a: solana_sdk::pubkey::Pubkey::default(),
        token_owner_account_b: solana_sdk::pubkey::Pubkey::default(),
        token_vault_b: solana_sdk::pubkey::Pubkey::default(),
        tick_array_0: solana_sdk::pubkey::Pubkey::default(),
        tick_array_1: solana_sdk::pubkey::Pubkey::default(),
        tick_array_2: solana_sdk::pubkey::Pubkey::default(),
    }))
}

/// 解析 Liquidity Increased 事件
fn parse_liquidity_increased_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let whirlpool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let tick_lower_index = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper_index = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_transfer_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, whirlpool, grpc_recv_us);

    Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index,
        tick_upper_index,
        liquidity,
        token_a_amount,
        token_b_amount,
        token_a_transfer_fee,
        token_b_transfer_fee,
    }))
}

/// 解析 Liquidity Decreased 事件
fn parse_liquidity_decreased_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let whirlpool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let tick_lower_index = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper_index = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_transfer_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, whirlpool, grpc_recv_us);

    Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index,
        tick_upper_index,
        liquidity,
        token_a_amount,
        token_b_amount,
        token_a_transfer_fee,
        token_b_transfer_fee,
    }))
}

/// 解析 Pool Initialized 事件
fn parse_pool_initialized_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let whirlpool = read_pubkey(data, offset)?;
    offset += 32;

    let whirlpools_config = read_pubkey(data, offset)?;
    offset += 32;

    let token_mint_a = read_pubkey(data, offset)?;
    offset += 32;

    let token_mint_b = read_pubkey(data, offset)?;
    offset += 32;

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let token_program_a = read_pubkey(data, offset)?;
    offset += 32;

    let token_program_b = read_pubkey(data, offset)?;
    offset += 32;

    let decimals_a = read_u8(data, offset)?;
    offset += 1;

    let decimals_b = read_u8(data, offset)?;
    offset += 1;

    let initial_sqrt_price = read_u128_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, whirlpool, grpc_recv_us);

    Some(DexEvent::OrcaWhirlpoolPoolInitialized(OrcaWhirlpoolPoolInitializedEvent {
        metadata,
        whirlpool,
        whirlpools_config,
        token_mint_a,
        token_mint_b,
        tick_spacing,
        token_program_a,
        token_program_b,
        decimals_a,
        decimals_b,
        initial_sqrt_price,
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