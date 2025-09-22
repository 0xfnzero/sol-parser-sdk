//! 事件合并器 - 纯函数式设计
//!
//! 负责合并来自指令和日志的事件数据，优先使用日志数据

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

// ===========================================
// PumpFun 事件合并函数
// ===========================================

/// 合并 PumpFun 交易事件
pub fn merge_pumpfun_trade_events(
    instruction_event: PumpFunTradeEvent,
    log_event: PumpFunTradeEvent,
) -> PumpFunTradeEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.sol_amount == 0 && instruction_event.sol_amount > 0 {
        merged.sol_amount = instruction_event.sol_amount;
    }
    if merged.token_amount == 0 && instruction_event.token_amount > 0 {
        merged.token_amount = instruction_event.token_amount;
    }
    if merged.max_sol_cost == 0 && instruction_event.max_sol_cost > 0 {
        merged.max_sol_cost = instruction_event.max_sol_cost;
    }
    if merged.min_sol_output == 0 && instruction_event.min_sol_output > 0 {
        merged.min_sol_output = instruction_event.min_sol_output;
    }
    if merged.amount == 0 && instruction_event.amount > 0 {
        merged.amount = instruction_event.amount;
    }

    // 填充账户信息
    if merged.mint == Default::default() && instruction_event.mint != Default::default() {
        merged.mint = instruction_event.mint;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }
    if merged.bonding_curve == Default::default() && instruction_event.bonding_curve != Default::default() {
        merged.bonding_curve = instruction_event.bonding_curve;
    }

    merged
}

/// 合并 PumpFun 创建事件
pub fn merge_pumpfun_create_events(
    instruction_event: PumpFunCreateTokenEvent,
    log_event: PumpFunCreateTokenEvent,
) -> PumpFunCreateTokenEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.mint == Default::default() && instruction_event.mint != Default::default() {
        merged.mint = instruction_event.mint;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }
    if merged.creator == Default::default() && instruction_event.creator != Default::default() {
        merged.creator = instruction_event.creator;
    }
    if merged.bonding_curve == Default::default() && instruction_event.bonding_curve != Default::default() {
        merged.bonding_curve = instruction_event.bonding_curve;
    }

    // 如果日志中没有名称信息，保持指令的默认设置
    if merged.name.is_empty() && !instruction_event.name.is_empty() {
        merged.name = instruction_event.name;
    }
    if merged.symbol.is_empty() && !instruction_event.symbol.is_empty() {
        merged.symbol = instruction_event.symbol;
    }
    if merged.uri.is_empty() && !instruction_event.uri.is_empty() {
        merged.uri = instruction_event.uri;
    }

    merged
}

// ===========================================
// Bonk 事件合并函数
// ===========================================

/// 合并 Bonk 交易事件
pub fn merge_bonk_trade_events(
    instruction_event: BonkTradeEvent,
    log_event: BonkTradeEvent,
) -> BonkTradeEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.amount_in == 0 && instruction_event.amount_in > 0 {
        merged.amount_in = instruction_event.amount_in;
    }
    if merged.amount_out == 0 && instruction_event.amount_out > 0 {
        merged.amount_out = instruction_event.amount_out;
    }

    // 填充账户信息
    if merged.pool_state == Default::default() && instruction_event.pool_state != Default::default() {
        merged.pool_state = instruction_event.pool_state;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }

    merged
}

// ===========================================
// PumpSwap 事件合并函数
// ===========================================

/// 合并 PumpSwap Buy 事件
pub fn merge_pumpswap_buy_events(
    instruction_event: PumpSwapBuyEvent,
    log_event: PumpSwapBuyEvent,
) -> PumpSwapBuyEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.sol_amount == 0 && instruction_event.sol_amount > 0 {
        merged.sol_amount = instruction_event.sol_amount;
    }
    if merged.token_amount == 0 && instruction_event.token_amount > 0 {
        merged.token_amount = instruction_event.token_amount;
    }
    if merged.slippage == 0 && instruction_event.slippage > 0 {
        merged.slippage = instruction_event.slippage;
    }

    // 填充账户信息
    if merged.pool_id == Default::default() && instruction_event.pool_id != Default::default() {
        merged.pool_id = instruction_event.pool_id;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
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
    if merged.sol_amount == 0 && instruction_event.sol_amount > 0 {
        merged.sol_amount = instruction_event.sol_amount;
    }
    if merged.token_amount == 0 && instruction_event.token_amount > 0 {
        merged.token_amount = instruction_event.token_amount;
    }
    if merged.slippage == 0 && instruction_event.slippage > 0 {
        merged.slippage = instruction_event.slippage;
    }

    // 填充账户信息
    if merged.pool_id == Default::default() && instruction_event.pool_id != Default::default() {
        merged.pool_id = instruction_event.pool_id;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }

    merged
}

// ===========================================
// Raydium CLMM 事件合并函数
// ===========================================

