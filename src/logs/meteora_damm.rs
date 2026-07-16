//! Meteora DAMM V2 日志解析器
//!
//! 解析 Meteora DAMM V2 程序的日志事件

use super::utils::*;
use crate::core::events::*;
use solana_sdk::signature::Signature;

/// Meteora DAMM V2 事件 discriminator 常量
pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const SWAP2_EVENT: [u8; 8] = [189, 66, 51, 168, 38, 80, 117, 153];
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

/// 主要的 Meteora DAMM V2 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, tx_index, block_time_us, grpc_recv_us)
}

/// 解析结构化日志（基于 discriminator）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
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
            parse_swap_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::SWAP2_EVENT => {
            parse_swap2_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::ADD_LIQUIDITY_EVENT => {
            parse_add_liquidity_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::REMOVE_LIQUIDITY_EVENT => parse_remove_liquidity_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::INITIALIZE_POOL_EVENT => parse_initialize_pool_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CREATE_POSITION_EVENT => parse_create_position_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CLOSE_POSITION_EVENT => {
            parse_close_position_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CLAIM_POSITION_FEE_EVENT => parse_claim_position_fee_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::INITIALIZE_REWARD_EVENT => parse_initialize_reward_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::FUND_REWARD_EVENT => {
            parse_fund_reward_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CLAIM_REWARD_EVENT => {
            parse_claim_reward_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        _ => None,
    }
}

/// 解析 Swap 事件
#[inline(always)]
pub fn parse_swap_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let _config = read_pubkey(data, offset)?;
    offset += 32;

    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    let has_referral = read_bool(data, offset)?;
    offset += 1;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let actual_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    let _amount_in_dup = read_u64_le(data, offset)?;
    offset += 8;

    let current_timestamp = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee: 0,
        referral_fee,
        actual_amount_in: actual_input_amount,
        current_timestamp,
        ..Default::default()
    }))
}

fn parse_swap_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_swap_from_data(data, metadata)
}

/// 解析 Swap2 事件 (EvtSwap2 格式)
#[inline(always)]
pub fn parse_swap2_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    let _collect_fee_mode = read_u8(data, offset)?;
    offset += 1;

    let has_referral = read_bool(data, offset)?;
    offset += 1;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let swap_mode = read_u8(data, offset)?;
    offset += 1;

    let included_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let _excluded_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let _amount_left = read_u64_le(data, offset)?;
    offset += 8;

    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    let _quote_reserve_amount = read_u64_le(data, offset)?;
    offset += 8;

    let _migration_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let current_timestamp = read_u64_le(data, offset)?;

    let (amount_in, minimum_amount_out) =
        if swap_mode == 0 { (amount_0, amount_1) } else { (amount_1, amount_0) };

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee: 0,
        referral_fee,
        actual_amount_in: included_fee_input_amount,
        current_timestamp,
        ..Default::default()
    }))
}

fn parse_swap2_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_swap2_from_data(data, metadata)
}

/// 解析 Add Liquidity 事件
#[inline(always)]
pub fn parse_add_liquidity_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_delta = read_u128_le(data, offset)?;
    offset += 16;

    let token_a_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;

    let total_amount_a = read_u64_le(data, offset)?;
    offset += 8;

    let total_amount_b = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
        metadata,
        pool,
        position,
        owner,
        liquidity_delta,
        token_a_amount_threshold,
        token_b_amount_threshold,
        token_a_amount,
        token_b_amount,
        total_amount_a,
        total_amount_b,
    }))
}

fn parse_add_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_add_liquidity_from_data(data, metadata)
}

/// 解析 Remove Liquidity 事件
#[inline(always)]
pub fn parse_remove_liquidity_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_delta = read_u128_le(data, offset)?;
    offset += 16;

    let token_a_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_b_amount = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
        metadata,
        pool,
        position,
        owner,
        liquidity_delta,
        token_a_amount_threshold,
        token_b_amount_threshold,
        token_a_amount,
        token_b_amount,
    }))
}

fn parse_remove_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_remove_liquidity_from_data(data, metadata)
}

/// 解析 Initialize Pool 事件
fn parse_initialize_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_initialize_pool_from_data(data, metadata)
}

