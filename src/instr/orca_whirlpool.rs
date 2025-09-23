//! Orca Whirlpool 指令解析器
//!
//! 使用 match discriminator 模式解析 Orca Whirlpool 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// Orca Whirlpool 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrcaWhirlpoolInstruction {
    InitializeConfig = 0,
    InitializePool = 1,
    InitializeTickArray = 2,
    InitializeFeeTier = 3,
    InitializeReward = 4,
    SetRewardEmissions = 5,
    OpenPosition = 6,
    OpenPositionWithMetadata = 7,
    IncreaseLiquidity = 8,
    DecreaseLiquidity = 9,
    UpdateFeesAndRewards = 10,
    CollectFees = 11,
    CollectReward = 12,
    CollectProtocolFees = 13,
    Swap = 14,
    ClosePosition = 15,
    SetDefaultFeeRate = 16,
    SetDefaultProtocolFeeRate = 17,
    SetFeeRate = 18,
    SetProtocolFeeRate = 19,
    SetFeeAuthority = 20,
    SetCollectProtocolFeesAuthority = 21,
    SetRewardAuthority = 22,
    SetRewardAuthorityBySuperAuthority = 23,
    SetRewardEmissionsSuperAuthority = 24,
    TwoHopSwap = 25,
    InitializePositionBundle = 26,
    InitializePositionBundleWithMetadata = 27,
    DeletePositionBundle = 28,
    OpenBundledPosition = 29,
    CloseBundledPosition = 30,
    CollectFeesV2 = 31,
    CollectProtocolFeesV2 = 32,
    CollectRewardV2 = 33,
    DecreaseLiquidityV2 = 34,
    IncreaseLiquidityV2 = 35,
    InitializePoolV2 = 36,
    InitializeRewardV2 = 37,
    SetRewardEmissionsV2 = 38,
    SwapV2 = 39,
    TwoHopSwapV2 = 40,
}

impl OrcaWhirlpoolInstruction {
    /// 从 discriminator 转换为指令类型
    pub fn from_discriminator(discriminator: &[u8; 8]) -> Option<Self> {
        match discriminator {
            &[208, 127, 21, 1, 194, 190, 196, 70] => Some(Self::InitializeConfig),
            &[17, 43, 80, 74, 168, 202, 6, 113] => Some(Self::InitializePool),
            &[214, 27, 15, 109, 164, 252, 221, 253] => Some(Self::InitializeTickArray),
            &[183, 74, 156, 160, 112, 2, 42, 30] => Some(Self::InitializeFeeTier),
            &[95, 135, 192, 196, 242, 129, 230, 68] => Some(Self::InitializeReward),
            &[13, 197, 86, 168, 109, 176, 27, 244] => Some(Self::SetRewardEmissions),
            &[87, 190, 72, 189, 204, 203, 226, 66] => Some(Self::OpenPosition),
            &[78, 217, 28, 185, 88, 104, 255, 231] => Some(Self::OpenPositionWithMetadata),
            &[46, 156, 243, 118, 13, 205, 251, 178] => Some(Self::IncreaseLiquidity),
            &[160, 38, 208, 111, 104, 91, 44, 1] => Some(Self::DecreaseLiquidity),
            &[173, 178, 66, 24, 33, 156, 204, 31] => Some(Self::UpdateFeesAndRewards),
            &[164, 152, 207, 99, 30, 186, 19, 182] => Some(Self::CollectFees),
            &[206, 68, 114, 253, 168, 177, 245, 180] => Some(Self::CollectReward),
            &[22, 67, 23, 98, 150, 178, 70, 220] => Some(Self::CollectProtocolFees),
            &[248, 198, 158, 145, 225, 117, 135, 200] => Some(Self::Swap),
            &[123, 134, 81, 0, 49, 68, 98, 98] => Some(Self::ClosePosition),
            &[43, 4, 237, 11, 26, 201, 30, 98] => Some(Self::SwapV2),
            &[195, 96, 237, 108, 68, 162, 219, 230] => Some(Self::TwoHopSwap),
            &[186, 143, 209, 29, 254, 2, 194, 117] => Some(Self::TwoHopSwapV2),
            _ => None,
        }
    }
}

/// Orca Whirlpool discriminator 常量
pub mod discriminators {
    pub const INITIALIZE_CONFIG: [u8; 8] = [208, 127, 21, 1, 194, 190, 196, 70];
    pub const INITIALIZE_POOL: [u8; 8] = [17, 43, 80, 74, 168, 202, 6, 113];
    pub const INITIALIZE_TICK_ARRAY: [u8; 8] = [214, 27, 15, 109, 164, 252, 221, 253];
    pub const INITIALIZE_FEE_TIER: [u8; 8] = [183, 74, 156, 160, 112, 2, 42, 30];
    pub const INITIALIZE_REWARD: [u8; 8] = [95, 135, 192, 196, 242, 129, 230, 68];
    pub const SET_REWARD_EMISSIONS: [u8; 8] = [13, 197, 86, 168, 109, 176, 27, 244];
    pub const OPEN_POSITION: [u8; 8] = [87, 190, 72, 189, 204, 203, 226, 66];
    pub const OPEN_POSITION_WITH_METADATA: [u8; 8] = [78, 217, 28, 185, 88, 104, 255, 231];
    pub const INCREASE_LIQUIDITY: [u8; 8] = [46, 156, 243, 118, 13, 205, 251, 178];
    pub const DECREASE_LIQUIDITY: [u8; 8] = [160, 38, 208, 111, 104, 91, 44, 1];
    pub const UPDATE_FEES_AND_REWARDS: [u8; 8] = [173, 178, 66, 24, 33, 156, 204, 31];
    pub const COLLECT_FEES: [u8; 8] = [164, 152, 207, 99, 30, 186, 19, 182];
    pub const COLLECT_REWARD: [u8; 8] = [206, 68, 114, 253, 168, 177, 245, 180];
    pub const COLLECT_PROTOCOL_FEES: [u8; 8] = [22, 67, 23, 98, 150, 178, 70, 220];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const TWO_HOP_SWAP: [u8; 8] = [195, 96, 237, 108, 68, 162, 219, 230];
    pub const TWO_HOP_SWAP_V2: [u8; 8] = [186, 143, 209, 29, 254, 2, 194, 117];
}

