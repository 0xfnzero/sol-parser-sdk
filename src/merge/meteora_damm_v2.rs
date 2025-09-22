//! Meteora DAMM V2 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Meteora DAMM V2 Swap 事件
pub fn merge_meteora_damm_v2_swap_events(
    instruction_event: MeteoraDammV2SwapEvent,
    log_event: MeteoraDammV2SwapEvent,
) -> MeteoraDammV2SwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    if merged.from == Pubkey::default() && instruction_event.from != Pubkey::default() {
        merged.from = instruction_event.from;
    }

    merged
}

/// 合并 Meteora DAMM V2 增加流动性事件
pub fn merge_meteora_damm_v2_add_liquidity_events(
    instruction_event: MeteoraDammV2AddLiquidityEvent,
    log_event: MeteoraDammV2AddLiquidityEvent,
) -> MeteoraDammV2AddLiquidityEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    if merged.from == Pubkey::default() && instruction_event.from != Pubkey::default() {
        merged.from = instruction_event.from;
    }
    if merged.position == Pubkey::default() && instruction_event.position != Pubkey::default() {
        merged.position = instruction_event.position;
    }

    merged
}

/// 合并 Meteora DAMM V2 Remove Liquidity 事件
pub fn merge_meteora_damm_v2_remove_liquidity_events(
    instruction_event: MeteoraDammV2RemoveLiquidityEvent,
    log_event: MeteoraDammV2RemoveLiquidityEvent,
) -> MeteoraDammV2RemoveLiquidityEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Initialize Pool 事件
pub fn merge_meteora_damm_v2_initialize_pool_events(
    instruction_event: MeteoraDammV2InitializePoolEvent,
    log_event: MeteoraDammV2InitializePoolEvent,
) -> MeteoraDammV2InitializePoolEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Create Position 事件
pub fn merge_meteora_damm_v2_create_position_events(
    instruction_event: MeteoraDammV2CreatePositionEvent,
    log_event: MeteoraDammV2CreatePositionEvent,
) -> MeteoraDammV2CreatePositionEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Close Position 事件
pub fn merge_meteora_damm_v2_close_position_events(
    instruction_event: MeteoraDammV2ClosePositionEvent,
    log_event: MeteoraDammV2ClosePositionEvent,
) -> MeteoraDammV2ClosePositionEvent {
    let mut merged = log_event;
    if merged.position == Pubkey::default() && instruction_event.position != Pubkey::default() {
        merged.position = instruction_event.position;
    }
    merged
}

/// 合并 Meteora DAMM V2 Claim Position Fee 事件
pub fn merge_meteora_damm_v2_claim_position_fee_events(
    instruction_event: MeteoraDammV2ClaimPositionFeeEvent,
    log_event: MeteoraDammV2ClaimPositionFeeEvent,
) -> MeteoraDammV2ClaimPositionFeeEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Initialize Reward 事件
pub fn merge_meteora_damm_v2_initialize_reward_events(
    instruction_event: MeteoraDammV2InitializeRewardEvent,
    log_event: MeteoraDammV2InitializeRewardEvent,
) -> MeteoraDammV2InitializeRewardEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Fund Reward 事件
pub fn merge_meteora_damm_v2_fund_reward_events(
    instruction_event: MeteoraDammV2FundRewardEvent,
    log_event: MeteoraDammV2FundRewardEvent,
) -> MeteoraDammV2FundRewardEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}

/// 合并 Meteora DAMM V2 Claim Reward 事件
pub fn merge_meteora_damm_v2_claim_reward_events(
    instruction_event: MeteoraDammV2ClaimRewardEvent,
    log_event: MeteoraDammV2ClaimRewardEvent,
) -> MeteoraDammV2ClaimRewardEvent {
    let mut merged = log_event;
    if merged.lb_pair == Pubkey::default() && instruction_event.lb_pair != Pubkey::default() {
        merged.lb_pair = instruction_event.lb_pair;
    }
    merged
}