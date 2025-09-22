//! Raydium CPMM 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Raydium CPMM Swap 事件
pub fn merge_raydium_cpmm_swap_events(
    instruction_event: RaydiumCpmmSwapEvent,
    log_event: RaydiumCpmmSwapEvent,
) -> RaydiumCpmmSwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.pool == Pubkey::default() && instruction_event.pool != Pubkey::default() {
        merged.pool = instruction_event.pool;
    }
    if merged.amount_in == 0 && instruction_event.amount_in > 0 {
        merged.amount_in = instruction_event.amount_in;
    }
    if merged.amount_out == 0 && instruction_event.amount_out > 0 {
        merged.amount_out = instruction_event.amount_out;
    }

    merged
}