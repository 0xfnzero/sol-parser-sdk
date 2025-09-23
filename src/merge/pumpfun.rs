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
    if merged.real_sol_reserves == 0 && instruction_event.real_sol_reserves > 0 {
        merged.real_sol_reserves = instruction_event.real_sol_reserves;
    }
    if merged.real_token_reserves == 0 && instruction_event.real_token_reserves > 0 {
        merged.real_token_reserves = instruction_event.real_token_reserves;
    }
    if merged.timestamp == 0 && instruction_event.timestamp > 0 {
        merged.timestamp = instruction_event.timestamp;
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
    // creator 字段已从精简版事件中移除
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