/// 合并 Raydium CLMM Swap 事件
pub fn merge_raydium_clmm_swap_events(
    instruction_event: RaydiumClmmSwapEvent,
    log_event: RaydiumClmmSwapEvent,
) -> RaydiumClmmSwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.amount == 0 && instruction_event.amount > 0 {
        merged.amount = instruction_event.amount;
    }
    if merged.other_amount_threshold == 0 && instruction_event.other_amount_threshold > 0 {
        merged.other_amount_threshold = instruction_event.other_amount_threshold;
    }
    if merged.sqrt_price_limit_x64 == 0 && instruction_event.sqrt_price_limit_x64 > 0 {
        merged.sqrt_price_limit_x64 = instruction_event.sqrt_price_limit_x64;
    }

    // 填充账户信息
    if merged.pool == Default::default() && instruction_event.pool != Default::default() {
        merged.pool = instruction_event.pool;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }

    merged
}

// ===========================================
// Raydium CPMM 事件合并函数
// ===========================================

/// 合并 Raydium CPMM Swap 事件
pub fn merge_raydium_cpmm_swap_events(
    instruction_event: RaydiumCpmmSwapEvent,
    log_event: RaydiumCpmmSwapEvent,
) -> RaydiumCpmmSwapEvent {
    let mut merged = log_event; // 以日志数据为基础

    // 用指令数据填充缺失的字段
    if merged.amount_in == 0 && instruction_event.amount_in > 0 {
        merged.amount_in = instruction_event.amount_in;
    }
    if merged.amount_out == 0 && instruction_event.amount_out > 0 {
        merged.amount_out = instruction_event.amount_out;
    }

    // 填充账户信息
    if merged.pool == Default::default() && instruction_event.pool != Default::default() {
        merged.pool = instruction_event.pool;
    }
    if merged.user == Default::default() && instruction_event.user != Default::default() {
        merged.user = instruction_event.user;
    }

    merged
}

// ===========================================
// 通用辅助函数
// ===========================================

/// 检查事件是否包含有效数据
pub fn has_valid_data(event: &DexEvent) -> bool {
    match event {
        DexEvent::PumpFunTrade(e) => e.sol_amount > 0 || e.token_amount > 0,
        DexEvent::PumpFunCreate(e) => !e.name.is_empty() || !e.symbol.is_empty(),
        DexEvent::BonkTrade(e) => e.amount_in > 0 || e.amount_out > 0,
        DexEvent::PumpSwapBuy(e) => e.sol_amount > 0 || e.token_amount > 0,
        DexEvent::PumpSwapSell(e) => e.token_amount > 0 || e.sol_amount > 0,
        DexEvent::RaydiumClmmSwap(e) => e.amount > 0,
        DexEvent::RaydiumCpmmSwap(e) => e.amount_in > 0 || e.amount_out > 0,
        _ => true, // 对于其他事件类型，假设它们有效
    }
}

/// 比较两个事件的完整性
pub fn is_more_complete(event1: &DexEvent, event2: &DexEvent) -> bool {
    count_non_zero_fields(event1) > count_non_zero_fields(event2)
}

/// 计算事件中非零字段的数量
pub fn count_non_zero_fields(event: &DexEvent) -> usize {
    match event {
        DexEvent::PumpFunTrade(e) => {
            let mut count = 0;
            if e.sol_amount > 0 { count += 1; }
            if e.token_amount > 0 { count += 1; }
            if e.virtual_sol_reserves > 0 { count += 1; }
            if e.virtual_token_reserves > 0 { count += 1; }
            if e.fee > 0 { count += 1; }
            if e.timestamp > 0 { count += 1; }
            if e.mint != Pubkey::default() { count += 1; }
            if e.user != Pubkey::default() { count += 1; }
            count
        },
        DexEvent::PumpFunCreate(e) => {
            let mut count = 0;
            if !e.name.is_empty() { count += 1; }
            if !e.symbol.is_empty() { count += 1; }
            if !e.uri.is_empty() { count += 1; }
            if e.virtual_sol_reserves > 0 { count += 1; }
            if e.virtual_token_reserves > 0 { count += 1; }
            if e.timestamp > 0 { count += 1; }
            count
        },
        DexEvent::BonkTrade(e) => {
            let mut count = 0;
            if e.amount_in > 0 { count += 1; }
            if e.amount_out > 0 { count += 1; }
            count
        },
        DexEvent::RaydiumClmmSwap(e) => {
            let mut count = 0;
            if e.amount > 0 { count += 1; }
            if e.other_amount_threshold > 0 { count += 1; }
            if e.sqrt_price_limit_x64 > 0 { count += 1; }
            count
        },
        DexEvent::RaydiumCpmmSwap(e) => {
            let mut count = 0;
            if e.amount_in > 0 { count += 1; }
            if e.amount_out > 0 { count += 1; }
            count
        },
        _ => 1, // 默认值
    }
}