/// Orca Whirlpool 程序 ID (为了向后兼容保留字符串版本)
pub const PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Orca Whirlpool 程序 ID (优化版本 - 使用 Pubkey 常量)
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::ORCA_WHIRLPOOL_PROGRAM_ID;

/// 主要的 Orca Whirlpool 指令解析函数
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
    let instruction_type = OrcaWhirlpoolInstruction::from_discriminator(&discriminator)?;
    let data = &instruction_data[8..];

    match instruction_type {
        OrcaWhirlpoolInstruction::Swap | OrcaWhirlpoolInstruction::SwapV2 => {
            parse_swap_instruction(data, accounts, signature, slot, block_time)
        },
        OrcaWhirlpoolInstruction::IncreaseLiquidity | OrcaWhirlpoolInstruction::IncreaseLiquidityV2 => {
            parse_increase_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        OrcaWhirlpoolInstruction::DecreaseLiquidity | OrcaWhirlpoolInstruction::DecreaseLiquidityV2 => {
            parse_decrease_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        OrcaWhirlpoolInstruction::InitializePool | OrcaWhirlpoolInstruction::InitializePoolV2 => {
            parse_initialize_pool_instruction(data, accounts, signature, slot, block_time)
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

    let amount = read_u64_le(data, offset)?;
    offset += 8;

    let other_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let sqrt_price_limit = read_u128_le(data, offset)?;
    offset += 16;

    let amount_specified_is_input = read_bool(data, offset)?;
    offset += 1;

    let a_to_b = read_bool(data, offset)?;

    let whirlpool = get_account(accounts, 1)?;
    let metadata = create_metadata(signature, slot, block_time, whirlpool);

    Some(DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
        metadata,
        whirlpool,
        a_to_b,
        pre_sqrt_price: sqrt_price_limit, // 从指令获取初始值，日志会覆盖
        post_sqrt_price: 0, // 从日志中获取
        input_amount: if amount_specified_is_input { amount } else { 0 },
        output_amount: if !amount_specified_is_input {
            amount
        } else {
            other_amount_threshold // 使用阈值作为初始值，日志会覆盖
        },
        input_transfer_fee: 0, // 从日志中获取
        output_transfer_fee: 0, // 从日志中获取
        lp_fee: 0, // 从日志中获取
        protocol_fee: 0, // 从日志中获取
    }))
}

/// 解析 Increase Liquidity 指令
fn parse_increase_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_amount = read_u128_le(data, offset)?;
    offset += 16;

    let token_max_a = read_u64_le(data, offset)?;
    offset += 8;

    let token_max_b = read_u64_le(data, offset)?;

    let whirlpool = get_account(accounts, 1)?;
    let position = get_account(accounts, 3)?;
    let metadata = create_metadata(signature, slot, block_time, whirlpool);

    Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index: 0, // 从日志中获取
        tick_upper_index: 0, // 从日志中获取
        liquidity: liquidity_amount,
        token_a_amount: token_max_a, // 从指令获取最大值，日志会覆盖实际值
        token_b_amount: token_max_b, // 从指令获取最大值，日志会覆盖实际值
        token_a_transfer_fee: 0, // 从日志中获取
        token_b_transfer_fee: 0, // 从日志中获取
    }))
}

/// 解析 Decrease Liquidity 指令
fn parse_decrease_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_amount = read_u128_le(data, offset)?;
    offset += 16;

    let token_min_a = read_u64_le(data, offset)?;
    offset += 8;

    let token_min_b = read_u64_le(data, offset)?;

    let whirlpool = get_account(accounts, 1)?;
    let position = get_account(accounts, 3)?;
    let metadata = create_metadata(signature, slot, block_time, whirlpool);

    Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index: 0, // 从日志中获取
        tick_upper_index: 0, // 从日志中获取
        liquidity: liquidity_amount,
        token_a_amount: token_min_a, // 从指令获取最小值，日志会覆盖实际值
        token_b_amount: token_min_b, // 从指令获取最小值，日志会覆盖实际值
        token_a_transfer_fee: 0, // 从日志中获取
        token_b_transfer_fee: 0, // 从日志中获取
    }))
}

/// 解析 Initialize Pool 指令
fn parse_initialize_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let initial_sqrt_price = read_u128_le(data, offset)?;

    let whirlpool = get_account(accounts, 1)?;
    let whirlpools_config = get_account(accounts, 2)?;
    let token_mint_a = get_account(accounts, 3)?;
    let token_mint_b = get_account(accounts, 4)?;
    let metadata = create_metadata(signature, slot, block_time, whirlpool);

    Some(DexEvent::OrcaWhirlpoolPoolInitialized(OrcaWhirlpoolPoolInitializedEvent {
        metadata,
        whirlpool,
        whirlpools_config,
        token_mint_a,
        token_mint_b,
        tick_spacing,
        token_program_a: get_account(accounts, 8).unwrap_or_default(),
        token_program_b: get_account(accounts, 9).unwrap_or_default(),
        decimals_a: 0, // 从日志中获取
        decimals_b: 0, // 从日志中获取
        initial_sqrt_price,
    }))
}