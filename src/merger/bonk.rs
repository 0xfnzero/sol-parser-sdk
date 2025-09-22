//! Bonk 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Bonk Trade 事件
pub fn merge_bonk_trade_events(
    instruction_event: BonkTradeEvent,
    log_event: BonkTradeEvent,
) -> BonkTradeEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.pool_state == Pubkey::default() && instruction_event.pool_state != Pubkey::default() {
        merged.pool_state = instruction_event.pool_state;
    }
    if merged.user == Pubkey::default() && instruction_event.user != Pubkey::default() {
        merged.user = instruction_event.user;
    }

    merged
}