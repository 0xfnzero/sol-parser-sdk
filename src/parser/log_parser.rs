//! 日志解析器 - 从旧版本集成日志数据解析功能
//!
//! 这个模块负责从交易日志中解析事件数据，配合指令解析器提供完整的事件信息

use crate::parser::events::*;
use borsh::BorshDeserialize;
use prost_types::Timestamp;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// PumpFun 日志解析器
pub mod pumpfun_logs {
    use super::*;

    // 从旧版本复制的常量
    pub const PUMPFUN_CREATE_TOKEN_EVENT_LOG_SIZE: usize = 257;
    pub const PUMPFUN_TRADE_EVENT_LOG_SIZE: usize = 250;
    pub const PUMPFUN_MIGRATE_EVENT_LOG_SIZE: usize = 160;

    // 事件判别符（从旧版本复制）
    pub const CREATE_TOKEN_EVENT: &[u8] =
        &[228, 69, 165, 46, 81, 203, 154, 29, 27, 114, 169, 77, 222, 235, 99, 118];
    pub const TRADE_EVENT: &[u8] =
        &[228, 69, 165, 46, 81, 203, 154, 29, 189, 219, 127, 211, 78, 230, 97, 238];
    pub const COMPLETE_PUMP_AMM_MIGRATION_EVENT: &[u8] =
        &[228, 69, 165, 46, 81, 203, 154, 29, 189, 233, 93, 185, 92, 148, 234, 148];

    /// PumpFun 创建代币事件日志数据结构
    #[derive(Clone, Debug, Default, PartialEq, Eq, BorshDeserialize)]
    pub struct PumpFunCreateTokenEventLog {
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

    /// PumpFun 交易事件日志数据结构
    #[derive(Clone, Debug, Default, PartialEq, Eq, BorshDeserialize)]
    pub struct PumpFunTradeEventLog {
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

    /// PumpFun 迁移事件日志数据结构
    #[derive(Clone, Debug, Default, PartialEq, Eq, BorshDeserialize)]
    pub struct PumpFunMigrateEventLog {
        pub user: Pubkey,
        pub mint: Pubkey,
        pub mint_amount: u64,
        pub sol_amount: u64,
        pub pool_migration_fee: u64,
        pub bonding_curve: Pubkey,
        pub timestamp: i64,
        pub pool: Pubkey,
    }

    /// 从日志数据解析 PumpFun 创建代币事件
    pub fn parse_create_token_event_log(data: &[u8]) -> Option<PumpFunCreateTokenEventLog> {
        if data.len() < PUMPFUN_CREATE_TOKEN_EVENT_LOG_SIZE {
            return None;
        }
        borsh::from_slice::<PumpFunCreateTokenEventLog>(&data[..PUMPFUN_CREATE_TOKEN_EVENT_LOG_SIZE]).ok()
    }

    /// 从日志数据解析 PumpFun 交易事件
    pub fn parse_trade_event_log(data: &[u8]) -> Option<PumpFunTradeEventLog> {
        if data.len() < PUMPFUN_TRADE_EVENT_LOG_SIZE {
            return None;
        }
        borsh::from_slice::<PumpFunTradeEventLog>(&data[..PUMPFUN_TRADE_EVENT_LOG_SIZE]).ok()
    }

    /// 从日志数据解析 PumpFun 迁移事件
    pub fn parse_migrate_event_log(data: &[u8]) -> Option<PumpFunMigrateEventLog> {
        if data.len() < PUMPFUN_MIGRATE_EVENT_LOG_SIZE {
            return None;
        }
        borsh::from_slice::<PumpFunMigrateEventLog>(&data[..PUMPFUN_MIGRATE_EVENT_LOG_SIZE]).ok()
    }

    /// 检查数据是否匹配创建代币事件判别符
    pub fn is_create_token_event(data: &[u8]) -> bool {
        data.len() >= 16 && &data[..16] == CREATE_TOKEN_EVENT
    }

    /// 检查数据是否匹配交易事件判别符
    pub fn is_trade_event(data: &[u8]) -> bool {
        data.len() >= 16 && &data[..16] == TRADE_EVENT
    }

