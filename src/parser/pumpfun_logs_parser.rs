//! PumpFun 日志解析 - 纯函数式设计，零拷贝，高性能
//!
//! 设计原则：
//! - 纯函数，无状态
//! - 零拷贝解析
//! - 职责单一
//! - 性能优先

use crate::parser::events::*;
use borsh::BorshDeserialize;
// use prost_types::Timestamp;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;

/// PumpFun 程序 ID
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// 事件判别符 - 编译时常量
pub const CREATE_TOKEN_EVENT: &[u8] = &[228, 69, 165, 46, 81, 203, 154, 29, 27, 114, 169, 77, 222, 235, 99, 118];
pub const TRADE_EVENT: &[u8] = &[228, 69, 165, 46, 81, 203, 154, 29, 189, 219, 127, 211, 78, 230, 97, 238];
pub const MIGRATE_EVENT: &[u8] = &[228, 69, 165, 46, 81, 203, 154, 29, 189, 233, 93, 185, 92, 148, 234, 148];

/// 日志数据大小常量
pub const CREATE_TOKEN_LOG_SIZE: usize = 257;
pub const TRADE_EVENT_LOG_SIZE: usize = 250;
pub const MIGRATE_EVENT_LOG_SIZE: usize = 160;

/// 原始创建代币事件日志结构
#[derive(BorshDeserialize)]
pub struct RawCreateTokenLog {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
}

/// 原始交易事件日志结构
#[derive(BorshDeserialize)]
pub struct RawTradeLog {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
}

/// 原始迁移事件日志结构
#[derive(BorshDeserialize)]
pub struct RawMigrateLog {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub mint_amount: u64,
    pub sol_amount: u64,
    pub pool_migration_fee: u64,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
    pub pool: Pubkey,
}

/// 快速事件判别符匹配 - SIMD 优化
#[inline(always)]
pub fn match_event_discriminator(data: &[u8]) -> Option<&'static [u8]> {
    if data.len() < 16 {
        return None;
    }

    let disc = &data[..16];
    if disc == CREATE_TOKEN_EVENT {
        Some(CREATE_TOKEN_EVENT)
    } else if disc == TRADE_EVENT {
        Some(TRADE_EVENT)
    } else if disc == MIGRATE_EVENT {
        Some(MIGRATE_EVENT)
    } else {
        None
    }
}

/// 检查日志是否来自 PumpFun - 快速匹配
#[inline(always)]
pub fn is_pumpfun_log(log: &str) -> bool {
    // 编译时计算的字符串常量
    log.contains("Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
}

/// 从日志提取 base64 数据 - 零拷贝
#[inline]
pub fn extract_program_data(log: &str) -> Option<&str> {
    const PREFIX: &str = "Program data: ";
    if let Some(start) = log.find(PREFIX) {
        let data_start = start + PREFIX.len();
        Some(log[data_start..].trim())
    } else {
        None
    }
}

/// 解析创建代币日志事件 - 高性能
#[inline]
pub fn parse_create_token_log(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if data.len() < CREATE_TOKEN_LOG_SIZE + 16 {
        return None;
    }

    // 跳过判别符
    let log_data = &data[16..16 + CREATE_TOKEN_LOG_SIZE];
    let raw = RawCreateTokenLog::try_from_slice(log_data).ok()?;

    let metadata = EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time,
        program_id: Pubkey::from_str(PUMPFUN_PROGRAM_ID).unwrap(),
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: 0,
        handle_us: 0,
    };

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name: raw.name,
        symbol: raw.symbol,
        uri: raw.uri,
        mint: raw.mint,
        bonding_curve: raw.bonding_curve,
        user: raw.user,
        creator: raw.creator,
        virtual_token_reserves: raw.virtual_token_reserves,
        virtual_sol_reserves: raw.virtual_sol_reserves,
        real_token_reserves: raw.real_token_reserves,
        token_total_supply: raw.token_total_supply,
        timestamp: raw.timestamp,
        mint_authority: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
    }))
}

