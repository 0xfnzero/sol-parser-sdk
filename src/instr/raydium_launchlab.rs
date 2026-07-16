//! Raydium LaunchLab 指令解析器
//!
//! 底层按 `idls/raydium_launchpad.json` 的真实 instruction discriminator
//! 和账户布局解析，对外事件名统一为 `RaydiumLaunchlab*`。

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium LaunchLab instruction discriminators from `idls/raydium_launchpad.json`.
pub mod discriminators {
    pub const BUY_EXACT_IN: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];
    pub const BUY_EXACT_OUT: [u8; 8] = [24, 211, 116, 40, 105, 3, 153, 56];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];
    pub const INITIALIZE_WITH_TOKEN_2022: [u8; 8] = [37, 190, 126, 222, 44, 154, 171, 17];
    pub const MIGRATE_TO_AMM: [u8; 8] = [207, 82, 192, 145, 254, 207, 145, 223];
    pub const MIGRATE_TO_CPSWAP: [u8; 8] = [136, 92, 200, 103, 28, 218, 144, 140];
    pub const SELL_EXACT_IN: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];
    pub const SELL_EXACT_OUT: [u8; 8] = [95, 200, 71, 34, 8, 9, 11, 166];
}

/// Raydium LaunchLab 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID;

/// 主要的 Raydium LaunchLab 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::BUY_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            true,
        ),
        discriminators::BUY_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            false,
        ),
        discriminators::SELL_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            true,
        ),
        discriminators::SELL_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            false,
        ),
        discriminators::INITIALIZE
        | discriminators::INITIALIZE_V2
        | discriminators::INITIALIZE_WITH_TOKEN_2022 => {
            parse_pool_create_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        // The LaunchLab IDL does not expose enough fields to synthesize a
        // migration event with the SDK's migrate layout.
        discriminators::MIGRATE_TO_AMM | discriminators::MIGRATE_TO_CPSWAP => None,
        _ => None,
    }
}

/// 解析 buy/sell 指令。
///
/// 外层指令只携带用户输入的 amount / min-max amount；真实成交量由 log 事件覆盖。
fn parse_trade_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    is_buy: bool,
    exact_in: bool,
) -> Option<DexEvent> {
    let first_amount = read_u64_le(data, 0)?;
    let second_amount = read_u64_le(data, 8)?;

    let (amount_in, amount_out) =
        if exact_in { (first_amount, second_amount) } else { (second_amount, first_amount) };

    let pool_state = get_account(accounts, 4)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
        metadata,
        pool_state,
        user: get_account(accounts, 0).unwrap_or_default(),
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
    }))
}

/// 解析 initialize / initialize_v2 / initialize_with_token_2022 指令。
fn parse_pool_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let base_mint_param = parse_mint_params(data)?;

    let pool_state = get_account(accounts, 5)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent {
        metadata,
        base_mint_param,
        pool_state,
        creator: get_account(accounts, 1).unwrap_or_default(),
    }))
}

fn parse_mint_params(data: &[u8]) -> Option<BaseMintParam> {
    let mut offset = 0usize;
    let decimals = *data.get(offset)?;
    offset += 1;
    let name = read_borsh_string(data, &mut offset)?;
    let symbol = read_borsh_string(data, &mut offset)?;
    let uri = read_borsh_string(data, &mut offset)?;
    Some(BaseMintParam { symbol, name, uri, decimals })
}

fn read_borsh_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    let end = (*offset).checked_add(len)?;
    let bytes = data.get(*offset..end)?;
    *offset = end;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}
