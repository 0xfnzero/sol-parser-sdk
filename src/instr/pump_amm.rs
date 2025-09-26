//! PumpSwap 指令解析器
//!
//! 使用 match discriminator 模式解析 PumpSwap 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// PumpSwap discriminator 常量
pub mod discriminators {
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
}

/// Pump AMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::PUMPSWAP_PROGRAM_ID;

/// 主要的 PumpSwap 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::BUY => {
            parse_buy_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        discriminators::SELL => {
            parse_sell_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        discriminators::CREATE_POOL => {
            parse_create_pool_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        _ => None,
    }
}

/// 解析买入指令
fn parse_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    let slippage = read_u16_le(data, offset)?;

    let token_mint = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, token_mint);

    Some(DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        metadata,
        pool_id: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        token_mint,
        sol_amount,
        token_amount: 0, // 将从日志填充
        price: 0, // 将从日志计算
        slippage,
    }))
}

/// 解析卖出指令
fn parse_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let slippage = read_u16_le(data, offset)?;

    let token_mint = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, token_mint);

    Some(DexEvent::PumpSwapSell(PumpSwapSellEvent {
        metadata,
        pool_id: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        token_mint,
        token_amount,
        sol_amount: 0, // 将从日志填充
        price: 0, // 将从日志计算
        slippage,
    }))
}

/// 解析池创建指令
fn parse_create_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let initial_sol_reserve = read_u64_le(data, offset)?;
    offset += 8;

    let initial_token_reserve = read_u64_le(data, offset)?;

    let token_mint = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, token_mint);

    Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
        metadata,
        pool_id: get_account(accounts, 2).unwrap_or_default(),
        creator: get_account(accounts, 1).unwrap_or_default(),
        token_mint,
        initial_sol_amount: initial_sol_reserve,
        initial_token_amount: initial_token_reserve,
        fee_rate: 100, // 默认费率
    }))
}