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
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if !is_pumpfun_log(log) {
        return None;
    }

    // 尝试结构化解析
    if let Some(event) = parse_structured_log(log, signature, slot, block_time, grpc_recv_us) {
        return Some(event);
    }

    // 回退到文本解析
    parse_text_log(log, signature, slot, block_time, grpc_recv_us)
}

/// 快速路径：只检查 discriminator，避免完整解码（用于事件类型过滤）
#[inline]
pub fn parse_log_fast_filter(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
    wanted_discriminator: [u8; 8],
) -> Option<DexEvent> {
    // 快速提取 discriminator（只解码前8字节）
    let discriminator = extract_discriminator_fast(log)?;

    // 如果不是想要的类型，立即返回
    if discriminator != wanted_discriminator {
        return None;
    }

    // 是想要的类型，完整解析
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
        discriminators::CREATE_EVENT => {
            parse_create_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::TRADE_EVENT => {
            parse_trade_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::MIGRATE_EVENT => {
            parse_migrate_event(data, signature, slot, block_time, grpc_recv_us)
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
    grpc_recv_us: i64,
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

    let metadata = create_metadata_simple(signature, slot, block_time, mint, grpc_recv_us);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        creator,
        timestamp,
        virtual_token_reserves,
        virtual_sol_reserves,
        real_token_reserves,
        token_total_supply,
    }))
}

/// 解析交易事件
fn parse_trade_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
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
    offset += 8;

    let track_volume = read_bool(data, offset).unwrap_or(false);
    offset += 1;

    let total_unclaimed_tokens = read_u64_le(data, offset).unwrap_or(0);
    offset += 8;

    let total_claimed_tokens = read_u64_le(data, offset).unwrap_or(0);
    offset += 8;

    let current_sol_volume = read_u64_le(data, offset).unwrap_or(0);

    let metadata = create_metadata_simple(signature, slot, block_time, mint, grpc_recv_us);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,

        // IDL TradeEvent 字段
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
        fee_recipient,
        fee_basis_points,
        fee,
        creator,
        creator_fee_basis_points,
        creator_fee,
        track_volume,
        total_unclaimed_tokens,
        total_claimed_tokens,
        current_sol_volume,
        last_update_timestamp: timestamp,

        // 指令账户字段 - 默认值，由account_filler填充
        global: Pubkey::default(),
        bonding_curve: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        associated_user: Pubkey::default(),
    }))
}

/// 解析完成事件
fn parse_complete_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let mint = read_pubkey(data, offset)?;
    offset += 32;

    let bonding_curve = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, mint, grpc_recv_us);

    let timestamp = read_i64_le(data, offset)?;

    Some(DexEvent::PumpFunComplete(PumpFunCompleteTokenEvent {
        metadata,
        user,
        mint,
        bonding_curve,
        timestamp,
    }))
}

/// 解析迁移事件
fn parse_migrate_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
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

    let metadata = create_metadata_simple(signature, slot, block_time, mint, grpc_recv_us);

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

    if log.contains("CreateEvent") || log.contains("create") {
        return parse_create_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("TradeEvent") || log.contains("swap") || log.contains("trade") {
        return parse_trade_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("CompleteEvent") || log.contains("graduation") {
        return parse_complete_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("MigrateEvent") || log.contains("migrate") {
        return parse_migrate_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    None
}

/// 从文本解析创建事件
fn parse_create_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name: extract_text_field(log, "name").unwrap_or_else(|| "Unknown".to_string()),
        symbol: extract_text_field(log, "symbol").unwrap_or_else(|| "UNK".to_string()),
        uri: extract_text_field(log, "uri").unwrap_or_default(),
        mint: Pubkey::default(),
        bonding_curve: Pubkey::default(),
        user: Pubkey::default(),
        creator: Pubkey::default(),
        timestamp: block_time.unwrap_or(0),
        virtual_token_reserves: 1_073_000_000_000_000,
        virtual_sol_reserves: 30_000_000_000,
        real_token_reserves: 0,
        token_total_supply: 0,
    }))
}

/// 从文本解析交易事件
fn parse_trade_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);
    let is_buy = detect_trade_type(log).unwrap_or(true);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,

        // IDL TradeEvent 字段
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
        fee_recipient: Pubkey::default(),
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: block_time.unwrap_or(0),

        // 指令账户字段
        global: Pubkey::default(),
        bonding_curve: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        associated_user: Pubkey::default(),
    }))
}

/// 从文本解析完成事件
fn parse_complete_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::PumpFunComplete(PumpFunCompleteTokenEvent {
        metadata,
        user: Pubkey::default(),
        mint: Pubkey::default(),
        bonding_curve: Pubkey::default(),
        timestamp: block_time.unwrap_or(0),
    }))
}

/// 从文本解析迁移事件
fn parse_migrate_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

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
    }))
}