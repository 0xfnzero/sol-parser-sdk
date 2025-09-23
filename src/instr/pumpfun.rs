//! PumpFun 指令解析器
//!
//! 使用 match discriminator 模式解析 PumpFun 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// PumpFun discriminator 常量
pub mod discriminators {
    pub const CREATE: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
}

/// PumpFun 程序 ID (为了向后兼容保留字符串版本)
pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// PumpFun 程序 ID (优化版本 - 使用 Pubkey 常量)
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::PUMPFUN_PROGRAM_ID;

/// 主要的 PumpFun 指令解析函数
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
        discriminators::CREATE => {
            parse_create_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::BUY => {
            parse_buy_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::SELL => {
            parse_sell_instruction(data, accounts, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析创建指令
fn parse_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mint = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name: "Unknown".to_string(),
        symbol: "UNK".to_string(),
        uri: String::new(),
        mint,
        bonding_curve: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        virtual_token_reserves: 1_073_000_000_000_000,
        virtual_sol_reserves: 30_000_000_000,
    }))
}

/// 解析买入指令
fn parse_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    let max_sol_cost = read_u64_le(data, offset)?;

    let mint = get_account(accounts, 2)?; // mint is at index 2
    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        sol_amount,
        token_amount: 0, // 将从日志填充
        is_buy: true,
        user: get_account(accounts, 6).unwrap_or_default(), // user is at index 6
        timestamp: block_time.unwrap_or(0),
        virtual_sol_reserves: 30_000_000_000, // 默认值，将从日志覆盖
        virtual_token_reserves: 1_073_000_000_000_000, // 默认值，将从日志覆盖
        real_sol_reserves: 0, // 将从日志填充
        real_token_reserves: 0, // 将从日志填充

        bonding_curve: get_account(accounts, 3).unwrap_or_default(), // bondingCurve is at index 3
        max_sol_cost,
        min_sol_output: 0,
        amount: sol_amount,
        // PumpFun 相关账户信息 - 买入指令账户映射
        global: get_account(accounts, 0).unwrap_or_default(),              // 索引0: global
        associated_bonding_curve: get_account(accounts, 4).unwrap_or_default(), // 索引4: associatedBondingCurve
        associated_user: get_account(accounts, 5).unwrap_or_default(),     // 索引5: associatedUser
        creator_vault: Pubkey::default(),                                  // 买入指令中没有creator_vault
        event_authority: get_account(accounts, 10).unwrap_or_default(),    // 索引10: eventAuthority
        global_volume_accumulator: Pubkey::default(),                      // 这些字段在买入指令中不存在
        user_volume_accumulator: Pubkey::default(),
    }))
}

/// 解析卖出指令
fn parse_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let min_sol_output = read_u64_le(data, offset)?;

    let mint = get_account(accounts, 2)?; // mint is at index 2
    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        sol_amount: 0, // 将从日志填充
        token_amount,
        is_buy: false,
        user: get_account(accounts, 6).unwrap_or_default(), // user is at index 6
        timestamp: block_time.unwrap_or(0),
        virtual_sol_reserves: 30_000_000_000, // 默认值，将从日志覆盖
        virtual_token_reserves: 1_073_000_000_000_000, // 默认值，将从日志覆盖
        real_sol_reserves: 0, // 将从日志填充
        real_token_reserves: 0, // 将从日志填充

        bonding_curve: get_account(accounts, 3).unwrap_or_default(), // bondingCurve is at index 3
        max_sol_cost: 0,
        min_sol_output,
        amount: token_amount,
        // PumpFun 相关账户信息 - 卖出指令账户映射
        global: get_account(accounts, 0).unwrap_or_default(),              // 索引0: global
        associated_bonding_curve: get_account(accounts, 4).unwrap_or_default(), // 索引4: associatedBondingCurve
        associated_user: get_account(accounts, 5).unwrap_or_default(),     // 索引5: associatedUser
        creator_vault: Pubkey::default(),                                  // 卖出指令中没有creator_vault
        event_authority: get_account(accounts, 10).unwrap_or_default(),    // 索引10: eventAuthority
        global_volume_accumulator: Pubkey::default(),                      // 这些字段在卖出指令中不存在
        user_volume_accumulator: Pubkey::default(),
    }))
}