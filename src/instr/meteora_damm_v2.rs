//! Meteora DAMM V2 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora DAMM V2 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Meteora DAMM V2 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeteoraDammV2Instruction {
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
    CreateTokenBadge = 38,
    SetTokenBadgeOwner = 39,
    CreateDynamicConfig = 40,
    InitializeRewardV2 = 41,
    FundRewardV2 = 42,
    ClaimRewardV2 = 43,
    ClaimPositionFee = 44,
    CloseDynamicConfig = 45,
    LockPosition = 46,
    PermanentLockPosition = 47,
}

impl MeteoraDammV2Instruction {
    /// 从 discriminator 转换为指令类型
    pub fn from_discriminator(discriminator: &[u8; 8]) -> Option<Self> {
        match discriminator {
            &[228, 50, 246, 85, 203, 66, 134, 37] => Some(Self::InitializeLbPair),
            &[129, 91, 188, 3, 246, 52, 185, 249] => Some(Self::InitializeReward),
            &[175, 242, 8, 157, 30, 247, 185, 169] => Some(Self::AddLiquidity),
            &[87, 46, 88, 98, 175, 96, 34, 91] => Some(Self::RemoveLiquidity),
            &[156, 15, 119, 198, 29, 181, 221, 55] => Some(Self::InitializePosition),
            &[20, 145, 144, 68, 143, 142, 214, 178] => Some(Self::ClosePosition),
            &[27, 60, 21, 213, 138, 170, 187, 147] => Some(Self::Swap),
            &[218, 86, 147, 200, 235, 188, 215, 231] => Some(Self::ClaimReward),
            &[198, 182, 183, 52, 97, 12, 49, 56] => Some(Self::ClaimPositionFee),
            &[104, 233, 237, 122, 199, 191, 121, 85] => Some(Self::FundReward),
            _ => None,
        }
    }
}

/// Meteora DAMM V2 discriminator 常量
pub mod discriminators {
    pub const INITIALIZE_LB_PAIR: [u8; 8] = [228, 50, 246, 85, 203, 66, 134, 37];
    pub const INITIALIZE_REWARD: [u8; 8] = [129, 91, 188, 3, 246, 52, 185, 249];
    pub const ADD_LIQUIDITY: [u8; 8] = [175, 242, 8, 157, 30, 247, 185, 169];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [87, 46, 88, 98, 175, 96, 34, 91];
    pub const INITIALIZE_POSITION: [u8; 8] = [156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION: [u8; 8] = [20, 145, 144, 68, 143, 142, 214, 178];
    pub const SWAP: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const CLAIM_REWARD: [u8; 8] = [218, 86, 147, 200, 235, 188, 215, 231];
    pub const CLAIM_POSITION_FEE: [u8; 8] = [198, 182, 183, 52, 97, 12, 49, 56];
    pub const FUND_REWARD: [u8; 8] = [104, 233, 237, 122, 199, 191, 121, 85];
}

/// Meteora DAMM V2 程序 ID
pub const PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// 主要的 Meteora DAMM V2 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let instruction_type = MeteoraDammV2Instruction::from_discriminator(&discriminator)?;
    let data = &instruction_data[8..];

    match instruction_type {
        MeteoraDammV2Instruction::Swap => {
            parse_swap_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::AddLiquidity => {
            parse_add_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::RemoveLiquidity => {
            parse_remove_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::InitializeLbPair => {
            parse_initialize_lb_pair_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::InitializePosition => {
            parse_initialize_position_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::ClosePosition => {
            parse_close_position_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::ClaimPositionFee => {
            parse_claim_position_fee_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::InitializeReward => {
            parse_initialize_reward_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::FundReward => {
            parse_fund_reward_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraDammV2Instruction::ClaimReward => {
            parse_claim_reward_instruction(data, accounts, signature, slot, block_time)
        },
        _ => None, // 其他指令暂不解析
    }
}

/// 解析 Swap 指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let min_amount_out = read_u64_le(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let user = get_account(accounts, 7)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        lb_pair,
        from: user,
        start_bin_id: 0, // 从日志中获取
        end_bin_id: 0, // 从日志中获取
        amount_in,
        amount_out: min_amount_out, // 先用指令中的最小值，日志会覆盖实际值
        swap_for_y: false, // 从日志中获取
        fee: 0, // 从日志中获取
        protocol_fee: 0, // 从日志中获取
        fee_bps: 0, // 从日志中获取
        host_fee: 0, // 从日志中获取
    }))
}

/// 解析 Add Liquidity 指令
fn parse_add_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    // liquidity_parameter 结构解析
    let amount_x = read_u64_le(data, offset)?;
    offset += 8;

    let amount_y = read_u64_le(data, offset)?;
    offset += 8;

    let bin_liquidity_dist = read_bin_liquidity_distribution(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let position = get_account(accounts, 1)?;
    let user = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
        metadata,
        lb_pair,
        from: user,
        position,
        amounts: [amount_x, amount_y],
        active_bin_id: 0, // 从日志中获取
    }))
}

/// 解析 Remove Liquidity 指令
fn parse_remove_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let bin_liquidity_removal = read_bin_liquidity_removal(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let position = get_account(accounts, 1)?;
    let user = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
        metadata,
        lb_pair,
        from: user,
        position,
        amounts: [0, 0], // 从日志中获取
        active_bin_id: 0, // 从日志中获取
    }))
}

