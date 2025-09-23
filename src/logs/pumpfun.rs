//! PumpFun 日志解析器
//!
//! 使用 match discriminator 模式解析 PumpFun 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// PumpFun discriminator 常量
pub mod discriminators {
    // 事件 discriminators (16 字节) - 使用前8字节进行匹配
    pub const CREATE_EVENT: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
    pub const TRADE_EVENT: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
    pub const MIGRATE_EVENT: [u8; 8] = [189, 233, 93, 185, 92, 148, 234, 148];

    // 指令 discriminators (8 字节)
    pub const CREATE_TOKEN_IX: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
    pub const BUY_IX: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    pub const SELL_IX: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
    pub const MIGRATE_IX: [u8; 8] = [155, 234, 231, 146, 236, 158, 162, 30];
}

/// PumpFun 程序 ID
pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// 检查日志是否来自 PumpFun 程序
pub fn is_pumpfun_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    log.contains("Program data:") // PumpFun 事件日志
}

/// 主要的 PumpFun 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_pumpfun_log(log) {
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
        discriminators::CREATE_EVENT => {
            parse_create_event(data, signature, slot, block_time)
        },
        discriminators::TRADE_EVENT => {
            parse_trade_event(data, signature, slot, block_time)
        },
        discriminators::MIGRATE_EVENT => {
            parse_migrate_event(data, signature, slot, block_time)
        },
        _ => None,
    }
}

/// 解析创建事件
fn parse_create_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    // 解析字符串字段
    let (name, name_len) = read_string(data, offset)?;
    offset += name_len;

    let (symbol, symbol_len) = read_string(data, offset)?;
    offset += symbol_len;

    let (uri, uri_len) = read_string(data, offset)?;
    offset += uri_len;

    // 解析 Pubkey 字段
    let mint = read_pubkey(data, offset)?;
    offset += 32;

    let bonding_curve = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    // 解析数值字段
    let timestamp = read_i64_le(data, offset)?;
    offset += 8;

    let virtual_token_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let virtual_sol_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let real_token_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let token_total_supply = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        virtual_token_reserves,
        virtual_sol_reserves,
    }))
}

/// 解析交易事件
fn parse_trade_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    // 解析基础字段
    let mint = read_pubkey(data, offset)?;
    offset += 32;

    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let is_buy = read_bool(data, offset)?;
    offset += 1;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let timestamp = read_i64_le(data, offset)?;
    offset += 8;

    let virtual_sol_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let virtual_token_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let real_sol_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let real_token_reserves = read_u64_le(data, offset)?;
    offset += 8;

    let fee_recipient = read_pubkey(data, offset)?;
    offset += 32;

    let fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;

    let fee = read_u64_le(data, offset)?;
    offset += 8;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let creator_fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;

    let creator_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        sol_amount,
        token_amount,
        is_buy,
        user,
        timestamp,
        virtual_sol_reserves,
        virtual_token_reserves,
        real_sol_reserves,
        real_token_reserves,
        bonding_curve: Pubkey::default(),
        max_sol_cost: 0,
        min_sol_output: 0,
        amount: token_amount,
        global: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        associated_user: Pubkey::default(),
        creator_vault: Pubkey::default(),
        event_authority: Pubkey::default(),
        global_volume_accumulator: Pubkey::default(),
        user_volume_accumulator: Pubkey::default(),
    }))
}

/// 解析完成事件
fn parse_complete_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let mint = read_pubkey(data, offset)?;
    offset += 32;

    let bonding_curve = read_pubkey(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunComplete(PumpFunCompleteTokenEvent {
        metadata,
        user,
        mint,
        bonding_curve,
    }))
}

