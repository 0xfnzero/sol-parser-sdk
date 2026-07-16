//! Raydium LaunchLab 日志解析器
//!
//! 底层按 `idls/raydium_launchpad.json` 的真实 event discriminator 和 Borsh
//! 布局解析，对外事件名统一为 `RaydiumLaunchlab*`。

use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium LaunchLab event discriminators from `idls/raydium_launchpad.json`.
pub mod discriminators {
    pub const CLAIM_VESTED: [u8; 8] = [21, 194, 114, 87, 120, 211, 226, 32];
    pub const CREATE_VESTING: [u8; 8] = [150, 152, 11, 179, 52, 210, 191, 125];
    pub const POOL_CREATE: [u8; 8] = [151, 215, 226, 9, 118, 161, 115, 174];
    pub const TRADE: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
}

/// Raydium LaunchLab 程序 ID
pub const PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";

/// 检查日志是否来自 Raydium LaunchLab 程序
pub fn is_raydium_launchlab_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID))
        || log.contains(&format!("Program {} success", PROGRAM_ID))
}

/// 主要的 Raydium LaunchLab 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    };

    match discriminator {
        discriminators::TRADE => parse_trade_from_data(data, metadata),
        discriminators::POOL_CREATE => parse_pool_create_from_data(data, metadata),
        _ => None,
    }
}

/// Parse Raydium LaunchLab TradeEvent from pre-decoded event data.
#[inline]
pub fn parse_trade_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const TRADE_EVENT_LEN: usize = 32 + 13 * 8 + 3;
    if data.len() != TRADE_EVENT_LEN {
        return None;
    }

    let pool_state = read_pubkey(data, 0)?;
    let amount_in = read_u64_le(data, 88)?;
    let amount_out = read_u64_le(data, 96)?;
    let trade_direction = *data.get(136)?;
    if trade_direction > 1 {
        return None;
    }
    let pool_status = *data.get(137)?;
    if pool_status > 2 {
        return None;
    }
    let exact_in_raw = *data.get(138)?;
    if exact_in_raw > 1 {
        return None;
    }
    let exact_in = exact_in_raw == 1;
    let is_buy = trade_direction == 0;

    Some(DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
        metadata,
        pool_state,
        user: Pubkey::default(),
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
    }))
}

/// Parse Raydium LaunchLab PoolCreateEvent from pre-decoded event data.
#[inline]
pub fn parse_pool_create_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0usize;
    let pool_state = read_pubkey(data, offset)?;
    offset += 32;
    let creator = read_pubkey(data, offset)?;
    offset += 32;
    let _config = read_pubkey(data, offset)?;
    offset += 32;
    let base_mint_param = parse_mint_params(data, &mut offset)?;

    Some(DexEvent::RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent {
        metadata,
        base_mint_param,
        pool_state,
        creator,
    }))
}

fn parse_mint_params(data: &[u8], offset: &mut usize) -> Option<BaseMintParam> {
    let decimals = *data.get(*offset)?;
    *offset += 1;
    let name = read_borsh_string(data, offset)?;
    let symbol = read_borsh_string(data, offset)?;
    let uri = read_borsh_string(data, offset)?;
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