    /// 检查数据是否匹配迁移事件判别符
    pub fn is_migrate_event(data: &[u8]) -> bool {
        data.len() >= 16 && &data[..16] == COMPLETE_PUMP_AMM_MIGRATION_EVENT
    }
}

/// 通用日志解析器
pub struct LogParser;

impl LogParser {
    /// 从 base64 编码的日志数据解析 PumpFun 事件
    pub fn parse_pumpfun_events_from_logs(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
    ) -> Vec<DexEvent> {
        let mut events = Vec::new();

        for log in logs {
            // 查找包含 base64 数据的日志行
            if let Some(data_start) = log.find("Program data: ") {
                let data_str = &log[data_start + 14..];

                // 尝试解码 base64 数据
                use base64::{Engine as _, engine::general_purpose};
                if let Ok(data) = general_purpose::STANDARD.decode(data_str.trim()) {
                    // 尝试解析各种事件类型
                    if pumpfun_logs::is_create_token_event(&data) {
                        if let Some(log_event) = pumpfun_logs::parse_create_token_event_log(&data[16..]) {
                            let event = PumpFunCreateTokenEvent {
                                metadata: EventMetadata {
                                    signature,
                                    slot,
                                    block_time: block_time.as_ref().map(|t| t.seconds),
                                    block_time_ms: block_time.as_ref().map(|t| t.seconds * 1000 + (t.nanos as i64) / 1_000_000),
                                    program_id: crate::parser::pumpfun::PUMPFUN_PROGRAM_ID.parse().unwrap(),
                                    outer_index: 0,
                                    inner_index: None,
                                    transaction_index: None,
                                    recv_us: 0,
                                    handle_us: 0,
                                },
                                name: log_event.name,
                                symbol: log_event.symbol,
                                uri: log_event.uri,
                                mint: log_event.mint,
                                bonding_curve: log_event.bonding_curve,
                                user: log_event.user,
                                creator: log_event.creator,
                                virtual_token_reserves: log_event.virtual_token_reserves,
                                virtual_sol_reserves: log_event.virtual_sol_reserves,
                                real_token_reserves: log_event.real_token_reserves,
                                token_total_supply: log_event.token_total_supply,
                                timestamp: log_event.timestamp,
                                mint_authority: Pubkey::default(),
                                associated_bonding_curve: Pubkey::default(),
                            };
                            events.push(DexEvent::PumpFunCreate(event));
                        }
                    } else if pumpfun_logs::is_trade_event(&data) {
                        if let Some(log_event) = pumpfun_logs::parse_trade_event_log(&data[16..]) {
                            let event = PumpFunTradeEvent {
                                metadata: EventMetadata {
                                    signature,
                                    slot,
                                    block_time: block_time.as_ref().map(|t| t.seconds),
                                    block_time_ms: block_time.as_ref().map(|t| t.seconds * 1000 + (t.nanos as i64) / 1_000_000),
                                    program_id: crate::parser::pumpfun::PUMPFUN_PROGRAM_ID.parse().unwrap(),
                                    outer_index: 0,
                                    inner_index: None,
                                    transaction_index: None,
                                    recv_us: 0,
                                    handle_us: 0,
                                },
                                mint: log_event.mint,
                                user: log_event.user,
                                sol_amount: log_event.sol_amount,
                                token_amount: log_event.token_amount,
                                is_buy: log_event.is_buy,
                                bonding_curve: Pubkey::default(),
                                virtual_sol_reserves: log_event.virtual_sol_reserves,
                                virtual_token_reserves: log_event.virtual_token_reserves,
                                real_sol_reserves: log_event.real_sol_reserves,
                                real_token_reserves: log_event.real_token_reserves,
                                fee_recipient: log_event.fee_recipient,
                                fee_basis_points: log_event.fee_basis_points,
                                fee: log_event.fee,
                                creator: log_event.creator,
                                creator_fee_basis_points: log_event.creator_fee_basis_points,
                                creator_fee: log_event.creator_fee,
                                total_unclaimed_tokens: log_event.total_unclaimed_tokens,
                                total_claimed_tokens: log_event.total_claimed_tokens,
                                current_sol_volume: log_event.current_sol_volume,
                                timestamp: log_event.timestamp,
                                last_update_timestamp: log_event.last_update_timestamp,
                                track_volume: log_event.track_volume,
                                max_sol_cost: 0,
                                min_sol_output: 0,
                                amount: 0,
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
                            };
                            events.push(DexEvent::PumpFunTrade(event));
                        }
                    }
                }
            }
        }

        events
    }

    /// 解析所有支持的 DEX 事件从日志
    pub fn parse_all_dex_events_from_logs(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
    ) -> Vec<DexEvent> {
        let mut events = Vec::new();

        // 解析 PumpFun 事件
        events.extend(Self::parse_pumpfun_events_from_logs(logs, signature, slot, block_time.clone()));

        // TODO: 添加其他 DEX 的日志解析
        // events.extend(Self::parse_bonk_events_from_logs(logs, signature, slot, block_time.clone()));
        // events.extend(Self::parse_pumpswap_events_from_logs(logs, signature, slot, block_time.clone()));
        // events.extend(Self::parse_raydium_events_from_logs(logs, signature, slot, block_time));

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_pumpfun_event_discriminators() {
        // 测试事件判别符检查
        let create_data = vec![228, 69, 165, 46, 81, 203, 154, 29, 27, 114, 169, 77, 222, 235, 99, 118];
        assert!(pumpfun_logs::is_create_token_event(&create_data));

        let trade_data = vec![228, 69, 165, 46, 81, 203, 154, 29, 189, 219, 127, 211, 78, 230, 97, 238];
        assert!(pumpfun_logs::is_trade_event(&trade_data));

        let migrate_data = vec![228, 69, 165, 46, 81, 203, 154, 29, 189, 233, 93, 185, 92, 148, 234, 148];
        assert!(pumpfun_logs::is_migrate_event(&migrate_data));
    }

    #[test]
    fn test_log_parser() {
        let logs = vec![
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
            "Program data: 5EWlrejeMeQ6Eq5tqbGBMqUqBPSR//FLZQqV0Q==".to_string(),
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
        ];

        let signature = Signature::default();
        let slot = 123;
        let block_time = None;

        let events = LogParser::parse_all_dex_events_from_logs(&logs, signature, slot, block_time);
        // 注意：这个测试可能不会返回事件，因为示例数据可能不是有效的事件数据
        // 在实际使用中，需要真实的交易日志数据
    }
}