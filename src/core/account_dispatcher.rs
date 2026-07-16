//! 账户填充调度器
//!
//! 主调度器，负责路由所有 DEX 事件到对应的协议填充器。
//! 从指令账户数据填充事件中缺失的账户字段。
//!
//! 各协议的具体填充逻辑在 account_fillers/ 子模块中实现。

use crate::core::account_fillers::{self, AccountGetter};
use crate::core::events::*;
use crate::instr::utils::get_instruction_account_getter;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to find the instruction invoke (not CPI log) with the most accounts
fn find_instruction_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
) -> Option<&'a (i32, i32)> {
    invokes.iter().max_by_key(|(outer_idx, inner_idx)| {
        if *inner_idx >= 0 {
            meta.inner_instructions
                .iter()
                .find(|inner| inner.index == *outer_idx as u32)
                .and_then(|inner_group| inner_group.instructions.get(*inner_idx as usize))
                .map(|ix| ix.accounts.len())
                .unwrap_or(0)
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(*outer_idx as usize))
                .map(|ix| ix.accounts.len())
                .unwrap_or(0)
        }
    })
}

fn instruction_has_discriminator(
    transaction: &Option<Transaction>,
    outer_idx: i32,
    discriminator: [u8; 8],
) -> bool {
    if outer_idx < 0 {
        return false;
    }
    transaction
        .as_ref()
        .and_then(|tx| tx.message.as_ref())
        .and_then(|msg| msg.instructions.get(outer_idx as usize))
        .and_then(|ix| ix.data.get(..8))
        .is_some_and(|disc| disc == discriminator)
}

fn find_pumpfun_create_invoke<'a>(
    invokes: &'a [(i32, i32)],
    transaction: &Option<Transaction>,
    ix_name: &str,
) -> Option<&'a (i32, i32)> {
    let discriminator = if ix_name == "create_v2" {
        crate::instr::pump::discriminators::CREATE_V2
    } else {
        crate::instr::pump::discriminators::CREATE
    };
    invokes
        .iter()
        .find(|(outer_idx, inner_idx)| {
            *inner_idx < 0 && instruction_has_discriminator(transaction, *outer_idx, discriminator)
        })
        .or_else(|| invokes.iter().find(|(_, inner_idx)| *inner_idx < 0))
}

/// 通用填充辅助宏
macro_rules! fill_event_accounts {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get($program_id) {
            if let Some(invoke) = find_instruction_invoke(invokes, $meta, $tx) {
                let account_keys =
                    $tx.as_ref().and_then(|tx| tx.message.as_ref()).map(|msg| &msg.account_keys);
                if let Some(get_account) = get_instruction_account_getter(
                    $meta,
                    $tx,
                    account_keys,
                    &$meta.loaded_writable_addresses,
                    &$meta.loaded_readonly_addresses,
                    invoke,
                ) {
                    $filler(&get_account);
                }
            }
        }
    };
}

macro_rules! fill_event_accounts_with_invoke {
    ($event:expr, $meta:expr, $tx:expr, $invoke:expr, $filler:expr) => {{
        let account_keys =
            $tx.as_ref().and_then(|tx| tx.message.as_ref()).map(|msg| &msg.account_keys);
        if let Some(get_account) = get_instruction_account_getter(
            $meta,
            $tx,
            account_keys,
            &$meta.loaded_writable_addresses,
            &$meta.loaded_readonly_addresses,
            $invoke,
        ) {
            $filler(&get_account);
        }
    }};
}

// ============================================================================
// Public API
// ============================================================================

