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

/// Like [`find_instruction_invoke`], but prefers the invoke whose FIRST
/// account equals `anchor` (for pool-scoped events: pAMM buy/sell account
/// layouts all put the pool at index 0).
///
/// Rationale: `max_by_key(accounts.len())` was only ever meant to skip the
/// 1-account event-CPI shells. It silently picks the WRONG instruction when
/// one transaction carries TWO real invokes of the same program — e.g. a
/// Jupiter token-to-token route (sell mint A → buy mint B) contains a pAMM
/// sell (24 accounts) and a pAMM buy (26 accounts, cashback variant): the
/// buy wins on length, and every event in the tx — including the SELL on
/// pool A — gets its `base_mint` backfilled from the BUY leg's accounts.
/// Anchoring on the event's own pool makes the match exact; when no invoke
/// matches (defensive: unknown future layout where the pool is not at
/// index 0) we fall back to the historical length heuristic.
fn find_instruction_invoke_anchored<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    account_keys: Option<&Vec<Vec<u8>>>,
    anchor_account_index: usize,
    anchor: &Pubkey,
) -> Option<&'a (i32, i32)> {
    if *anchor != Pubkey::default() {
        let anchored = invokes.iter().find(|invoke| {
            get_instruction_account_getter(
                meta,
                transaction,
                account_keys,
                &meta.loaded_writable_addresses,
                &meta.loaded_readonly_addresses,
                invoke,
            )
            .is_some_and(|get_account| get_account(anchor_account_index) == *anchor)
        });
        if anchored.is_some() {
            return anchored;
        }
    }
    find_instruction_invoke(invokes, meta, transaction)
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

