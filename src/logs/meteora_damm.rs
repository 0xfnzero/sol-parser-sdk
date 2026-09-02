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
    pub const LIQUIDITY_CHANGE_EVENT: [u8; 8] = [197, 171, 78, 127, 224, 211, 87, 13];
    pub const INITIALIZE_POOL_EVENT: [u8; 8] = [228, 50, 246, 85, 203, 66, 134, 37];
    pub const CREATE_POSITION_EVENT: [u8; 8] = [156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION_EVENT: [u8; 8] = [20, 145, 144, 68, 143, 142, 214, 178];
    pub const CLAIM_POSITION_FEE_EVENT: [u8; 8] = [198, 182, 183, 52, 97, 12, 49, 56];
    pub const INITIALIZE_REWARD_EVENT: [u8; 8] = [129, 91, 188, 3, 246, 52, 185, 249];
    pub const FUND_REWARD_EVENT: [u8; 8] = [104, 233, 237, 122, 199, 191, 121, 85];
    pub const CLAIM_REWARD_EVENT: [u8; 8] = [218, 86, 147, 200, 235, 188, 215, 231];
}

/// Mainnet upgrade that replaced `trading_fee/partner_fee` in `EvtSwap2` with
/// `claiming_fee/compounding_fee` without changing its discriminator or size.
/// Upgrade transaction: `51mWiF7buxgQWqEGKWhrdE35ssnuPmANkXCShKqsn3S3siQFKyvwUnNHRZHLqMfQeUhMyhneCfxJF8g2V2G9eqFy`.
pub const COMPOUNDING_FEE_LAYOUT_ACTIVATION_SLOT: u64 = 406_048_752;

#[inline(always)]
fn uses_compounding_fee_layout(slot: u64) -> bool {
    // Direct payload callers do not always have transaction metadata. In that
    // case, prefer the current wire semantics.
    slot == 0 || slot >= COMPOUNDING_FEE_LAYOUT_ACTIVATION_SLOT
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
        discriminators::LIQUIDITY_CHANGE_EVENT => parse_liquidity_change_event(
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

    let collect_fee_mode = read_u8(data, offset)?;
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

    let excluded_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let amount_left = read_u64_le(data, offset)?;
    offset += 8;

    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    let claiming_or_trading_fee = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    let compounding_or_partner_fee = read_u64_le(data, offset)?;
    offset += 8;

    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    let included_transfer_fee_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let included_transfer_fee_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let excluded_transfer_fee_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let current_timestamp = read_u64_le(data, offset)?;
    offset += 8;

    let reserve_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let reserve_b_amount = read_u64_le(data, offset)?;

    // ExactIn (0) and PartialFill (1) use amount_0 as input and amount_1 as
    // minimum output. ExactOut (2) uses amount_0 as output and amount_1 as max input.
    let (amount_in, minimum_amount_out) = match swap_mode {
        0 | 1 => (amount_0, amount_1),
        2 => (amount_1, amount_0),
        _ => return None,
    };

    let (lp_fee, partner_fee, claiming_fee, compounding_fee) =
        if uses_compounding_fee_layout(metadata.slot) {
            (
                claiming_or_trading_fee.checked_add(compounding_or_partner_fee)?,
                compounding_or_partner_fee,
                claiming_or_trading_fee,
                compounding_or_partner_fee,
            )
        } else {
            (claiming_or_trading_fee, compounding_or_partner_fee, 0, 0)
        };

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        collect_fee_mode,
        has_referral,
        amount_0,
        amount_1,
        swap_mode,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee,
        referral_fee,
        actual_amount_in: included_fee_input_amount,
        excluded_fee_input_amount,
        amount_left,
        claiming_fee,
        compounding_fee,
        included_transfer_fee_amount_in,
        included_transfer_fee_amount_out,
        excluded_transfer_fee_amount_out,
        current_timestamp,
        reserve_a_amount,
        reserve_b_amount,
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
        ..Default::default()
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
        ..Default::default()
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

/// Parse current `EvtLiquidityChange`; `change_type` 0 is add and 1 is remove.
#[inline(always)]
pub fn parse_liquidity_change_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const LEN: usize = 177;
    if data.len() < LEN {
        return None;
    }

    let pool = read_pubkey(data, 0)?;
    let position = read_pubkey(data, 32)?;
    let owner = read_pubkey(data, 64)?;
    let token_a_amount = read_u64_le(data, 96)?;
    let token_b_amount = read_u64_le(data, 104)?;
    let total_amount_a = read_u64_le(data, 112)?;
    let total_amount_b = read_u64_le(data, 120)?;
    let reserve_a_amount = read_u64_le(data, 128)?;
    let reserve_b_amount = read_u64_le(data, 136)?;
    let liquidity_delta = read_u128_le(data, 144)?;
    let token_a_amount_threshold = read_u64_le(data, 160)?;
    let token_b_amount_threshold = read_u64_le(data, 168)?;

    match read_u8(data, 176)? {
        0 => Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
            metadata,
            pool,
            position,
            owner,
            token_a_amount,
            token_b_amount,
            liquidity_delta,
            token_a_amount_threshold,
            token_b_amount_threshold,
            total_amount_a,
            total_amount_b,
            reserve_a_amount,
            reserve_b_amount,
        })),
        1 => Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
            metadata,
            pool,
            position,
            owner,
            token_a_amount,
            token_b_amount,
            liquidity_delta,
            token_a_amount_threshold,
            token_b_amount_threshold,
            total_amount_a,
            total_amount_b,
            reserve_a_amount,
            reserve_b_amount,
        })),
        _ => None,
    }
}

