//! Bonk 指令解析器
//!
//! 使用 match discriminator 模式解析 Bonk 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Bonk discriminator 常量
pub mod discriminators {
    pub const TRADE: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
    pub const POOL_CREATE: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const MIGRATE_AMM: [u8; 8] = [3, 4, 5, 6, 7, 8, 9, 10];
}

/// Bonk 程序 ID
pub const PROGRAM_ID: &str = "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1";

/// 主要的 Bonk 指令解析函数
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
        discriminators::TRADE => {
            parse_trade_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::POOL_CREATE => {
            parse_pool_create_instruction(data, accounts, signature, slot, block_time)
        },
        discriminators::MIGRATE_AMM => {
            parse_migrate_amm_instruction(data, accounts, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析交易指令
fn parse_trade_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out_min = read_u64_le(data, offset)?;

    let pool_state = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state,
        user: get_account(accounts, 1).unwrap_or_default(),
        amount_in,
        amount_out: 0, // 将从日志填充
        is_buy: true, // 默认为买入，实际值从日志确定
        trade_direction: TradeDirection::Buy,
        exact_in: true,
    }))
}

/// 解析池创建指令
fn parse_pool_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let pool_state = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::BonkPoolCreate(BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol: "BONK".to_string(),
            name: "Bonk Pool".to_string(),
            uri: "https://bonk.com".to_string(),
            decimals: 5,
        },
        pool_state,
        creator: get_account(accounts, 1).unwrap_or_default(),
    }))
}

/// 解析 AMM 迁移指令
fn parse_migrate_amm_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_amount = read_u64_le(data, offset)?;

    let old_pool = get_account(accounts, 0)?;
    let metadata = create_metadata(signature, slot, block_time, old_pool);

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool,
        new_pool: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        liquidity_amount,
    }))
}