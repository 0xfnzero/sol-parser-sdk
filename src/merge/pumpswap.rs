//! PumpSwap 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 PumpSwap Buy 事件
pub fn merge_pumpswap_buy_events(
    instruction_event: PumpSwapBuyEvent,
    log_event: PumpSwapBuyEvent,
) -> PumpSwapBuyEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.pool_id == Pubkey::default() && instruction_event.pool_id != Pubkey::default() {
        merged.pool_id = instruction_event.pool_id;
    }
    if merged.user == Pubkey::default() && instruction_event.user != Pubkey::default() {
        merged.user = instruction_event.user;
    }

    merged
}

/// 合并 PumpSwap Sell 事件
pub fn merge_pumpswap_sell_events(
    instruction_event: PumpSwapSellEvent,
    log_event: PumpSwapSellEvent,
) -> PumpSwapSellEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.pool_id == Pubkey::default() && instruction_event.pool_id != Pubkey::default() {
        merged.pool_id = instruction_event.pool_id;
    }
    if merged.user == Pubkey::default() && instruction_event.user != Pubkey::default() {
        merged.user = instruction_event.user;
    }

    merged
}