fn parse_liquidity_change_event(
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
    parse_liquidity_change_from_data(data, metadata)
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use solana_sdk::pubkey::Pubkey;

    fn push_pubkey(data: &mut Vec<u8>, byte: u8) -> Pubkey {
        let key = Pubkey::new_from_array([byte; 32]);
        data.extend_from_slice(key.as_ref());
        key
    }

    fn current_swap2_payload() -> (Vec<u8>, Pubkey) {
        let mut data = Vec::with_capacity(180);
        let pool = push_pubkey(&mut data, 1);
        data.push(1); // trade_direction
        data.push(2); // collect_fee_mode
        data.push(1); // has_referral
        data.extend_from_slice(&1_000u64.to_le_bytes()); // amount_0
        data.extend_from_slice(&900u64.to_le_bytes()); // amount_1
        data.push(0); // exact-in
        data.extend_from_slice(&1_000u64.to_le_bytes()); // included_fee_input_amount
        data.extend_from_slice(&990u64.to_le_bytes()); // excluded_fee_input_amount
        data.extend_from_slice(&5u64.to_le_bytes()); // amount_left
        data.extend_from_slice(&880u64.to_le_bytes()); // output_amount
        data.extend_from_slice(&123_456u128.to_le_bytes()); // next_sqrt_price
        data.extend_from_slice(&3u64.to_le_bytes()); // claiming_fee
        data.extend_from_slice(&2u64.to_le_bytes()); // protocol_fee
        data.extend_from_slice(&1u64.to_le_bytes()); // compounding_fee
        data.extend_from_slice(&4u64.to_le_bytes()); // referral_fee
        data.extend_from_slice(&10u64.to_le_bytes()); // included_transfer_fee_amount_in
        data.extend_from_slice(&11u64.to_le_bytes()); // included_transfer_fee_amount_out
        data.extend_from_slice(&870u64.to_le_bytes()); // excluded_transfer_fee_amount_out
        data.extend_from_slice(&1_725_000_000u64.to_le_bytes()); // current_timestamp
        data.extend_from_slice(&50_000u64.to_le_bytes()); // reserve_a_amount
        data.extend_from_slice(&60_000u64.to_le_bytes()); // reserve_b_amount
        assert_eq!(data.len(), 180);
        (data, pool)
    }

    fn current_liquidity_change_payload(change_type: u8) -> (Vec<u8>, Pubkey, Pubkey, Pubkey) {
        let mut data = Vec::with_capacity(177);
        let pool = push_pubkey(&mut data, 2);
        let position = push_pubkey(&mut data, 3);
        let owner = push_pubkey(&mut data, 4);
        data.extend_from_slice(&101u64.to_le_bytes()); // token_a_amount
        data.extend_from_slice(&202u64.to_le_bytes()); // token_b_amount
        data.extend_from_slice(&111u64.to_le_bytes()); // transfer_fee_included_token_a_amount
        data.extend_from_slice(&222u64.to_le_bytes()); // transfer_fee_included_token_b_amount
        data.extend_from_slice(&1_001u64.to_le_bytes()); // reserve_a_amount
        data.extend_from_slice(&2_002u64.to_le_bytes()); // reserve_b_amount
        data.extend_from_slice(&303u128.to_le_bytes()); // liquidity_delta
        data.extend_from_slice(&404u64.to_le_bytes()); // token_a_amount_threshold
        data.extend_from_slice(&505u64.to_le_bytes()); // token_b_amount_threshold
        data.push(change_type);
        assert_eq!(data.len(), 177);
        (data, pool, position, owner)
    }

    fn program_data_log(discriminator: [u8; 8], payload: &[u8]) -> String {
        let mut data = Vec::with_capacity(8 + payload.len());
        data.extend_from_slice(&discriminator);
        data.extend_from_slice(payload);
        format!("Program data: {}", STANDARD.encode(data))
    }

    #[test]
    fn parses_current_swap2_layout() {
        let (data, pool) = current_swap2_payload();
        let event = parse_swap2_from_data(&data, EventMetadata::default()).expect("swap2 event");
        let DexEvent::MeteoraDammV2Swap(event) = event else {
            panic!("expected DAMM v2 swap");
        };

        assert_eq!(event.pool, pool);
        assert_eq!(event.trade_direction, 1);
        assert_eq!(event.collect_fee_mode, 2);
        assert!(event.has_referral);
        assert_eq!((event.amount_0, event.amount_1, event.swap_mode), (1_000, 900, 0));
        assert_eq!(event.amount_in, 1_000);
        assert_eq!(event.minimum_amount_out, 900);
        assert_eq!(event.actual_amount_in, 1_000);
        assert_eq!(event.excluded_fee_input_amount, 990);
        assert_eq!(event.amount_left, 5);
        assert_eq!(event.output_amount, 880);
        assert_eq!(event.next_sqrt_price, 123_456);
        assert_eq!((event.claiming_fee, event.compounding_fee), (3, 1));
        assert_eq!(event.lp_fee, 4);
        assert_eq!(event.protocol_fee, 2);
        assert_eq!(event.partner_fee, 1);
        assert_eq!(event.referral_fee, 4);
        assert_eq!(event.included_transfer_fee_amount_in, 10);
        assert_eq!(event.included_transfer_fee_amount_out, 11);
        assert_eq!(event.excluded_transfer_fee_amount_out, 870);
        assert_eq!(event.current_timestamp, 1_725_000_000);
        assert_eq!((event.reserve_a_amount, event.reserve_b_amount), (50_000, 60_000));

        let (data, _) = current_swap2_payload();
        let event = crate::instr::all_inner::meteora_damm::parse(
            &crate::instr::all_inner::meteora_damm::discriminators::SWAP2,
            &data,
            EventMetadata::default(),
        )
        .expect("inner Swap2 event");
        let DexEvent::MeteoraDammV2Swap(event) = event else {
            panic!("expected inner DAMM v2 swap");
        };
        assert_eq!((event.actual_amount_in, event.output_amount), (1_000, 880));
        assert_eq!((event.reserve_a_amount, event.reserve_b_amount), (50_000, 60_000));
    }

    #[test]
    fn swap2_fee_slots_follow_the_mainnet_upgrade_boundary() {
        let (data, _) = current_swap2_payload();

        let legacy = parse_swap2_from_data(
            &data,
            EventMetadata {
                slot: COMPOUNDING_FEE_LAYOUT_ACTIVATION_SLOT - 1,
                ..Default::default()
            },
        )
        .expect("legacy swap2 event");
        let DexEvent::MeteoraDammV2Swap(legacy) = legacy else {
            panic!("expected DAMM v2 swap");
        };
        assert_eq!(legacy.lp_fee, 3);
        assert_eq!(legacy.partner_fee, 1);
        assert_eq!((legacy.claiming_fee, legacy.compounding_fee), (0, 0));

        let current = parse_swap2_from_data(
            &data,
            EventMetadata { slot: COMPOUNDING_FEE_LAYOUT_ACTIVATION_SLOT, ..Default::default() },
        )
        .expect("current swap2 event");
        let DexEvent::MeteoraDammV2Swap(current) = current else {
            panic!("expected DAMM v2 swap");
        };
        assert_eq!(current.lp_fee, 4);
        assert_eq!(current.partner_fee, 1);
        assert_eq!((current.claiming_fee, current.compounding_fee), (3, 1));
    }

    #[test]
    fn current_swap2_maps_all_swap_modes() {
        const SWAP_MODE_OFFSET: usize = 32 + 1 + 1 + 1 + 8 + 8;

        for (swap_mode, expected) in [(0, (1_000, 900)), (1, (1_000, 900)), (2, (900, 1_000))] {
            let (mut data, _) = current_swap2_payload();
            data[SWAP_MODE_OFFSET] = swap_mode;
            let event =
                parse_swap2_from_data(&data, EventMetadata::default()).expect("swap2 event");
            let DexEvent::MeteoraDammV2Swap(event) = event else {
                panic!("expected DAMM v2 swap");
            };
            assert_eq!((event.amount_in, event.minimum_amount_out), expected);
        }

        let (mut data, _) = current_swap2_payload();
        data[SWAP_MODE_OFFSET] = 3;
        assert!(parse_swap2_from_data(&data, EventMetadata::default()).is_none());
    }

    #[test]
    fn parses_current_liquidity_change_as_add_or_remove() {
        for (change_type, expect_add) in [(0, true), (1, false)] {
            let (data, pool, position, owner) = current_liquidity_change_payload(change_type);
            let log = program_data_log([197, 171, 78, 127, 224, 211, 87, 13], &data);
            let event = parse_log(&log, Signature::default(), 1, 2, Some(3), 4)
                .expect("liquidity change event");

            match event {
                DexEvent::MeteoraDammV2AddLiquidity(event) if expect_add => {
                    assert_eq!((event.pool, event.position, event.owner), (pool, position, owner));
                    assert_eq!((event.token_a_amount, event.token_b_amount), (101, 202));
                    assert_eq!(event.liquidity_delta, 303);
                    assert_eq!(
                        (event.token_a_amount_threshold, event.token_b_amount_threshold),
                        (404, 505)
                    );
                    assert_eq!((event.total_amount_a, event.total_amount_b), (111, 222));
                    assert_eq!((event.reserve_a_amount, event.reserve_b_amount), (1_001, 2_002));
                }
                DexEvent::MeteoraDammV2RemoveLiquidity(event) if !expect_add => {
                    assert_eq!((event.pool, event.position, event.owner), (pool, position, owner));
                    assert_eq!((event.token_a_amount, event.token_b_amount), (101, 202));
                    assert_eq!(event.liquidity_delta, 303);
                    assert_eq!(
                        (event.token_a_amount_threshold, event.token_b_amount_threshold),
                        (404, 505)
                    );
                    assert_eq!((event.total_amount_a, event.total_amount_b), (111, 222));
                    assert_eq!((event.reserve_a_amount, event.reserve_b_amount), (1_001, 2_002));
                }
                other => panic!("unexpected liquidity event: {other:?}"),
            }
        }
    }

    #[test]
    fn current_liquidity_change_honors_exact_filters_and_inner_routing() {
        use crate::grpc::{EventType, EventTypeFilter};

        let program_id = crate::instr::program_ids::METEORA_DAMM_V2_PROGRAM_ID;
        let add_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2AddLiquidity]);
        let remove_filter =
            EventTypeFilter::include_only(vec![EventType::MeteoraDammV2RemoveLiquidity]);

        for (change_type, allowed_filter, rejected_filter) in
            [(0, &add_filter, &remove_filter), (1, &remove_filter, &add_filter)]
        {
            let (data, _, _, _) = current_liquidity_change_payload(change_type);
            let log = program_data_log(discriminators::LIQUIDITY_CHANGE_EVENT, &data);

            let event = crate::logs::parse_log_with_program_id(
                &log,
                Signature::default(),
                1,
                2,
                Some(3),
                4,
                Some(allowed_filter),
                false,
                None,
                Some(&program_id),
            );
            assert!(event.is_some(), "matching change_type must pass its exact filter");

            let event = crate::logs::parse_log_with_program_id(
                &log,
                Signature::default(),
                1,
                2,
                Some(3),
                4,
                Some(rejected_filter),
                false,
                None,
                Some(&program_id),
            );
            assert!(event.is_none(), "non-matching change_type must be filtered out");

            let event = crate::instr::all_inner::meteora_damm::parse(
                &crate::instr::all_inner::meteora_damm::discriminators::LIQUIDITY_CHANGE,
                &data,
                EventMetadata::default(),
            )
            .expect("inner liquidity-change event");
            assert_eq!(matches!(event, DexEvent::MeteoraDammV2AddLiquidity(_)), change_type == 0);
        }
    }
}