/// 解析 Initialize LB Pair 指令
fn parse_initialize_lb_pair_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let active_id = read_u32_le(data, offset)? as i32;
    offset += 4;

    let bin_step = read_u16_le(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let token_x = get_account(accounts, 2)?;
    let token_y = get_account(accounts, 3)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent {
        metadata,
        lb_pair,
        bin_step,
        token_x,
        token_y,
    }))
}

/// 解析 Initialize Position 指令
fn parse_initialize_position_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let lb_pair = get_account(accounts, 0)?;
    let position = get_account(accounts, 1)?;
    let owner = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
        metadata,
        lb_pair,
        position,
        owner,
    }))
}

/// 解析 Close Position 指令
fn parse_close_position_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let position = get_account(accounts, 0)?;
    let owner = get_account(accounts, 1)?;
    let metadata = create_metadata(signature, slot, block_time, position);

    Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
        metadata,
        position,
        owner,
    }))
}

/// 解析 Claim Position Fee 指令
fn parse_claim_position_fee_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let lb_pair = get_account(accounts, 0)?;
    let position = get_account(accounts, 1)?;
    let owner = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2ClaimPositionFee(MeteoraDammV2ClaimPositionFeeEvent {
        metadata,
        lb_pair,
        position,
        owner,
        fee_x: 0, // 从日志中获取
        fee_y: 0, // 从日志中获取
    }))
}

/// 解析 Initialize Reward 指令
fn parse_initialize_reward_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let reward_index = read_u64_le(data, offset)?;
    offset += 8;

    let reward_duration = read_u64_le(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let reward_mint = get_account(accounts, 1)?;
    let funder = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2InitializeReward(MeteoraDammV2InitializeRewardEvent {
        metadata,
        lb_pair,
        reward_mint,
        funder,
        reward_index,
        reward_duration,
    }))
}

/// 解析 Fund Reward 指令
fn parse_fund_reward_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let reward_index = read_u64_le(data, offset)?;
    offset += 8;

    let amount = read_u64_le(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let funder = get_account(accounts, 1)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2FundReward(MeteoraDammV2FundRewardEvent {
        metadata,
        lb_pair,
        funder,
        reward_index,
        amount,
    }))
}

/// 解析 Claim Reward 指令
fn parse_claim_reward_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let reward_index = read_u64_le(data, offset)?;

    let lb_pair = get_account(accounts, 0)?;
    let position = get_account(accounts, 1)?;
    let owner = get_account(accounts, 2)?;
    let metadata = create_metadata(signature, slot, block_time, lb_pair);

    Some(DexEvent::MeteoraDammV2ClaimReward(MeteoraDammV2ClaimRewardEvent {
        metadata,
        lb_pair,
        position,
        owner,
        reward_index,
        total_reward: 0, // 从日志中获取
    }))
}

/// 读取 bin liquidity distribution （简化版本）
fn read_bin_liquidity_distribution(data: &[u8], offset: usize) -> Option<Vec<u8>> {
    // 这里应该根据实际结构解析，暂时返回空 vec
    Some(vec![])
}

/// 读取 bin liquidity removal （简化版本）
fn read_bin_liquidity_removal(data: &[u8], offset: usize) -> Option<Vec<u8>> {
    // 这里应该根据实际结构解析，暂时返回空 vec
    Some(vec![])
}