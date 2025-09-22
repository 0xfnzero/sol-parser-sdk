//! Raydium CPMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium CPMM 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Raydium CPMM discriminator 常量
pub mod discriminators {
    pub const SWAP_BASE_IN: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
    pub const SWAP_BASE_OUT: [u8; 8] = [55, 217, 98, 86, 163, 74, 180, 173];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
    pub const WITHDRAW: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
}

/// Raydium CPMM 程序 ID
pub const PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// 主要的 Raydium CPMM 指令解析函数
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
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::SWAP_BASE_IN => {
            parse_swap_base_in_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::SWAP_BASE_OUT => {
            parse_swap_base_out_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::INITIALIZE => {
            parse_initialize_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::DEPOSIT => {
            parse_deposit_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::WITHDRAW => {
            parse_withdraw_instruction(data, accounts, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析 Base In 交换指令
fn parse_swap_base_in_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        amount_in,
        amount_out: 0, // 将从日志填充
        is_base_input: true,
    }))
}

/// 解析 Base Out 交换指令
fn parse_swap_base_out_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let maximum_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        amount_in: 0, // 将从日志填充
        amount_out,
        is_base_input: false,
    }))
}

/// 解析初始化指令
fn parse_initialize_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let init_amount0 = read_u64_le(data, offset)?;
    offset += 8;

    let init_amount1 = read_u64_le(data, offset)?;
    offset += 8;

    let open_time = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
        metadata,
        pool,
        creator: get_account(accounts, 1).unwrap_or_default(),
        init_amount0,
        init_amount1,
    }))
}

/// 解析存款指令
fn parse_deposit_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_1_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        lp_token_amount,
        token0_amount: 0, // 将从日志填充
        token1_amount: 0, // 将从日志填充
    }))
}

/// 解析提款指令
fn parse_withdraw_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_1_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool);

    Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        lp_token_amount,
        token0_amount: 0, // 将从日志填充
        token1_amount: 0, // 将从日志填充
    }))
}