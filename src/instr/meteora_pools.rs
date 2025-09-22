//! Meteora Pools 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora Pools 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Meteora Pools 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeteoraPoolsInstruction {
    Initialize = 0,
    Swap = 1,
    AddLiquidity = 2,
    RemoveLiquidity = 3,
    CreateConfig = 4,
    CloseConfig = 5,
    UpdateCurveInfo = 6,
    TransferAdmin = 7,
    SetPoolFees = 8,
    OverrideCurveParam = 9,
    SetNewFeeOwner = 10,
    PartnerClaimFees = 11,
    WithdrawProtocolFees = 12,
    CreateLockEscrow = 13,
    Lock = 14,
    ClaimFee = 15,
    CreatePool = 16,
    EnableOrDisablePool = 17,
    BootstrapLiquidity = 18,
    MigrateFeeAccount = 19,
}

impl MeteoraPoolsInstruction {
    /// 从 discriminator 转换为指令类型
    pub fn from_discriminator(discriminator: &[u8; 8]) -> Option<Self> {
        match discriminator {
            &[175, 175, 109, 31, 13, 152, 155, 237] => Some(Self::Initialize),
            &[248, 198, 158, 145, 225, 117, 135, 200] => Some(Self::Swap),
            &[181, 157, 89, 67, 143, 182, 52, 72] => Some(Self::AddLiquidity),
            &[80, 85, 209, 72, 24, 206, 177, 108] => Some(Self::RemoveLiquidity),
            &[208, 127, 21, 1, 194, 190, 196, 70] => Some(Self::CreateConfig),
            &[123, 134, 81, 0, 49, 68, 98, 98] => Some(Self::CloseConfig),
            &[95, 180, 10, 172, 84, 174, 232, 40] => Some(Self::CreatePool),
            _ => None,
        }
    }
}

/// Meteora Pools discriminator 常量
pub mod discriminators {
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const ADD_LIQUIDITY: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [80, 85, 209, 72, 24, 206, 177, 108];
    pub const CREATE_CONFIG: [u8; 8] = [208, 127, 21, 1, 194, 190, 196, 70];
    pub const CLOSE_CONFIG: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    pub const CREATE_POOL: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];
}

/// Meteora Pools 程序 ID
pub const PROGRAM_ID: &str = "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB";

/// 主要的 Meteora Pools 指令解析函数
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
    let instruction_type = MeteoraPoolsInstruction::from_discriminator(&discriminator)?;
    let data = &instruction_data[8..];

    match instruction_type {
        MeteoraPoolsInstruction::Swap => {
            parse_swap_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraPoolsInstruction::AddLiquidity => {
            parse_add_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraPoolsInstruction::RemoveLiquidity => {
            parse_remove_liquidity_instruction(data, accounts, signature, slot, block_time)
        },
        MeteoraPoolsInstruction::CreatePool => {
            parse_create_pool_instruction(data, accounts, signature, slot, block_time)
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

    let in_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_out_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraPoolsSwap(MeteoraPoolsSwapEvent {
        metadata,
        in_amount,
        out_amount: 0, // 从日志中获取
        trade_fee: 0, // 从日志中获取
        protocol_fee: 0, // 从日志中获取
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

    let pool_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_b_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraPoolsAddLiquidity(MeteoraPoolsAddLiquidityEvent {
        metadata,
        lp_mint_amount: pool_token_amount,
        token_a_amount: 0, // 从日志中获取
        token_b_amount: 0, // 从日志中获取
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

    let pool_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_b_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent {
        metadata,
        lp_unmint_amount: pool_token_amount,
        token_a_out_amount: 0, // 从日志中获取
        token_b_out_amount: 0, // 从日志中获取
    }))
}

/// 解析 Create Pool 指令
fn parse_create_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let curve_type = read_u8(data, offset)?;
    offset += 1;

    let trade_fee_numerator = read_u64_le(data, offset)?;
    offset += 8;

    let trade_fee_denominator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_trade_fee_numerator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_trade_fee_denominator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_withdraw_fee_numerator = read_u64_le(data, offset)?;
    offset += 8;

    let owner_withdraw_fee_denominator = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let token_a_mint = get_account(accounts, 8)?;
    let token_b_mint = get_account(accounts, 9)?;
    let lp_mint = get_account(accounts, 4)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent {
        metadata,
        lp_mint,
        token_a_mint,
        token_b_mint,
        pool_type: curve_type,
        pool,
    }))
}