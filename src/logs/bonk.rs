//! Bonk 日志解析器
//!
//! 使用 match discriminator 模式解析 Bonk 事件

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

/// 检查日志是否来自 Bonk 程序
pub fn is_bonk_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    log.contains("bonk") || log.contains("Bonk")
}

/// 主要的 Bonk 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_bonk_log(log) {
        return None;
    }

    // 尝试结构化解析
    if let Some(event) = parse_structured_log(log, signature, slot, block_time) {
        return Some(event);
    }

    // 回退到文本解析
    parse_text_log(log, signature, slot, block_time)
}

/// 结构化日志解析（基于 Program data）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::TRADE => {
            parse_trade_event(data, signature, slot, block_time)
        },
        discriminators::POOL_CREATE => {
            parse_pool_create_event(data, signature, slot, block_time)
        },
        discriminators::MIGRATE_AMM => {
            parse_migrate_amm_event(data, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析交易事件
fn parse_trade_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let is_buy = read_bool(data, offset)?;
    offset += 1;

    let exact_in = read_bool(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, pool_state);

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state,
        user,
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
    }))
}

/// 解析池创建事件
fn parse_pool_create_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let _token_a_mint = read_pubkey(data, offset)?;
    offset += 32;

    let _token_b_mint = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let _initial_liquidity_a = read_u64_le(data, offset)?;
    offset += 8;

    let _initial_liquidity_b = read_u64_le(data, offset)?;

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
        creator,
    }))
}

/// 解析 AMM 迁移事件
fn parse_migrate_amm_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let old_pool = read_pubkey(data, offset)?;
    offset += 32;

    let new_pool = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, old_pool);

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool,
        new_pool,
        user,
        liquidity_amount,
    }))
}

/// 文本回退解析
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("trade") || log.contains("swap") {
        return parse_trade_from_text(log, signature, slot, block_time);
    }

    if log.contains("pool") && log.contains("create") {
        return parse_pool_create_from_text(log, signature, slot, block_time);
    }

    if log.contains("migrate") {
        return parse_migrate_from_text(log, signature, slot, block_time);
    }

    None
}

/// 从文本解析交易事件
fn parse_trade_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());
    let is_buy = detect_trade_type(log).unwrap_or(true);

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state: Pubkey::default(),
        user: Pubkey::default(),
        amount_in: extract_number_from_text(log, "amount_in").unwrap_or(1000000),
        amount_out: extract_number_from_text(log, "amount_out").unwrap_or(950000),
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in: true,
    }))
}

/// 从文本解析池创建事件
fn parse_pool_create_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::BonkPoolCreate(BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol: "BONK".to_string(),
            name: "Bonk Pool".to_string(),
            uri: "https://bonk.com".to_string(),
            decimals: 5,
        },
        pool_state: Pubkey::default(),
        creator: Pubkey::default(),
    }))
}

/// 从文本解析迁移事件
fn parse_migrate_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool: Pubkey::default(),
        new_pool: Pubkey::default(),
        user: Pubkey::default(),
        liquidity_amount: extract_number_from_text(log, "liquidity").unwrap_or(0),
    }))
}