/// Pool-anchored variant of [`fill_event_accounts`]: resolves the invoke
/// whose first account equals `$anchor` before backfilling, so multi-invoke
/// transactions (token-to-token routes) enrich each event from its own leg.
macro_rules! fill_event_accounts_anchored {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $anchor:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get($program_id) {
            let account_keys =
                $tx.as_ref().and_then(|tx| tx.message.as_ref()).map(|msg| &msg.account_keys);
            if let Some(invoke) =
                find_instruction_invoke_anchored(invokes, $meta, $tx, account_keys, 0, $anchor)
            {
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

/// Pool-anchored account filling for protocols whose pool is not account zero.
macro_rules! fill_event_accounts_anchored_at {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $anchor_index:expr, $anchor:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get($program_id) {
            let account_keys =
                $tx.as_ref().and_then(|tx| tx.message.as_ref()).map(|msg| &msg.account_keys);
            if let Some(invoke) = find_instruction_invoke_anchored(
                invokes,
                $meta,
                $tx,
                account_keys,
                $anchor_index,
                $anchor,
            ) {
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
            let pool = e.pool;
            fill_event_accounts_anchored!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                &pool,
                |get: &AccountGetter<'_>| {
                    account_fillers::pumpswap::fill_buy_accounts(e, get);
                }
            );
        }
        DexEvent::PumpSwapSell(e) => {
            let pool = e.pool;
            fill_event_accounts_anchored!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                &pool,
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
            let pool = e.pool;
            fill_event_accounts_anchored!(
                e,
                meta,
                transaction,
                program_invokes,
                &METEORA_DLMM_PROGRAM,
                &pool,
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
            let pool = e.pool_state;
            fill_event_accounts_anchored_at!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_LAUNCHLAB_PROGRAM,
                4,
                &pool,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium_launchlab::fill_trade_accounts(e, get);
                }
            );
        }
        DexEvent::RaydiumLaunchlabPoolCreate(e) => {
            let pool = e.pool_state;
            fill_event_accounts_anchored_at!(
                e,
                meta,
                transaction,
                program_invokes,
                &RAYDIUM_LAUNCHLAB_PROGRAM,
                5,
                &pool,
                |get: &AccountGetter<'_>| {
                    account_fillers::raydium_launchlab::fill_pool_create_accounts(e, get);
                }
            );
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{
        MeteoraDlmmSwapEvent, PumpSwapBuyEvent, PumpSwapSellEvent, RaydiumLaunchlabTradeEvent,
    };
    use crate::grpc::program_ids::{
        METEORA_DLMM_PROGRAM, PUMPSWAP_PROGRAM, RAYDIUM_LAUNCHLAB_PROGRAM,
    };
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, Message, MessageHeader, Transaction, TransactionStatusMeta,
    };

    struct RouteFixture {
        meta: TransactionStatusMeta,
        transaction: Option<Transaction>,
        invokes: HashMap<Pubkey, Vec<(i32, i32)>>,
        sell_pool: Pubkey,
        buy_pool: Pubkey,
        sell_mint: Pubkey,
        buy_mint: Pubkey,
    }

    /// A Jupiter-style token-to-token route: one outer pAMM sell invoke
    /// (24 accounts, pool/base_mint of leg A) followed by one outer pAMM buy
    /// invoke (26 accounts — the cashback variant is longer — pool/base_mint
    /// of leg B). Mirrors live tx DRCWs7iv… where the sell event's base_mint
    /// was backfilled from the buy leg.
    fn token_to_token_fixture() -> RouteFixture {
        let sell_pool = Pubkey::new_unique();
        let buy_pool = Pubkey::new_unique();
        let sell_mint = Pubkey::new_unique();
        let buy_mint = Pubkey::new_unique();
        let padding = Pubkey::new_unique();

        // static keys: [0]=sell_pool [1]=buy_pool [2]=sell_mint [3]=buy_mint
        // [4]=pumpswap program [5]=padding
        let static_keys: Vec<Vec<u8>> =
            [sell_pool, buy_pool, sell_mint, buy_mint, PUMPSWAP_PROGRAM, padding]
                .iter()
                .map(|k| k.to_bytes().to_vec())
                .collect();

        let mut sell_accounts = vec![5u8; 24];
        sell_accounts[0] = 0; // pool
        sell_accounts[3] = 2; // base_mint
        let mut buy_accounts = vec![5u8; 26];
        buy_accounts[0] = 1; // pool
        buy_accounts[3] = 3; // base_mint

        let transaction = Some(Transaction {
            signatures: vec![vec![0u8; 64]],
            message: Some(Message {
                header: Some(MessageHeader::default()),
                account_keys: static_keys,
                recent_blockhash: vec![0u8; 32],
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: sell_accounts,
                        data: vec![0],
                    },
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: buy_accounts,
                        data: vec![0],
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
            }),
        });
        let meta = TransactionStatusMeta::default();
        let mut invokes = HashMap::new();
        invokes.insert(PUMPSWAP_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)]);

        RouteFixture { meta, transaction, invokes, sell_pool, buy_pool, sell_mint, buy_mint }
    }

    #[test]
    fn token_to_token_route_backfills_each_leg_from_its_own_invoke() {
        let f = token_to_token_fixture();

        let mut sell =
            DexEvent::PumpSwapSell(PumpSwapSellEvent { pool: f.sell_pool, ..Default::default() });
        fill_accounts_with_owned_keys(&mut sell, &f.meta, &f.transaction, &f.invokes);
        match sell {
            DexEvent::PumpSwapSell(e) => assert_eq!(
                e.base_mint, f.sell_mint,
                "sell event must backfill from the SELL leg, not the longer buy invoke"
            ),
            _ => unreachable!(),
        }

        let mut buy =
            DexEvent::PumpSwapBuy(PumpSwapBuyEvent { pool: f.buy_pool, ..Default::default() });
        fill_accounts_with_owned_keys(&mut buy, &f.meta, &f.transaction, &f.invokes);
        match buy {
            DexEvent::PumpSwapBuy(e) => assert_eq!(e.base_mint, f.buy_mint),
            _ => unreachable!(),
        }
    }

    #[test]
    fn anchored_lookup_is_order_independent() {
        let mut f = token_to_token_fixture();
        // Reverse invoke order: the buy leg now comes first.
        f.invokes.get_mut(&PUMPSWAP_PROGRAM).unwrap().reverse();

        let mut sell =
            DexEvent::PumpSwapSell(PumpSwapSellEvent { pool: f.sell_pool, ..Default::default() });
        fill_accounts_with_owned_keys(&mut sell, &f.meta, &f.transaction, &f.invokes);
        match sell {
            DexEvent::PumpSwapSell(e) => assert_eq!(e.base_mint, f.sell_mint),
            _ => unreachable!(),
        }
    }

    #[test]
    fn unmatched_pool_falls_back_to_longest_invoke() {
        let f = token_to_token_fixture();
        // Pool that matches no invoke's first account (e.g. a future layout
        // where the pool moved): keep the historical max-accounts behavior.
        let mut sell = DexEvent::PumpSwapSell(PumpSwapSellEvent {
            pool: Pubkey::new_unique(),
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut sell, &f.meta, &f.transaction, &f.invokes);
        match sell {
            DexEvent::PumpSwapSell(e) => assert_eq!(
                e.base_mint, f.buy_mint,
                "fallback must preserve the pre-fix heuristic (longest invoke wins)"
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn dlmm_route_backfills_each_leg_from_matching_pool_invoke() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let first_x_mint = Pubkey::new_unique();
        let first_y_mint = Pubkey::new_unique();
        let second_x_mint = Pubkey::new_unique();
        let second_y_mint = Pubkey::new_unique();
        let padding = Pubkey::new_unique();
        let static_pubkeys = [
            first_pool,
            second_pool,
            first_x_mint,
            first_y_mint,
            second_x_mint,
            second_y_mint,
            METEORA_DLMM_PROGRAM,
            padding,
        ];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();

        let dlmm_accounts = |len, pool_index, x_mint_index, y_mint_index| {
            let mut accounts = vec![7u8; len];
            accounts[0] = pool_index;
            accounts[6] = x_mint_index;
            accounts[7] = y_mint_index;
            accounts
        };
        let transaction = Some(Transaction {
            signatures: vec![vec![0u8; 64]],
            message: Some(Message {
                header: Some(MessageHeader::default()),
                account_keys,
                recent_blockhash: vec![0u8; 32],
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 6,
                        accounts: dlmm_accounts(20, 0, 2, 3),
                        data: vec![0],
                    },
                    CompiledInstruction {
                        program_id_index: 6,
                        accounts: dlmm_accounts(15, 1, 4, 5),
                        data: vec![0],
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(METEORA_DLMM_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);
        let swap_event = |pool| {
            DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
                metadata: EventMetadata::default(),
                token_x_mint: Pubkey::default(),
                token_y_mint: Pubkey::default(),
                pool,
                from: Pubkey::default(),
                start_bin_id: 0,
                end_bin_id: 0,
                amount_in: 1,
                amount_out: 1,
                swap_for_y: false,
                fee: 0,
                protocol_fee: 0,
                fee_bps: 0,
                host_fee: 0,
            })
        };

        let mut first_event = swap_event(first_pool);
        fill_accounts_with_owned_keys(&mut first_event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraDlmmSwap(first_event) = first_event else {
            unreachable!();
        };
        assert_eq!(first_event.token_x_mint, first_x_mint);
        assert_eq!(first_event.token_y_mint, first_y_mint);

        let mut second_event = swap_event(second_pool);
        fill_accounts_with_owned_keys(&mut second_event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraDlmmSwap(second_event) = second_event else {
            unreachable!();
        };
        assert_eq!(second_event.token_x_mint, second_x_mint);
        assert_eq!(second_event.token_y_mint, second_y_mint);
    }

    #[test]
    fn launchlab_trade_backfills_from_matching_pool_invoke() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let first_quote_mint = Pubkey::new_unique();
        let second_quote_mint = Pubkey::new_unique();
        let padding = Pubkey::new_unique();
        let static_pubkeys = [
            first_pool,
            second_pool,
            first_quote_mint,
            second_quote_mint,
            RAYDIUM_LAUNCHLAB_PROGRAM,
            padding,
        ];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();
        let launchlab_accounts = |pool_index, quote_mint_index| {
            let mut accounts = vec![5u8; 15];
            accounts[4] = pool_index;
            accounts[10] = quote_mint_index;
            accounts[14] = 4;
            accounts
        };
        let transaction = Some(Transaction {
            signatures: vec![vec![0u8; 64]],
            message: Some(Message {
                header: Some(MessageHeader::default()),
                account_keys,
                recent_blockhash: vec![0u8; 32],
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: launchlab_accounts(0, 2),
                        data: vec![0],
                    },
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: launchlab_accounts(1, 3),
                        data: vec![0],
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes =
            HashMap::from([(RAYDIUM_LAUNCHLAB_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);
        let mut event = DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
            metadata: EventMetadata::default(),
            pool_state: first_pool,
            user: Pubkey::default(),
            amount_in: 1,
            amount_out: 2,
            is_buy: true,
            trade_direction: TradeDirection::Buy,
            exact_in: true,
            global_config: Pubkey::default(),
            platform_config: Pubkey::default(),
            user_base_token: Pubkey::default(),
            user_quote_token: Pubkey::default(),
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            base_mint: Pubkey::default(),
            quote_mint: Pubkey::default(),
            base_token_program: Pubkey::default(),
            quote_token_program: Pubkey::default(),
        });

        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);

        let DexEvent::RaydiumLaunchlabTrade(event) = event else {
            unreachable!();
        };
        assert_eq!(event.quote_mint, first_quote_mint);
        assert_ne!(event.quote_mint, second_quote_mint);
    }
}
