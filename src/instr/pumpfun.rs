//! PumpFun 指令解析器
//!
//! 使用 match discriminator 模式解析 PumpFun 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// PumpFun discriminator 常量
pub mod discriminators {
    pub const CREATE: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
}

/// PumpFun 程序 ID
pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

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
        creator: get_account(accounts, 2).unwrap_or_default(),
        virtual_token_reserves: 1_073_000_000_000_000,
        virtual_sol_reserves: 30_000_000_000,
        real_token_reserves: 0,
        token_total_supply: 1_000_000_000_000_000,
        timestamp: block_time.unwrap_or(0),
        mint_authority: get_account(accounts, 3).unwrap_or_default(),
        associated_bonding_curve: get_account(accounts, 4).unwrap_or_default(),
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

    let mint = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        user: get_account(accounts, 1).unwrap_or_default(),
        sol_amount,
        token_amount: 0, // 将从日志填充
        is_buy: true,
        bonding_curve: get_account(accounts, 2).unwrap_or_default(),
        virtual_sol_reserves: 30_000_000_000,
        virtual_token_reserves: 1_073_000_000_000_000,
        real_sol_reserves: 0,
        real_token_reserves: 793_100_000_000_000,
        fee_recipient: get_account(accounts, 3).unwrap_or_default(),
        fee_basis_points: 100,
        fee: sol_amount / 100, // 1% 费用
        creator: get_account(accounts, 4).unwrap_or_default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        timestamp: block_time.unwrap_or(0),
        last_update_timestamp: block_time.unwrap_or(0),
        track_volume: false,
        max_sol_cost,
        min_sol_output: 0,
        amount: 0, // 将从日志填充
        is_bot: false,
        is_dev_create_token_trade: false,
        global: get_account(accounts, 5).unwrap_or_default(),
        associated_bonding_curve: get_account(accounts, 6).unwrap_or_default(),
        associated_user: get_account(accounts, 7).unwrap_or_default(),
        system_program: get_account(accounts, 8).unwrap_or_default(),
        token_program: get_account(accounts, 9).unwrap_or_default(),
        creator_vault: get_account(accounts, 10).unwrap_or_default(),
        event_authority: get_account(accounts, 11).unwrap_or_default(),
        program: get_account(accounts, 12).unwrap_or_default(),
        global_volume_accumulator: get_account(accounts, 13).unwrap_or_default(),
        user_volume_accumulator: get_account(accounts, 14).unwrap_or_default(),
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

    let mint = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        user: get_account(accounts, 1).unwrap_or_default(),
        sol_amount: 0, // 将从日志填充
        token_amount,
        is_buy: false,
        bonding_curve: get_account(accounts, 2).unwrap_or_default(),
        virtual_sol_reserves: 30_000_000_000,
        virtual_token_reserves: 1_073_000_000_000_000,
        real_sol_reserves: 0,
        real_token_reserves: 793_100_000_000_000,
        fee_recipient: get_account(accounts, 3).unwrap_or_default(),
        fee_basis_points: 100,
        fee: token_amount / 100, // 1% 费用
        creator: get_account(accounts, 4).unwrap_or_default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        timestamp: block_time.unwrap_or(0),
        last_update_timestamp: block_time.unwrap_or(0),
        track_volume: false,
        max_sol_cost: 0,
        min_sol_output,
        amount: token_amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: get_account(accounts, 5).unwrap_or_default(),
        associated_bonding_curve: get_account(accounts, 6).unwrap_or_default(),
        associated_user: get_account(accounts, 7).unwrap_or_default(),
        system_program: get_account(accounts, 8).unwrap_or_default(),
        token_program: get_account(accounts, 9).unwrap_or_default(),
        creator_vault: get_account(accounts, 10).unwrap_or_default(),
        event_authority: get_account(accounts, 11).unwrap_or_default(),
        program: get_account(accounts, 12).unwrap_or_default(),
        global_volume_accumulator: get_account(accounts, 13).unwrap_or_default(),
        user_volume_accumulator: get_account(accounts, 14).unwrap_or_default(),
    }))
}