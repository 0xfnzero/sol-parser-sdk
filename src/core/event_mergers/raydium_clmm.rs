//! Raydium CLMM 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Raydium CLMM Swap 事件
pub fn merge_raydium_clmm_swap_events(
    instruction_event: RaydiumClmmSwapEvent,
    log_event: RaydiumClmmSwapEvent,
) -> RaydiumClmmSwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.pool == Pubkey::default() && instruction_event.pool != Pubkey::default() {
        merged.pool = instruction_event.pool;
    }
    if merged.amount == 0 && instruction_event.amount > 0 {
        merged.amount = instruction_event.amount;
    }
    if merged.other_amount_threshold == 0 && instruction_event.other_amount_threshold > 0 {
        merged.other_amount_threshold = instruction_event.other_amount_threshold;
    }

    merged
}
