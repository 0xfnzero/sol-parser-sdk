//! PumpFun 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

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

    // 填充账户信息
    if merged.mint == Pubkey::default() && instruction_event.mint != Pubkey::default() {
        merged.mint = instruction_event.mint;
    }
    if merged.user == Pubkey::default() && instruction_event.user != Pubkey::default() {
        merged.user = instruction_event.user;
    }
    if merged.bonding_curve == Pubkey::default() && instruction_event.bonding_curve != Pubkey::default() {
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
    if merged.mint == Pubkey::default() && instruction_event.mint != Pubkey::default() {
        merged.mint = instruction_event.mint;
    }
    if merged.user == Pubkey::default() && instruction_event.user != Pubkey::default() {
        merged.user = instruction_event.user;
    }
    if merged.creator == Pubkey::default() && instruction_event.creator != Pubkey::default() {
        merged.creator = instruction_event.creator;
    }
    if merged.bonding_curve == Pubkey::default() && instruction_event.bonding_curve != Pubkey::default() {
        merged.bonding_curve = instruction_event.bonding_curve;
    }

    // 填充字符串字段
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