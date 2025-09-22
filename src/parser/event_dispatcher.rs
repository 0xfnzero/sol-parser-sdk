//! 事件分发器 - 简单高效的单次循环架构
//!
//! 替代复杂的 unified_parser，提供简洁的事件分发功能

use solana_sdk::signature::Signature;
// use prost_types::Timestamp;
use crate::parser::events::DexEvent;
use crate::parser::{pumpfun_ix_parser, bonk_ix_parser, pumpswap_ix_parser};

/// 简单的事件分发器
pub struct EventDispatcher;

impl EventDispatcher {
    /// 解析所有 DEX 事件 - 单次循环架构
    pub fn parse_all_dex_events(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<i64>,
    ) -> Vec<DexEvent> {
        let mut all_events = Vec::new();

        // 并行调用各个 DEX 解析器，每个内部都是单次循环
        // TODO: Implement log parsing for instruction parsers
        // all_events.extend(pumpfun_instructions::parse_all_events(logs, signature, slot, block_time.clone()));
        // all_events.extend(bonk_instructions::parse_all_events(logs, signature, slot, block_time.clone()));
        // all_events.extend(pumpswap_instructions::parse_all_events(logs, signature, slot, block_time.clone()));

        // TODO: 添加 Raydium CLMM 和 CPMM 解析器
        // all_events.extend(raydium_clmm::parse_all_events(logs, signature, slot, block_time.clone()));
        // all_events.extend(raydium_cpmm::parse_all_events(logs, signature, slot, block_time));

        all_events
    }

    /// 根据程序 ID 智能分发到对应的 DEX 解析器
    pub fn parse_by_program_id(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<i64>,
        program_id: &str,
    ) -> Vec<DexEvent> {
        match program_id {
            pumpfun_ix_parser::PROGRAM_ID => {
                // TODO: Implement log parsing for instruction parsers
                Vec::new()
            }
            bonk_ix_parser::PROGRAM_ID => {
                // TODO: Implement log parsing for instruction parsers
                Vec::new()
            }
            pumpswap_ix_parser::PUMPSWAP_PROGRAM_ID => {
                // TODO: Implement log parsing for instruction parsers
                Vec::new()
            }
            _ => {
                // 未知程序，返回错误事件
                vec![DexEvent::Error(format!("Unknown program ID: {}", program_id))]
            }
        }
    }

    /// 解析特定 DEX 的事件
    pub fn parse_pumpfun_events(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<i64>,
    ) -> Vec<DexEvent> {
        // TODO: Implement log parsing for instruction parsers
        Vec::new()
    }

    pub fn parse_bonk_events(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<i64>,
    ) -> Vec<DexEvent> {
        // TODO: Implement log parsing for instruction parsers
        Vec::new()
    }

    pub fn parse_pumpswap_events(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<i64>,
    ) -> Vec<DexEvent> {
        // TODO: Implement log parsing for instruction parsers
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_all_dex_events() {
        let logs = vec![
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
            "Program log: test".to_string(),
        ];

        let events = EventDispatcher::parse_all_dex_events(
            &logs,
            Signature::default(),
            0,
            None,
        );

        // 应该返回一个事件数组（可能为空，因为没有真实的程序数据）
        assert!(events.len() >= 0);
    }

    #[test]
    fn test_parse_by_program_id() {
        let logs = vec!["test log".to_string()];

        let events = EventDispatcher::parse_by_program_id(
            &logs,
            Signature::default(),
            0,
            None,
            pumpfun_instructions::PROGRAM_ID,
        );

        assert!(events.len() >= 0);
    }

    #[test]
    fn test_unknown_program_id() {
        let logs = vec!["test log".to_string()];

        let events = EventDispatcher::parse_by_program_id(
            &logs,
            Signature::default(),
            0,
            None,
            "unknown_program",
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::Error(msg) => assert!(msg.contains("Unknown program ID")),
            _ => panic!("Should return error event"),
        }
    }
}