/// 从交易 meta 将缺失账户填入事件（`program_invokes`: program id → (outer, inner) 索引列表）
pub fn fill_accounts_with_owned_keys(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &HashMap<Pubkey, Vec<(i32, i32)>>,
) {
    use crate::grpc::program_ids::*;

    match event {
        // PumpFun
        DexEvent::PumpFunTrade(e)
        | DexEvent::PumpFunBuy(e)
        | DexEvent::PumpFunSell(e)
        | DexEvent::PumpFunBuyExactSolIn(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPFUN_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpfun::fill_trade_accounts(e, get);
                }
            );
        }
        DexEvent::PumpFunCreate(e) => {
            if let Some(invokes) = program_invokes.get(&PUMPFUN_PROGRAM) {
                if let Some(invoke) = find_pumpfun_create_invoke(invokes, transaction, &e.ix_name) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            if e.ix_name == "create_v2" {
                                account_fillers::pumpfun::fill_create_accounts_from_v2(e, get);
                            } else {
                                account_fillers::pumpfun::fill_create_accounts(e, get);
                            }
                        }
                    );
                }
            }
        }
        DexEvent::PumpFunCreateV2(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPFUN_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpfun::fill_create_v2_accounts(e, get);
                }
            );
        }
        DexEvent::PumpFunMigrate(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPFUN_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpfun::fill_migrate_accounts(e, get);
                }
            );
        }

        // PumpSwap
        DexEvent::PumpSwapBuy(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_buy_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapSell(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_sell_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapTrade(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_trade_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapCreatePool(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_create_pool_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapLiquidityAdded(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_liquidity_added_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapLiquidityRemoved(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_liquidity_removed_accounts(e, get);
                }
            );
        }

        // Raydium CLMM
        DexEvent::RaydiumClmmSwap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_swap_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumClmmCreatePool(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_create_pool_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumClmmOpenPosition(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_open_position_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumClmmClosePosition(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_close_position_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_increase_liquidity_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumClmmDecreaseLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_clmm_decrease_liquidity_accounts(e, get);
                }
            );
        }

        // Raydium CPMM
        DexEvent::RaydiumCpmmSwap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CPMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_cpmm_swap_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumCpmmDeposit(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CPMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_cpmm_deposit_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumCpmmWithdraw(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CPMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_cpmm_withdraw_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumCpmmInitialize(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_CPMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_cpmm_initialize_accounts(e, get);
                }
            );
        }

        // Raydium AMM V4
        DexEvent::RaydiumAmmV4Swap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_AMM_V4_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_amm_v4_swap_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumAmmV4Deposit(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_AMM_V4_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_amm_v4_deposit_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumAmmV4Withdraw(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_AMM_V4_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium::fill_amm_v4_withdraw_accounts(e, get);
                }
            );
        }

        // Orca Whirlpool
        DexEvent::OrcaWhirlpoolSwap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &ORCA_WHIRLPOOL_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::orca::fill_whirlpool_swap_accounts(e, get);
                }
            );
        }
        DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &ORCA_WHIRLPOOL_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::orca::fill_whirlpool_liquidity_increased_accounts(e, get);
                }
            );
        }
        DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &ORCA_WHIRLPOOL_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::orca::fill_whirlpool_liquidity_decreased_accounts(e, get);
                }
            );
        }

        // Meteora DAMM V2
        DexEvent::MeteoraDammV2Swap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_swap_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDammV2CreatePosition(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_create_position_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDammV2ClosePosition(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_close_position_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDammV2AddLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_add_liquidity_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_remove_liquidity_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDammV2InitializePool(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DAMM_V2_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_damm_v2_initialize_pool_accounts(e, get);
                }
            );
        }

        // Meteora Pools
        DexEvent::MeteoraPoolsSwap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_POOLS_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_pools_swap_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraPoolsAddLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_POOLS_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_pools_add_liquidity_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraPoolsRemoveLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_POOLS_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_pools_remove_liquidity_accounts(e, get);
                }
            );
        }

        // Meteora DLMM
        DexEvent::MeteoraDlmmSwap(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_dlmm_swap_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDlmmAddLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_dlmm_add_liquidity_accounts(e, get);
                }
            );
        }
        DexEvent::MeteoraDlmmRemoveLiquidity(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DLMM_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::meteora::fill_dlmm_remove_liquidity_accounts(e, get);
                }
            );
        }

        // RaydiumLaunchlab
        DexEvent::RaydiumLaunchlabTrade(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_LAUNCHLAB_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium_launchlab::fill_trade_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumLaunchlabPoolCreate(e) => {
            fill_event_accounts!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_LAUNCHLAB_PROGRAM,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium_launchlab::fill_pool_create_accounts(e, get);
                }
            );
        }

        _ => {}
    }
}
