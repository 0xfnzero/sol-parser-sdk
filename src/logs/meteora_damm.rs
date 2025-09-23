//! Meteora DAMM V2 日志解析器
//!
//! 解析 Meteora DAMM V2 程序的日志事件

use solana_sdk::signature::Signature;
use crate::core::events::*;
use super::utils::*;

/// Meteora DAMM V2 事件 discriminator 常量
pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const ADD_LIQUIDITY_EVENT: [u8; 8] = [175, 242, 8, 157, 30, 247, 185, 169];
    pub const REMOVE_LIQUIDITY_EVENT: [u8; 8] = [87, 46, 88, 98, 175, 96, 34, 91];
    pub const INITIALIZE_POOL_EVENT: [u8; 8] = [228, 50, 246, 85, 203, 66, 134, 37];
    pub const CREATE_POSITION_EVENT: [u8; 8] = [156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION_EVENT: [u8; 8] = [20, 145, 144, 68, 143, 142, 214, 178];
    pub const CLAIM_POSITION_FEE_EVENT: [u8; 8] = [198, 182, 183, 52, 97, 12, 49, 56];
    pub const INITIALIZE_REWARD_EVENT: [u8; 8] = [129, 91, 188, 3, 246, 52, 185, 249];
    pub const FUND_REWARD_EVENT: [u8; 8] = [104, 233, 237, 122, 199, 191, 121, 85];
    pub const CLAIM_REWARD_EVENT: [u8; 8] = [218, 86, 147, 200, 235, 188, 215, 231];
}

/// 判断是否为 Meteora DAMM 日志
pub fn is_meteora_damm_log(log: &str) -> bool {
    log.contains("Program data: ") &&
    (log.contains("Program log: Instruction:") || log.contains("meteora") || log.contains("LB"))
}

/// 主要的 Meteora DAMM V2 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
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
        discriminators::CLAIM_POSITION_FEE_EVENT => {
            parse_claim_position_fee_event(data, signature, slot, block_time)
        },
        discriminators::INITIALIZE_REWARD_EVENT => {
            parse_initialize_reward_event(data, signature, slot, block_time)
        },
        discriminators::FUND_REWARD_EVENT => {
            parse_fund_reward_event(data, signature, slot, block_time)
        },
        discriminators::CLAIM_REWARD_EVENT => {
            parse_claim_reward_event(data, signature, slot, block_time)
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
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let start_bin_id = read_i32_le(data, offset)?;
    offset += 4;

    let end_bin_id = read_i32_le(data, offset)?;
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

    let fee_bps = read_u128_le(data, offset)?;
    offset += 16;

    let host_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        lb_pair,
        from,
        start_bin_id,
        end_bin_id,
        amount_in,
        amount_out,
        swap_for_y,
        fee,
        protocol_fee,
        fee_bps,
        host_fee,
    }))
}

/// 解析 Add Liquidity 事件
fn parse_add_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let amounts = [
        read_u64_le(data, offset)?,
        read_u64_le(data, offset + 8)?,
    ];
    offset += 16;

    let active_bin_id = read_i32_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
        metadata,
        lb_pair,
        from,
        position,
        amounts,
        active_bin_id,
    }))
}

/// 解析 Remove Liquidity 事件
fn parse_remove_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let from = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let amounts = [
        read_u64_le(data, offset)?,
        read_u64_le(data, offset + 8)?,
    ];
    offset += 16;

    let active_bin_id = read_i32_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
        metadata,
        lb_pair,
        from,
        position,
        amounts,
        active_bin_id,
    }))
}

/// 解析 Initialize Pool 事件
fn parse_initialize_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let bin_step = read_u16_le(data, offset)?;
    offset += 2;

    let token_x = read_pubkey(data, offset)?;
    offset += 32;

    let token_y = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent {
        metadata,
        lb_pair,
        bin_step,
        token_x,
        token_y,
    }))
}

/// 解析 Create Position 事件
fn parse_create_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
        metadata,
        lb_pair,
        position,
        owner,
    }))
}

/// 解析 Close Position 事件
fn parse_close_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, position);

    Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
        metadata,
        position,
        owner,
    }))
}

/// 解析 Claim Position Fee 事件
fn parse_claim_position_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let fee_x = read_u64_le(data, offset)?;
    offset += 8;

    let fee_y = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2ClaimPositionFee(MeteoraDammV2ClaimPositionFeeEvent {
        metadata,
        lb_pair,
        position,
        owner,
        fee_x,
        fee_y,
    }))
}

/// 解析 Initialize Reward 事件
fn parse_initialize_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let reward_mint = read_pubkey(data, offset)?;
    offset += 32;

    let funder = read_pubkey(data, offset)?;
    offset += 32;

    let reward_index = read_u64_le(data, offset)?;
    offset += 8;

    let reward_duration = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2InitializeReward(MeteoraDammV2InitializeRewardEvent {
        metadata,
        lb_pair,
        reward_mint,
        funder,
        reward_index,
        reward_duration,
    }))
}

/// 解析 Fund Reward 事件
fn parse_fund_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let funder = read_pubkey(data, offset)?;
    offset += 32;

    let reward_index = read_u64_le(data, offset)?;
    offset += 8;

    let amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2FundReward(MeteoraDammV2FundRewardEvent {
        metadata,
        lb_pair,
        funder,
        reward_index,
        amount,
    }))
}

/// 解析 Claim Reward 事件
fn parse_claim_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lb_pair = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let reward_index = read_u64_le(data, offset)?;
    offset += 8;

    let total_reward = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2ClaimReward(MeteoraDammV2ClaimRewardEvent {
        metadata,
        lb_pair,
        position,
        owner,
        reward_index,
        total_reward,
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