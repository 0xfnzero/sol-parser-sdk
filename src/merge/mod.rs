//! 事件合并器模块 - 指令和日志数据合并
//!
//! 实现指令事件和日志事件的智能合并，优先使用日志数据
//!
//! 模块化架构：
//! - 各 DEX 的合并函数分别在对应的子模块中
//! - 统一的合并逻辑和事件匹配在此主模块
//! - 通过 traits 定义通用合并行为

use crate::core::events::*;

// 导入各 DEX 的合并函数模块
mod pumpfun;
mod raydium_clmm;
mod raydium_cpmm;
mod raydium_amm_v4;
mod bonk;
mod pumpswap;
mod orca_whirlpool;
mod meteora_pools;
mod meteora_damm_v2;

// 重新导出合并函数
use pumpfun::*;
use raydium_clmm::*;
use raydium_cpmm::*;
use raydium_amm_v4::*;
use bonk::*;
use pumpswap::*;
use orca_whirlpool::*;
use meteora_pools::*;
use meteora_damm_v2::*;

/// 合并指令和日志事件列表，优先使用日志数据
pub fn merge_instruction_and_log_events(
    instruction_events: Vec<DexEvent>,
    log_events: Vec<DexEvent>,
) -> Vec<DexEvent> {
    // 快速路径：如果没有指令事件，直接返回日志事件
    if instruction_events.is_empty() {
        return log_events;
    }

    // 快速路径：如果没有日志事件，直接返回指令事件
    if log_events.is_empty() {
        return instruction_events;
    }

    let mut merged_events = Vec::with_capacity(log_events.len() + instruction_events.len());

    // 1. 先添加所有日志事件（优先）
    merged_events.extend(log_events);

    // 2. 对于指令事件，检查是否有对应的日志事件可以合并
    'instr_loop: for instr_event in instruction_events {
        // 从后向前搜索，因为最近的事件更可能匹配
        for i in (0..merged_events.len()).rev() {
            if events_can_merge(&merged_events[i], &instr_event) {
                // 找到匹配的日志事件，进行合并
                let log_event = merged_events[i].clone();
                if let Some(merged) = merge_single_event(instr_event, log_event) {
                    merged_events[i] = merged;
                }
                continue 'instr_loop;
            }
        }
        // 没有找到匹配的日志事件，直接添加指令事件
        merged_events.push(instr_event);
    }

    merged_events
}


