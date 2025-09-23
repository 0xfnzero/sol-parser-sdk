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
    if merged.pool_state == Pubkey::default() && instruction_event.pool_state != Pubkey::default() {
        merged.pool_state = instruction_event.pool_state;
    }
    if merged.amount_in == 0 && instruction_event.amount_in > 0 {
        merged.amount_in = instruction_event.amount_in;
    }
    if merged.output_amount == 0 && instruction_event.output_amount > 0 {
        merged.output_amount = instruction_event.output_amount;
    }

    merged
}