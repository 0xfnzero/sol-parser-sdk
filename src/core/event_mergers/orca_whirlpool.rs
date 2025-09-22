//! Orca Whirlpool 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub fn merge_orca_whirlpool_swap_events(
    instruction_event: OrcaWhirlpoolSwapEvent,
    log_event: OrcaWhirlpoolSwapEvent,
) -> OrcaWhirlpoolSwapEvent {
    let mut merged = log_event;
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    merged
}

pub fn merge_orca_whirlpool_increase_liquidity_events(
    instruction_event: OrcaWhirlpoolLiquidityIncreasedEvent,
    log_event: OrcaWhirlpoolLiquidityIncreasedEvent,
) -> OrcaWhirlpoolLiquidityIncreasedEvent {
    let mut merged = log_event;
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    merged
}

pub fn merge_orca_whirlpool_decrease_liquidity_events(
    instruction_event: OrcaWhirlpoolLiquidityDecreasedEvent,
    log_event: OrcaWhirlpoolLiquidityDecreasedEvent,
) -> OrcaWhirlpoolLiquidityDecreasedEvent {
    let mut merged = log_event;
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    merged
}

pub fn merge_orca_whirlpool_initialize_pool_events(
    instruction_event: OrcaWhirlpoolPoolInitializedEvent,
    log_event: OrcaWhirlpoolPoolInitializedEvent,
) -> OrcaWhirlpoolPoolInitializedEvent {
    let mut merged = log_event;
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    merged
}