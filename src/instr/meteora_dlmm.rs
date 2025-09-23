//! Meteora DLMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora DLMM 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// Meteora DLMM 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeteoraDlmmInstruction {
    InitializeLbPair = 0,
    InitializeBinArray = 1,
    AddLiquidity = 2,
    AddLiquidityByWeight = 3,
    AddLiquidityByStrategy = 4,
    AddLiquidityByStrategyOneSide = 5,
    AddLiquidityOneSide = 6,
    RemoveLiquidity = 7,
    InitializePosition = 8,
    UpdatePosition = 9,
    WithdrawIneligibleReward = 10,
    Swap = 11,
    ClaimReward = 12,
    ClaimFee = 13,
    ClosePosition = 14,
    UpdateRewardFunder = 15,
    UpdateRewardDuration = 16,
    FundReward = 17,
    InitializeReward = 18,
    SetActivationSlot = 19,
    UpdateWhitelistedWallet = 20,
    MigratePosition = 21,
    MigrateBinArray = 22,
    UpdateFeesAndRewards = 23,
    SwapWithPriceImpact = 24,
    GoToABin = 25,
    SetPreActivationSwapAddress = 26,
    SetLockReleaseSlot = 27,
    RemoveAllLiquidity = 28,
    TogglePairStatus = 29,
    UpdateSwapCapDeactivateSlot = 30,
    CreateConfig = 31,
    CreateClaimFeeOperator = 32,
    CloseClaimFeeOperator = 33,
    ClaimPartnerFee = 34,
    ClaimProtocolFee = 35,
    CloseConfig = 36,
    SetPoolStatus = 37,
    UpdateLockDuration = 38,
    CreatePool = 39,
    SetPresetParameter = 40,
    RemovePresetParameter = 41,
}

/// Meteora DLMM 程序 ID (使用常量)
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::METEORA_DLMM_PROGRAM_ID;

/// 主要的 Meteora DLMM 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.is_empty() {
        return None;
    }

    let instruction_type = instruction_data[0];
    let data = &instruction_data[1..];

    match instruction_type {
        0 => parse_initialize_lb_pair_instruction(data, accounts, signature, slot, tx_index, block_time),
        1 => parse_initialize_bin_array_instruction(data, accounts, signature, slot, tx_index, block_time),
        2 => parse_add_liquidity_instruction(data, accounts, signature, slot, tx_index, block_time),
        7 => parse_remove_liquidity_instruction(data, accounts, signature, slot, tx_index, block_time),
        8 => parse_initialize_position_instruction(data, accounts, signature, slot, tx_index, block_time),
        11 => parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time),
        13 => parse_claim_fee_instruction(data, accounts, signature, slot, tx_index, block_time),
        14 => parse_close_position_instruction(data, accounts, signature, slot, tx_index, block_time),
        _ => None,
    }
}

/// 解析初始化LB池指令
fn parse_initialize_lb_pair_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let active_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let bin_step = read_u16_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
        metadata,
        pool,
        creator: get_account(accounts, 1).unwrap_or_default(),
        active_bin_id: active_id,
        bin_step,
    }))
}

/// 解析初始化Bin数组指令
fn parse_initialize_bin_array_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let index = read_u64_le(data, offset)? as i64;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmInitializeBinArray(MeteoraDlmmInitializeBinArrayEvent {
        metadata,
        pool,
        bin_array: get_account(accounts, 1).unwrap_or_default(),
        index,
    }))
}

/// 解析添加流动性指令
fn parse_add_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_parameter = read_bytes(data, offset, 32)?;
    offset += 32;

    let amounts = read_vec_u64(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
        metadata,
        pool,
        from: get_account(accounts, 1).unwrap_or_default(),
        position: get_account(accounts, 2).unwrap_or_default(),
        amounts: [
            amounts.get(0).copied().unwrap_or(0),
            amounts.get(1).copied().unwrap_or(0),
        ],
        active_bin_id: 0, // 从日志中获取
    }))
}

/// 解析移除流动性指令
fn parse_remove_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let bin_liquidity_removal = read_bytes(data, offset, 32)?;
    offset += 32;

    let amounts = read_vec_u64(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
        metadata,
        pool,
        from: get_account(accounts, 1).unwrap_or_default(),
        position: get_account(accounts, 2).unwrap_or_default(),
        amounts: [
            amounts.get(0).copied().unwrap_or(0),
            amounts.get(1).copied().unwrap_or(0),
        ],
        active_bin_id: 0, // 从日志中获取
    }))
}

/// 解析初始化头寸指令
fn parse_initialize_position_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lower_bin_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let width = read_u32_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
        metadata,
        pool,
        position: get_account(accounts, 1).unwrap_or_default(),
        owner: get_account(accounts, 2).unwrap_or_default(),
        lower_bin_id,
        width,
    }))
}

/// 解析交换指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let min_amount_out = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        pool,
        from: get_account(accounts, 1).unwrap_or_default(),
        start_bin_id: 0, // 从日志填充
        end_bin_id: 0, // 从日志填充
        amount_in,
        amount_out: 0, // 从日志填充
        swap_for_y: false, // 从日志填充
        fee: 0, // 从日志填充
        protocol_fee: 0, // 从日志填充
        fee_bps: 0, // 从日志填充
        host_fee: 0, // 从日志填充
    }))
}

/// 解析费用领取指令
fn parse_claim_fee_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent {
        metadata,
        pool,
        position: get_account(accounts, 1).unwrap_or_default(),
        owner: get_account(accounts, 2).unwrap_or_default(),
        fee_x: 0, // 从日志填充
        fee_y: 0, // 从日志填充
    }))
}

/// 解析关闭头寸指令
fn parse_close_position_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, pool);

    Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
        metadata,
        pool,
        position: get_account(accounts, 1).unwrap_or_default(),
        owner: get_account(accounts, 2).unwrap_or_default(),
    }))
}