//! Orca Whirlpool 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Orca Whirlpool Swap 事件
pub fn merge_orca_whirlpool_swap_events(
    instruction_event: OrcaWhirlpoolSwapEvent,
    log_event: OrcaWhirlpoolSwapEvent,
) -> OrcaWhirlpoolSwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的账户信息
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    if merged.token_vault_a == Pubkey::default() && instruction_event.token_vault_a != Pubkey::default() {
        merged.token_vault_a = instruction_event.token_vault_a;
    }
    if merged.token_vault_b == Pubkey::default() && instruction_event.token_vault_b != Pubkey::default() {
        merged.token_vault_b = instruction_event.token_vault_b;
    }

    merged
}

/// 合并 Orca Whirlpool 增加流动性事件
pub fn merge_orca_whirlpool_increase_liquidity_events(
    instruction_event: OrcaWhirlpoolLiquidityIncreasedEvent,
    log_event: OrcaWhirlpoolLiquidityIncreasedEvent,
) -> OrcaWhirlpoolLiquidityIncreasedEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    if merged.position == Pubkey::default() && instruction_event.position != Pubkey::default() {
        merged.position = instruction_event.position;
    }

    merged
}

/// 合并 Orca Whirlpool 减少流动性事件
pub fn merge_orca_whirlpool_decrease_liquidity_events(
    instruction_event: OrcaWhirlpoolLiquidityDecreasedEvent,
    log_event: OrcaWhirlpoolLiquidityDecreasedEvent,
) -> OrcaWhirlpoolLiquidityDecreasedEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    if merged.position == Pubkey::default() && instruction_event.position != Pubkey::default() {
        merged.position = instruction_event.position;
    }

    merged
}

/// 合并 Orca Whirlpool 池初始化事件
pub fn merge_orca_whirlpool_initialize_pool_events(
    instruction_event: OrcaWhirlpoolPoolInitializedEvent,
    log_event: OrcaWhirlpoolPoolInitializedEvent,
) -> OrcaWhirlpoolPoolInitializedEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.whirlpool == Pubkey::default() && instruction_event.whirlpool != Pubkey::default() {
        merged.whirlpool = instruction_event.whirlpool;
    }
    if merged.whirlpools_config == Pubkey::default() && instruction_event.whirlpools_config != Pubkey::default() {
        merged.whirlpools_config = instruction_event.whirlpools_config;
    }
    if merged.token_mint_a == Pubkey::default() && instruction_event.token_mint_a != Pubkey::default() {
        merged.token_mint_a = instruction_event.token_mint_a;
    }
    if merged.token_mint_b == Pubkey::default() && instruction_event.token_mint_b != Pubkey::default() {
        merged.token_mint_b = instruction_event.token_mint_b;
    }

    merged
}