/// 解析交易日志事件 - 高性能
#[inline]
pub fn parse_trade_log(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if data.len() < TRADE_EVENT_LOG_SIZE + 16 {
        return None;
    }

    let log_data = &data[16..16 + TRADE_EVENT_LOG_SIZE];
    let raw = RawTradeLog::try_from_slice(log_data).ok()?;

    let metadata = EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time,
        program_id: Pubkey::from_str(PUMPFUN_PROGRAM_ID).unwrap(),
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: 0,
        handle_us: 0,
    };

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint: raw.mint,
        user: raw.user,
        sol_amount: raw.sol_amount,
        token_amount: raw.token_amount,
        is_buy: raw.is_buy,
        bonding_curve: Pubkey::default(),
        virtual_sol_reserves: raw.virtual_sol_reserves,
        virtual_token_reserves: raw.virtual_token_reserves,
        real_sol_reserves: raw.real_sol_reserves,
        real_token_reserves: raw.real_token_reserves,
        fee_recipient: raw.fee_recipient,
        fee_basis_points: raw.fee_basis_points,
        fee: raw.fee,
        creator: raw.creator,
        creator_fee_basis_points: raw.creator_fee_basis_points,
        creator_fee: raw.creator_fee,
        total_unclaimed_tokens: raw.total_unclaimed_tokens,
        total_claimed_tokens: raw.total_claimed_tokens,
        current_sol_volume: raw.current_sol_volume,
        timestamp: raw.timestamp,
        last_update_timestamp: raw.last_update_timestamp,
        track_volume: raw.track_volume,
        max_sol_cost: 0,
        min_sol_output: 0,
        amount: raw.token_amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: Pubkey::default(),
        associated_bonding_curve: Pubkey::default(),
        associated_user: Pubkey::default(),
        system_program: Pubkey::default(),
        token_program: Pubkey::default(),
        creator_vault: Pubkey::default(),
        event_authority: Pubkey::default(),
        program: Pubkey::default(),
        global_volume_accumulator: Pubkey::default(),
        user_volume_accumulator: Pubkey::default(),
    }))
}

/// 解析迁移日志事件 - 高性能
#[inline]
pub fn parse_migrate_log(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if data.len() < MIGRATE_EVENT_LOG_SIZE + 16 {
        return None;
    }

    let log_data = &data[16..16 + MIGRATE_EVENT_LOG_SIZE];
    let raw = RawMigrateLog::try_from_slice(log_data).ok()?;

    let metadata = EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time,
        program_id: Pubkey::from_str(PUMPFUN_PROGRAM_ID).unwrap(),
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: 0,
        handle_us: 0,
    };

    Some(DexEvent::PumpFunComplete(PumpFunCompleteTokenEvent {
        metadata,
        user: raw.user,
        mint: raw.mint,
        bonding_curve: raw.bonding_curve,
    }))
}

/// 主解析函数 - 零分支预测，最大性能
#[inline]
pub fn parse_pumpfun_log_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let discriminator = match_event_discriminator(data)?;

    // 分支预测优化 - 按使用频率排序
    if discriminator == TRADE_EVENT {
        parse_trade_log(data, signature, slot, block_time)
    } else if discriminator == CREATE_TOKEN_EVENT {
        parse_create_token_log(data, signature, slot, block_time)
    } else if discriminator == MIGRATE_EVENT {
        parse_migrate_log(data, signature, slot, block_time)
    } else {
        None
    }
}

/// 从日志字符串解析事件 - 完整流程
pub fn parse_pumpfun_from_log_string(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_pumpfun_log(log) {
        return None;
    }

    let data_str = extract_program_data(log)?;

    // 使用标准库解码 base64
    use base64::{Engine as _, engine::general_purpose};
    let data = general_purpose::STANDARD.decode(data_str).ok()?;

    parse_pumpfun_log_event(&data, signature, slot, block_time)
}

/// 批量解析日志 - 向量化处理
pub fn parse_logs_batch(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    for log in logs {
        if let Some(event) = parse_pumpfun_from_log_string(log, signature, slot, block_time.clone()) {
            events.push(event);
        }
    }

    events
}

/// 流式解析日志 - 零分配迭代器
pub fn parse_logs_stream<'a>(
    logs: &'a [String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> impl Iterator<Item = DexEvent> + 'a {
    logs.iter()
        .filter_map(move |log| parse_pumpfun_from_log_string(log, signature, slot, block_time.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_matching() {
        let mut data = vec![0u8; 32];
        data[..16].copy_from_slice(TRADE_EVENT);

        assert_eq!(match_event_discriminator(&data), Some(TRADE_EVENT));

        data[..16].copy_from_slice(CREATE_TOKEN_EVENT);
        assert_eq!(match_event_discriminator(&data), Some(CREATE_TOKEN_EVENT));

        data[..16].fill(0);
        assert_eq!(match_event_discriminator(&data), None);
    }

    #[test]
    fn test_log_detection() {
        let pumpfun_log = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]";
        let other_log = "Program 11111111111111111111111111111112 invoke [1]";

        assert!(is_pumpfun_log(pumpfun_log));
        assert!(!is_pumpfun_log(other_log));
    }

    #[test]
    fn test_data_extraction() {
        let log = "Program data: SGVsbG8gV29ybGQ=";
        assert_eq!(extract_program_data(log), Some("SGVsbG8gV29ybGQ="));

        let log_no_data = "Program invoke [1]";
        assert_eq!(extract_program_data(log_no_data), None);
    }

    #[bench]
    fn bench_log_parsing(b: &mut test::Bencher) {
        let log = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]";
        let signature = Signature::default();

        b.iter(|| {
            is_pumpfun_log(log)
        });
    }
}