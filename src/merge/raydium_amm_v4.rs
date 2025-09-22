//! Raydium AMM V4 事件合并函数

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 合并 Raydium AMM V4 Swap 事件
pub fn merge_raydium_amm_v4_swap_events(
    instruction_event: RaydiumAmmV4SwapEvent,
    log_event: RaydiumAmmV4SwapEvent,
) -> RaydiumAmmV4SwapEvent {
    let mut merged = log_event;
    if merged.amm == Pubkey::default() && instruction_event.amm != Pubkey::default() {
        merged.amm = instruction_event.amm;
    }
    merged
}

/// 合并 Raydium AMM V4 Deposit 事件
pub fn merge_raydium_amm_v4_deposit_events(
    instruction_event: RaydiumAmmV4DepositEvent,
    log_event: RaydiumAmmV4DepositEvent,
) -> RaydiumAmmV4DepositEvent {
    let mut merged = log_event;
    if merged.amm == Pubkey::default() && instruction_event.amm != Pubkey::default() {
        merged.amm = instruction_event.amm;
    }
    merged
}

/// 合并 Raydium AMM V4 Withdraw 事件
pub fn merge_raydium_amm_v4_withdraw_events(
    instruction_event: RaydiumAmmV4WithdrawEvent,
    log_event: RaydiumAmmV4WithdrawEvent,
) -> RaydiumAmmV4WithdrawEvent {
    let mut merged = log_event;
    if merged.amm == Pubkey::default() && instruction_event.amm != Pubkey::default() {
        merged.amm = instruction_event.amm;
    }
    merged
}

/// 合并 Raydium AMM V4 Initialize2 事件
pub fn merge_raydium_amm_v4_initialize2_events(
    instruction_event: RaydiumAmmV4Initialize2Event,
    log_event: RaydiumAmmV4Initialize2Event,
) -> RaydiumAmmV4Initialize2Event {
    let mut merged = log_event;
    if merged.amm == Pubkey::default() && instruction_event.amm != Pubkey::default() {
        merged.amm = instruction_event.amm;
    }
    merged
}

/// 合并 Raydium AMM V4 WithdrawPnl 事件
pub fn merge_raydium_amm_v4_withdraw_pnl_events(
    instruction_event: RaydiumAmmV4WithdrawPnlEvent,
    log_event: RaydiumAmmV4WithdrawPnlEvent,
) -> RaydiumAmmV4WithdrawPnlEvent {
    let mut merged = log_event;
    if merged.amm == Pubkey::default() && instruction_event.amm != Pubkey::default() {
        merged.amm = instruction_event.amm;
    }
    merged
}