/// 解析 Initialize Pool 事件载荷
#[inline(always)]
pub fn parse_initialize_pool_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0usize;

    let pool = read_pubkey(data, offset)?;
    offset += 32;
    let token_a_mint = read_pubkey(data, offset)?;
    offset += 32;
    let token_b_mint = read_pubkey(data, offset)?;
    offset += 32;
    let creator = read_pubkey(data, offset)?;
    offset += 32;
    let payer = read_pubkey(data, offset)?;
    offset += 32;
    let alpha_vault = read_pubkey(data, offset)?;
    offset += 32;

    offset = skip_pool_fee_parameters(data, offset)?;
    if data.len() < offset + 109 {
        return None;
    }

    let sqrt_min_price = read_u128_le(data, offset)?;
    offset += 16;
    let sqrt_max_price = read_u128_le(data, offset)?;
    offset += 16;
    let activation_type = read_u8(data, offset)?;
    offset += 1;
    let collect_fee_mode = read_u8(data, offset)?;
    offset += 1;
    let liquidity = read_u128_le(data, offset)?;
    offset += 16;
    let sqrt_price = read_u128_le(data, offset)?;
    offset += 16;
    let activation_point = Some(read_u64_le(data, offset)?);
    offset += 8;
    let token_a_flag = read_u8(data, offset)?;
    offset += 1;
    let token_b_flag = read_u8(data, offset)?;
    offset += 1;
    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;
    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;
    let total_amount_a = read_u64_le(data, offset)?;
    offset += 8;
    let total_amount_b = read_u64_le(data, offset)?;
    offset += 8;
    let pool_type = read_u8(data, offset)?;

    Some(DexEvent::MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent {
        metadata,
        pool,
        token_a_mint,
        token_b_mint,
        creator,
        payer,
        alpha_vault,
        sqrt_min_price,
        sqrt_max_price,
        activation_type,
        collect_fee_mode,
        liquidity,
        sqrt_price,
        activation_point,
        token_a_flag,
        token_b_flag,
        token_a_amount,
        token_b_amount,
        total_amount_a,
        total_amount_b,
        pool_type,
        ..Default::default()
    }))
}

#[inline(always)]
fn skip_pool_fee_parameters(data: &[u8], offset: usize) -> Option<usize> {
    let tag_offset = offset + 30;
    let tag = *data.get(tag_offset)?;
    match tag {
        0 => Some(tag_offset + 1),
        1 => data.get(tag_offset + 1..tag_offset + 33).map(|_| tag_offset + 33),
        _ => None,
    }
}

/// 解析 Create Position 事件
#[inline(always)]
pub fn parse_create_position_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let position_nft_mint = read_pubkey(data, offset)?;

    Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
        metadata,
        pool,
        owner,
        position,
        position_nft_mint,
    }))
}

fn parse_create_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_create_position_from_data(data, metadata)
}

/// 解析 Close Position 事件
#[inline(always)]
pub fn parse_close_position_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let position = read_pubkey(data, offset)?;
    offset += 32;

    let position_nft_mint = read_pubkey(data, offset)?;

    Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
        metadata,
        pool,
        owner,
        position,
        position_nft_mint,
    }))
}

fn parse_close_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool = read_pubkey(data, 0)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);
    parse_close_position_from_data(data, metadata)
}

/// 解析 Claim Position Fee 事件
fn parse_claim_position_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;
    // offset += 32;

    // let fee_x = read_u64_le(data, offset)?;
    // offset += 8;

    // let fee_y = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2ClaimPositionFee(MeteoraDammV2ClaimPositionFeeEvent {
    //     metadata,
    //     lb_pair,
    //     position,
    //     owner,
    //     fee_x,
    //     fee_y,
    // }))
    None
}

/// 解析 Initialize Reward 事件
fn parse_initialize_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_mint = read_pubkey(data, offset)?;
    // offset += 32;

    // let funder = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let reward_duration = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2InitializeReward(MeteoraDammV2InitializeRewardEvent {
    //     metadata,
    //     lb_pair,
    //     reward_mint,
    //     funder,
    //     reward_index,
    //     reward_duration,
    // }))
    None
}

/// 解析 Fund Reward 事件
fn parse_fund_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let funder = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let amount = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2FundReward(MeteoraDammV2FundRewardEvent {
    //     metadata,
    //     lb_pair,
    //     funder,
    //     reward_index,
    //     amount,
    // }))
    None
}

/// 解析 Claim Reward 事件
fn parse_claim_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let total_reward = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2ClaimReward(MeteoraDammV2ClaimRewardEvent {
    //     metadata,
    //     lb_pair,
    //     position,
    //     owner,
    //     reward_index,
    //     total_reward,
    // }))
    None
}

/// 解析文本格式日志
fn parse_text_log(
    _log: &str,
    _signature: Signature,
    _slot: u64,
    tx_index: u64,
    _block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // 目前暂不实现文本解析，主要依赖结构化解析
    None
}
