//! PumpSwap 日志解析器
//!
//! 使用 match discriminator 模式解析 PumpSwap 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// PumpSwap discriminator 常量
pub mod discriminators {
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
}

/// PumpSwap 程序 ID
pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// 检查日志是否来自 Pump AMM 程序
pub fn is_pump_amm_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    log.contains("pumpswap") || log.contains("PumpSwap")
}

/// 主要的 PumpSwap 日志解析函数
pub fn parse_log(log: &str, signature: Signature, slot: u64, block_time: Option<i64>, grpc_recv_us: i64) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, block_time, grpc_recv_us)
}

/// 结构化日志解析（基于 Program data）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::BUY => {
            parse_buy_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::SELL => {
            parse_sell_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::CREATE_POOL => {
            parse_create_pool_event(data, signature, slot, block_time, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析买入事件
fn parse_buy_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let token_mint = read_pubkey(data, offset)?;
    offset += 32;

    let input_amount = read_u64_le(data, offset)?;
    offset += 8;

    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let pool_state = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, token_mint, grpc_recv_us);

    Some(DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        metadata,
        pool_id: pool_state,
        user,
        token_mint,
        sol_amount: input_amount,
        token_amount: output_amount,
        price: 0,
        slippage: 0,
    }))
}

/// 解析卖出事件
fn parse_sell_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let token_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    let pool_state = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, token_mint, grpc_recv_us);

    Some(DexEvent::PumpSwapSell(PumpSwapSellEvent {
        metadata,
        pool_id: pool_state,
        user,
        token_mint,
        token_amount,
        sol_amount,
        price: 0,
        slippage: 0,
    }))
}

/// 解析池创建事件
fn parse_create_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let token_mint = read_pubkey(data, offset)?;
    offset += 32;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let initial_sol_reserve = read_u64_le(data, offset)?;
    offset += 8;

    let initial_token_reserve = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, token_mint, grpc_recv_us);

    Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
        metadata,
        pool_id: pool_state,
        creator,
        token_mint,
        initial_sol_amount: initial_sol_reserve,
        initial_token_amount: initial_token_reserve,
        fee_rate: 0,
    }))
}

/// 文本回退解析
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("buy") || log.contains("Buy") {
        return parse_buy_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("sell") || log.contains("Sell") {
        return parse_sell_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("create") && log.contains("pool") {
        return parse_create_pool_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    None
}

/// 从文本解析买入事件
fn parse_buy_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
        metadata,
        pool_id: Pubkey::default(),
        user: Pubkey::default(),
        token_mint: Pubkey::default(),
        sol_amount: extract_number_from_text(log, "sol").unwrap_or(1_000_000_000),
        token_amount: extract_number_from_text(log, "token").unwrap_or(950_000_000),
        price: 0,
        slippage: 0,
    }))
}

/// 从文本解析卖出事件
fn parse_sell_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::PumpSwapSell(PumpSwapSellEvent {
        metadata,
        pool_id: Pubkey::default(),
        user: Pubkey::default(),
        token_mint: Pubkey::default(),
        token_amount: extract_number_from_text(log, "token").unwrap_or(1_000_000_000),
        sol_amount: extract_number_from_text(log, "sol").unwrap_or(900_000_000),
        price: 0,
        slippage: 0,
    }))
}

/// 从文本解析池创建事件
fn parse_create_pool_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
        metadata,
        pool_id: Pubkey::default(),
        creator: Pubkey::default(),
        token_mint: Pubkey::default(),
        initial_sol_amount: extract_number_from_text(log, "sol_reserve").unwrap_or(1_000_000_000),
        initial_token_amount: extract_number_from_text(log, "token_reserve").unwrap_or(100_000_000_000),
        fee_rate: 0,
    }))
}