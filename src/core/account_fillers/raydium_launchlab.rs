//! Raydium LaunchLab 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 填充 Raydium LaunchLab Trade 事件账户
///
/// Raydium LaunchLab trade instruction account mapping:
/// 0: payer
/// 4: pool_state
pub fn fill_trade_accounts(e: &mut RaydiumLaunchlabTradeEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(4);
    }
}

/// Raydium LaunchLab Pool Create 账户填充
///
/// Raydium LaunchLab initialize instruction account mapping:
/// 1: creator
/// 5: pool_state
pub fn fill_pool_create_accounts(e: &mut RaydiumLaunchlabPoolCreateEvent, get: &AccountGetter<'_>) {
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(5);
    }
    if e.creator == Pubkey::default() {
        e.creator = get(1);
    }
}
