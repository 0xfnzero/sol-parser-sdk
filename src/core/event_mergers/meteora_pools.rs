//! Meteora Pools 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub fn merge_meteora_pools_add_liquidity_events(
    instruction_event: MeteoraPoolsAddLiquidityEvent,
    log_event: MeteoraPoolsAddLiquidityEvent,
) -> MeteoraPoolsAddLiquidityEvent {
    let mut merged = log_event;
    if merged.lp_mint_amount == 0 && instruction_event.lp_mint_amount > 0 {
        merged.lp_mint_amount = instruction_event.lp_mint_amount;
    }
    merged
}

pub fn merge_meteora_pools_remove_liquidity_events(
    instruction_event: MeteoraPoolsRemoveLiquidityEvent,
    log_event: MeteoraPoolsRemoveLiquidityEvent,
) -> MeteoraPoolsRemoveLiquidityEvent {
    let mut merged = log_event;
    if merged.lp_unmint_amount == 0 && instruction_event.lp_unmint_amount > 0 {
        merged.lp_unmint_amount = instruction_event.lp_unmint_amount;
    }
    merged
}

pub fn merge_meteora_pools_swap_events(
    instruction_event: MeteoraPoolsSwapEvent,
    log_event: MeteoraPoolsSwapEvent,
) -> MeteoraPoolsSwapEvent {
    let mut merged = log_event;
    if merged.in_amount == 0 && instruction_event.in_amount > 0 {
        merged.in_amount = instruction_event.in_amount;
    }
    merged
}

pub fn merge_meteora_pools_bootstrap_liquidity_events(
    instruction_event: MeteoraPoolsBootstrapLiquidityEvent,
    log_event: MeteoraPoolsBootstrapLiquidityEvent,
) -> MeteoraPoolsBootstrapLiquidityEvent {
    let mut merged = log_event;
    if merged.pool == Pubkey::default() && instruction_event.pool != Pubkey::default() {
        merged.pool = instruction_event.pool;
    }
    merged
}

pub fn merge_meteora_pools_pool_created_events(
    instruction_event: MeteoraPoolsPoolCreatedEvent,
    log_event: MeteoraPoolsPoolCreatedEvent,
) -> MeteoraPoolsPoolCreatedEvent {
    let mut merged = log_event;
    if merged.pool == Pubkey::default() && instruction_event.pool != Pubkey::default() {
        merged.pool = instruction_event.pool;
    }
    merged
}

pub fn merge_meteora_pools_set_pool_fees_events(
    instruction_event: MeteoraPoolsSetPoolFeesEvent,
    log_event: MeteoraPoolsSetPoolFeesEvent,
) -> MeteoraPoolsSetPoolFeesEvent {
    let mut merged = log_event;
    if merged.pool == Pubkey::default() && instruction_event.pool != Pubkey::default() {
        merged.pool = instruction_event.pool;
    }
    merged
}