/// 检查两个事件是否可以合并（同类型、同交易、同关键字段）
fn events_can_merge(event1: &DexEvent, event2: &DexEvent) -> bool {
    match (event1, event2) {
        (DexEvent::PumpFunTrade(e1), DexEvent::PumpFunTrade(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.mint == e2.mint &&
            e1.is_buy == e2.is_buy
        },
        (DexEvent::PumpFunCreate(e1), DexEvent::PumpFunCreate(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.mint == e2.mint
        },
        (DexEvent::RaydiumClmmSwap(e1), DexEvent::RaydiumClmmSwap(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool_state == e2.pool_state &&
            e1.zero_for_one == e2.zero_for_one
        },
        (DexEvent::RaydiumCpmmSwap(e1), DexEvent::RaydiumCpmmSwap(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool_state == e2.pool_state
        },
        (DexEvent::BonkTrade(e1), DexEvent::BonkTrade(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool_state == e2.pool_state
        },
        (DexEvent::PumpSwapBuy(e1), DexEvent::PumpSwapBuy(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool_id == e2.pool_id
        },
        (DexEvent::PumpSwapSell(e1), DexEvent::PumpSwapSell(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool_id == e2.pool_id
        },
        (DexEvent::RaydiumAmmV4Swap(e1), DexEvent::RaydiumAmmV4Swap(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.amm == e2.amm
        },
        (DexEvent::RaydiumAmmV4Deposit(e1), DexEvent::RaydiumAmmV4Deposit(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.amm == e2.amm
        },
        (DexEvent::RaydiumAmmV4Withdraw(e1), DexEvent::RaydiumAmmV4Withdraw(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.amm == e2.amm
        },
        (DexEvent::RaydiumAmmV4Initialize2(e1), DexEvent::RaydiumAmmV4Initialize2(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.amm == e2.amm
        },
        (DexEvent::RaydiumAmmV4WithdrawPnl(e1), DexEvent::RaydiumAmmV4WithdrawPnl(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.amm == e2.amm
        },
        // Orca Whirlpool 事件
        (DexEvent::OrcaWhirlpoolSwap(e1), DexEvent::OrcaWhirlpoolSwap(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.whirlpool == e2.whirlpool
        },
        (DexEvent::OrcaWhirlpoolLiquidityIncreased(e1), DexEvent::OrcaWhirlpoolLiquidityIncreased(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.whirlpool == e2.whirlpool &&
            e1.position == e2.position
        },
        (DexEvent::OrcaWhirlpoolLiquidityDecreased(e1), DexEvent::OrcaWhirlpoolLiquidityDecreased(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.whirlpool == e2.whirlpool &&
            e1.position == e2.position
        },
        (DexEvent::OrcaWhirlpoolPoolInitialized(e1), DexEvent::OrcaWhirlpoolPoolInitialized(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.whirlpool == e2.whirlpool
        },
        // Meteora Pools 事件
        (DexEvent::MeteoraPoolsSwap(e1), DexEvent::MeteoraPoolsSwap(e2)) => {
            e1.metadata.signature == e2.metadata.signature
        },
        (DexEvent::MeteoraPoolsAddLiquidity(e1), DexEvent::MeteoraPoolsAddLiquidity(e2)) => {
            e1.metadata.signature == e2.metadata.signature
        },
        (DexEvent::MeteoraPoolsRemoveLiquidity(e1), DexEvent::MeteoraPoolsRemoveLiquidity(e2)) => {
            e1.metadata.signature == e2.metadata.signature
        },
        (DexEvent::MeteoraPoolsBootstrapLiquidity(e1), DexEvent::MeteoraPoolsBootstrapLiquidity(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool == e2.pool
        },
        (DexEvent::MeteoraPoolsPoolCreated(e1), DexEvent::MeteoraPoolsPoolCreated(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool == e2.pool
        },
        (DexEvent::MeteoraPoolsSetPoolFees(e1), DexEvent::MeteoraPoolsSetPoolFees(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.pool == e2.pool
        },
        // Meteora DAMM V2 事件
        (DexEvent::MeteoraDammV2Swap(e1), DexEvent::MeteoraDammV2Swap(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair
        },
        (DexEvent::MeteoraDammV2AddLiquidity(e1), DexEvent::MeteoraDammV2AddLiquidity(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair &&
            e1.position == e2.position
        },
        (DexEvent::MeteoraDammV2RemoveLiquidity(e1), DexEvent::MeteoraDammV2RemoveLiquidity(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair &&
            e1.position == e2.position
        },
        (DexEvent::MeteoraDammV2InitializePool(e1), DexEvent::MeteoraDammV2InitializePool(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair
        },
        (DexEvent::MeteoraDammV2CreatePosition(e1), DexEvent::MeteoraDammV2CreatePosition(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.position == e2.position
        },
        (DexEvent::MeteoraDammV2ClosePosition(e1), DexEvent::MeteoraDammV2ClosePosition(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.position == e2.position
        },
        (DexEvent::MeteoraDammV2ClaimPositionFee(e1), DexEvent::MeteoraDammV2ClaimPositionFee(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.position == e2.position
        },
        (DexEvent::MeteoraDammV2InitializeReward(e1), DexEvent::MeteoraDammV2InitializeReward(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair &&
            e1.reward_index == e2.reward_index
        },
        (DexEvent::MeteoraDammV2FundReward(e1), DexEvent::MeteoraDammV2FundReward(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.lb_pair == e2.lb_pair &&
            e1.reward_index == e2.reward_index
        },
        (DexEvent::MeteoraDammV2ClaimReward(e1), DexEvent::MeteoraDammV2ClaimReward(e2)) => {
            e1.metadata.signature == e2.metadata.signature &&
            e1.position == e2.position &&
            e1.reward_index == e2.reward_index
        },
        _ => false,
    }
}

/// 合并单个事件，优先使用日志数据
fn merge_single_event(instr_event: DexEvent, log_event: DexEvent) -> Option<DexEvent> {
    match (instr_event, log_event) {
        (DexEvent::PumpFunTrade(instr), DexEvent::PumpFunTrade(log)) => {
            Some(DexEvent::PumpFunTrade(merge_pumpfun_trade_events(instr, log)))
        },
        (DexEvent::PumpFunCreate(instr), DexEvent::PumpFunCreate(log)) => {
            Some(DexEvent::PumpFunCreate(merge_pumpfun_create_events(instr, log)))
        },
        (DexEvent::RaydiumClmmSwap(instr), DexEvent::RaydiumClmmSwap(log)) => {
            Some(DexEvent::RaydiumClmmSwap(merge_raydium_clmm_swap_events(instr, log)))
        },
        (DexEvent::RaydiumCpmmSwap(instr), DexEvent::RaydiumCpmmSwap(log)) => {
            Some(DexEvent::RaydiumCpmmSwap(merge_raydium_cpmm_swap_events(instr, log)))
        },
        (DexEvent::BonkTrade(instr), DexEvent::BonkTrade(log)) => {
            Some(DexEvent::BonkTrade(merge_bonk_trade_events(instr, log)))
        },
        (DexEvent::PumpSwapBuy(instr), DexEvent::PumpSwapBuy(log)) => {
            Some(DexEvent::PumpSwapBuy(merge_pumpswap_buy_events(instr, log)))
        },
        (DexEvent::PumpSwapSell(instr), DexEvent::PumpSwapSell(log)) => {
            Some(DexEvent::PumpSwapSell(merge_pumpswap_sell_events(instr, log)))
        },
        (DexEvent::RaydiumAmmV4Swap(instr), DexEvent::RaydiumAmmV4Swap(log)) => {
            Some(DexEvent::RaydiumAmmV4Swap(merge_raydium_amm_v4_swap_events(instr, log)))
        },
        (DexEvent::RaydiumAmmV4Deposit(instr), DexEvent::RaydiumAmmV4Deposit(log)) => {
            Some(DexEvent::RaydiumAmmV4Deposit(merge_raydium_amm_v4_deposit_events(instr, log)))
        },
        (DexEvent::RaydiumAmmV4Withdraw(instr), DexEvent::RaydiumAmmV4Withdraw(log)) => {
            Some(DexEvent::RaydiumAmmV4Withdraw(merge_raydium_amm_v4_withdraw_events(instr, log)))
        },
        (DexEvent::RaydiumAmmV4Initialize2(instr), DexEvent::RaydiumAmmV4Initialize2(log)) => {
            Some(DexEvent::RaydiumAmmV4Initialize2(merge_raydium_amm_v4_initialize2_events(instr, log)))
        },
        (DexEvent::RaydiumAmmV4WithdrawPnl(instr), DexEvent::RaydiumAmmV4WithdrawPnl(log)) => {
            Some(DexEvent::RaydiumAmmV4WithdrawPnl(merge_raydium_amm_v4_withdraw_pnl_events(instr, log)))
        },
        // Orca Whirlpool 事件
        (DexEvent::OrcaWhirlpoolSwap(instr), DexEvent::OrcaWhirlpoolSwap(log)) => {
            Some(DexEvent::OrcaWhirlpoolSwap(merge_orca_whirlpool_swap_events(instr, log)))
        },
        (DexEvent::OrcaWhirlpoolLiquidityIncreased(instr), DexEvent::OrcaWhirlpoolLiquidityIncreased(log)) => {
            Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(merge_orca_whirlpool_increase_liquidity_events(instr, log)))
        },
        (DexEvent::OrcaWhirlpoolLiquidityDecreased(instr), DexEvent::OrcaWhirlpoolLiquidityDecreased(log)) => {
            Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(merge_orca_whirlpool_decrease_liquidity_events(instr, log)))
        },
        (DexEvent::OrcaWhirlpoolPoolInitialized(instr), DexEvent::OrcaWhirlpoolPoolInitialized(log)) => {
            Some(DexEvent::OrcaWhirlpoolPoolInitialized(merge_orca_whirlpool_initialize_pool_events(instr, log)))
        },
        // Meteora Pools 事件
        (DexEvent::MeteoraPoolsSwap(instr), DexEvent::MeteoraPoolsSwap(log)) => {
            Some(DexEvent::MeteoraPoolsSwap(merge_meteora_pools_swap_events(instr, log)))
        },
        (DexEvent::MeteoraPoolsAddLiquidity(instr), DexEvent::MeteoraPoolsAddLiquidity(log)) => {
            Some(DexEvent::MeteoraPoolsAddLiquidity(merge_meteora_pools_add_liquidity_events(instr, log)))
        },
        (DexEvent::MeteoraPoolsRemoveLiquidity(instr), DexEvent::MeteoraPoolsRemoveLiquidity(log)) => {
            Some(DexEvent::MeteoraPoolsRemoveLiquidity(merge_meteora_pools_remove_liquidity_events(instr, log)))
        },
        (DexEvent::MeteoraPoolsBootstrapLiquidity(instr), DexEvent::MeteoraPoolsBootstrapLiquidity(log)) => {
            Some(DexEvent::MeteoraPoolsBootstrapLiquidity(merge_meteora_pools_bootstrap_liquidity_events(instr, log)))
        },
        (DexEvent::MeteoraPoolsPoolCreated(instr), DexEvent::MeteoraPoolsPoolCreated(log)) => {
            Some(DexEvent::MeteoraPoolsPoolCreated(merge_meteora_pools_pool_created_events(instr, log)))
        },
        (DexEvent::MeteoraPoolsSetPoolFees(instr), DexEvent::MeteoraPoolsSetPoolFees(log)) => {
            Some(DexEvent::MeteoraPoolsSetPoolFees(merge_meteora_pools_set_pool_fees_events(instr, log)))
        },
        // Meteora DAMM V2 事件
        (DexEvent::MeteoraDammV2Swap(instr), DexEvent::MeteoraDammV2Swap(log)) => {
            Some(DexEvent::MeteoraDammV2Swap(merge_meteora_damm_v2_swap_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2AddLiquidity(instr), DexEvent::MeteoraDammV2AddLiquidity(log)) => {
            Some(DexEvent::MeteoraDammV2AddLiquidity(merge_meteora_damm_v2_add_liquidity_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2RemoveLiquidity(instr), DexEvent::MeteoraDammV2RemoveLiquidity(log)) => {
            Some(DexEvent::MeteoraDammV2RemoveLiquidity(merge_meteora_damm_v2_remove_liquidity_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2InitializePool(instr), DexEvent::MeteoraDammV2InitializePool(log)) => {
            Some(DexEvent::MeteoraDammV2InitializePool(merge_meteora_damm_v2_initialize_pool_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2CreatePosition(instr), DexEvent::MeteoraDammV2CreatePosition(log)) => {
            Some(DexEvent::MeteoraDammV2CreatePosition(merge_meteora_damm_v2_create_position_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2ClosePosition(instr), DexEvent::MeteoraDammV2ClosePosition(log)) => {
            Some(DexEvent::MeteoraDammV2ClosePosition(merge_meteora_damm_v2_close_position_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2ClaimPositionFee(instr), DexEvent::MeteoraDammV2ClaimPositionFee(log)) => {
            Some(DexEvent::MeteoraDammV2ClaimPositionFee(merge_meteora_damm_v2_claim_position_fee_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2InitializeReward(instr), DexEvent::MeteoraDammV2InitializeReward(log)) => {
            Some(DexEvent::MeteoraDammV2InitializeReward(merge_meteora_damm_v2_initialize_reward_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2FundReward(instr), DexEvent::MeteoraDammV2FundReward(log)) => {
            Some(DexEvent::MeteoraDammV2FundReward(merge_meteora_damm_v2_fund_reward_events(instr, log)))
        },
        (DexEvent::MeteoraDammV2ClaimReward(instr), DexEvent::MeteoraDammV2ClaimReward(log)) => {
            Some(DexEvent::MeteoraDammV2ClaimReward(merge_meteora_damm_v2_claim_reward_events(instr, log)))
        },
        _ => None,
    }
}