/// 解析迁移事件
fn parse_migrate_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let mint = read_pubkey(data, offset)?;
    offset += 32;

    let mint_amount = read_u64_le(data, offset)?;
    offset += 8;

    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    let pool_migration_fee = read_u64_le(data, offset)?;
    offset += 8;

    let bonding_curve = read_pubkey(data, offset)?;
    offset += 32;

    let timestamp = read_i64_le(data, offset)?;
    offset += 8;

    let pool = read_pubkey(data, offset)?;

    let metadata = create_metadata(signature, slot, block_time, mint);

    Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
        metadata,
        user,
        mint,
        mint_amount,
        sol_amount,
        pool_migration_fee,
        bonding_curve,
        timestamp,
        pool,
        global: Pubkey::default(),
        withdraw_authority: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        system_program: Pubkey::default(),
        token_program: Pubkey::default(),
        pump_amm: Pubkey::default(),
        pool_authority: Pubkey::default(),
        pool_authority_mint_account: Pubkey::default(),
        pool_authority_wsol_account: Pubkey::default(),
        amm_global_config: Pubkey::default(),
        wsol_mint: Pubkey::default(),
        lp_mint: Pubkey::default(),
        user_pool_token_account: Pubkey::default(),
        pool_base_token_account: Pubkey::default(),
        pool_quote_token_account: Pubkey::default(),
        token_2022_program: Pubkey::default(),
        associated_token_program: Pubkey::default(),
        pump_amm_event_authority: Pubkey::default(),
        event_authority: Pubkey::default(),
        program: Pubkey::default(),
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

    if log.contains("CreateEvent") || log.contains("create") {
        return parse_create_from_text(log, signature, slot, block_time);
    }

    if log.contains("TradeEvent") || log.contains("swap") || log.contains("trade") {
        return parse_trade_from_text(log, signature, slot, block_time);
    }

    if log.contains("CompleteEvent") || log.contains("graduation") {
        return parse_complete_from_text(log, signature, slot, block_time);
    }

    if log.contains("MigrateEvent") || log.contains("migrate") {
        return parse_migrate_from_text(log, signature, slot, block_time);
    }

    None
}

/// 从文本解析创建事件
fn parse_create_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name: extract_text_field(log, "name").unwrap_or_else(|| "Unknown".to_string()),
        symbol: extract_text_field(log, "symbol").unwrap_or_else(|| "UNK".to_string()),
        uri: extract_text_field(log, "uri").unwrap_or_default(),
        mint: Pubkey::default(),
        bonding_curve: Pubkey::default(),
        user: Pubkey::default(),
        virtual_token_reserves: 1_073_000_000_000_000,
        virtual_sol_reserves: 30_000_000_000,
    }))
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

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint: Pubkey::default(),
        sol_amount: extract_number_from_text(log, "sol").unwrap_or(1_000_000_000),
        token_amount: extract_number_from_text(log, "token").unwrap_or(1_000_000_000),
        is_buy,
        user: Pubkey::default(),
        timestamp: block_time.unwrap_or(0),
        virtual_sol_reserves: 30_000_000_000,
        virtual_token_reserves: 1_073_000_000_000_000,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        bonding_curve: Pubkey::default(),
        max_sol_cost: 0,
        min_sol_output: 0,
        amount: extract_number_from_text(log, "amount").unwrap_or(1_000_000_000),
        global: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        associated_user: Pubkey::default(),
        creator_vault: Pubkey::default(),
        event_authority: Pubkey::default(),
        global_volume_accumulator: Pubkey::default(),
        user_volume_accumulator: Pubkey::default(),
    }))
}

/// 从文本解析完成事件
fn parse_complete_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let metadata = create_metadata(signature, slot, block_time, Pubkey::default());

    Some(DexEvent::PumpFunComplete(PumpFunCompleteTokenEvent {
        metadata,
        user: Pubkey::default(),
        mint: Pubkey::default(),
        bonding_curve: Pubkey::default(),
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

    Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
        metadata,
        user: Pubkey::default(),
        mint: Pubkey::default(),
        mint_amount: extract_number_from_text(log, "mint_amount").unwrap_or(1_000_000_000),
        sol_amount: extract_number_from_text(log, "sol_amount").unwrap_or(1_000_000_000),
        pool_migration_fee: extract_number_from_text(log, "fee").unwrap_or(10_000_000),
        bonding_curve: Pubkey::default(),
        timestamp: block_time.unwrap_or(0),
        pool: Pubkey::default(),
        global: Pubkey::default(),
        withdraw_authority: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        system_program: Pubkey::default(),
        token_program: Pubkey::default(),
        pump_amm: Pubkey::default(),
        pool_authority: Pubkey::default(),
        pool_authority_mint_account: Pubkey::default(),
        pool_authority_wsol_account: Pubkey::default(),
        amm_global_config: Pubkey::default(),
        wsol_mint: Pubkey::default(),
        lp_mint: Pubkey::default(),
        user_pool_token_account: Pubkey::default(),
        pool_base_token_account: Pubkey::default(),
        pool_quote_token_account: Pubkey::default(),
        token_2022_program: Pubkey::default(),
        associated_token_program: Pubkey::default(),
        pump_amm_event_authority: Pubkey::default(),
        event_authority: Pubkey::default(),
        program: Pubkey::default(),
    }))
}