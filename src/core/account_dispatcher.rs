//! 账户填充调度器
//!
//! 主调度器，负责路由所有 DEX 事件到对应的协议填充器。
//! 从指令账户数据填充事件中缺失的账户字段。
//!
//! 各协议的具体填充逻辑在 account_fillers/ 子模块中实现。

use crate::core::account_fillers::{self, AccountGetter};
use crate::core::events::*;
use crate::core::invoke_context::{InvokeContext, InvokeLookup};
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
/// Strict pool-slot match — returns `None` when no invoke carries `anchor`
/// at `anchor_account_index` (does **not** fall back to length heuristic).
fn find_instruction_invoke_matching_anchor<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    account_keys: Option<&Vec<Vec<u8>>>,
    anchor_account_index: usize,
    anchor: &Pubkey,
) -> Option<&'a (i32, i32)> {
    if *anchor == Pubkey::default() {
        return None;
    }
    invokes.iter().find(|invoke| {
        get_instruction_account_getter(
            meta,
            transaction,
            account_keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )
        .is_some_and(|get_account| get_account(anchor_account_index) == *anchor)
    })
}

fn find_instruction_invoke_anchored<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    account_keys: Option<&Vec<Vec<u8>>>,
    anchor_account_index: usize,
    anchor: &Pubkey,
) -> Option<&'a (i32, i32)> {
    if *anchor != Pubkey::default() {
        return only_match(invokes.iter().filter(|invoke| {
            get_instruction_account_getter(
                meta,
                transaction,
                account_keys,
                &meta.loaded_writable_addresses,
                &meta.loaded_readonly_addresses,
                invoke,
            )
            .is_some_and(|get_account| get_account(anchor_account_index) == *anchor)
        }));
    }

    find_instruction_invoke(invokes, meta, transaction)
}

// CPMM logs do not carry all account addresses. Match both instruction kind and
// pool, and leave unresolved when multiple invocations could own the event.
fn find_cpmm_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    pool: Pubkey,
    pool_index: usize,
    discriminator: [u8; 8],
) -> Option<&'a (i32, i32)> {
    if pool == Pubkey::default() {
        return None;
    }
    let keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter(|invoke| {
        let data = if invoke.1 >= 0 {
            meta.inner_instructions
                .iter()
                .find(|g| g.index == invoke.0 as u32)
                .and_then(|g| g.instructions.get(invoke.1 as usize))
                .map(|ix| ix.data.as_slice())
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(invoke.0 as usize))
                .map(|ix| ix.data.as_slice())
        };
        if data.and_then(|data| data.get(..8)) != Some(discriminator.as_slice()) {
            return false;
        }
        get_instruction_account_getter(
            meta,
            transaction,
            keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )
        .is_some_and(|get| get(pool_index) == pool)
    }))
}

fn only_match<T>(mut matches: impl Iterator<Item = T>) -> Option<T> {
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
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

fn find_damm_v2_swap_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    pool: Pubkey,
) -> Option<(&'a (i32, i32), usize)> {
    if pool == Pubkey::default() {
        return None;
    }
    let account_keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    let mut matches = invokes.iter().filter_map(|invoke| {
        let (data, accounts) = if invoke.1 >= 0 {
            let ix = meta
                .inner_instructions
                .iter()
                .find(|group| group.index == invoke.0 as u32)?
                .instructions
                .get(invoke.1 as usize)?;
            (ix.data.as_slice(), ix.accounts.as_slice())
        } else {
            let ix = transaction.as_ref()?.message.as_ref()?.instructions.get(invoke.0 as usize)?;
            (ix.data.as_slice(), ix.accounts.as_slice())
        };
        use crate::instr::meteora_damm::discriminators::{SWAP, SWAP2};
        if !matches!(data.get(..8), Some(disc) if disc == SWAP || disc == SWAP2)
            || !(13..=14).contains(&accounts.len())
        {
            return None;
        }
        let get = get_instruction_account_getter(
            meta,
            transaction,
            account_keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )?;
        (get(1) == pool
            && get(accounts.len() - 1) == crate::grpc::program_ids::METEORA_DAMM_V2_PROGRAM)
            .then_some((invoke, accounts.len()))
    });
    let matched = matches.next()?;
    // The event carries a pool, but no per-invoke position: repeated swaps in
    // the same pool cannot safely be assigned to individual events.
    matches.next().is_none().then_some(matched)
}

// Distinguish repeated swaps in the same CLMM pool by their instruction
// account context. A v2 swap with no tick arrays must not inherit v1 arrays.
fn find_clmm_swap_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    event: &RaydiumClmmSwapEvent,
) -> Option<&'a (i32, i32)> {
    let account_keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter(|invoke| {
        use crate::instr::raydium_clmm::discriminators::{SWAP, SWAP_V2};
        let Some(data) =
            crate::core::common_filler::get_instruction_data(meta, transaction, invoke)
        else {
            return false;
        };
        let name = match data.get(..8) {
            Some(disc) if disc == SWAP => "swap",
            Some(disc) if disc == SWAP_V2 => "swap_v2",
            _ => return false,
        };
        if !event.ix_name.is_empty() && event.ix_name != name {
            return false;
        }
        get_instruction_account_getter(
            meta,
            transaction,
            account_keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )
        .is_some_and(|get| {
            let matches = |key: Pubkey, index| key == Pubkey::default() || get(index) == key;
            event.pool_state != Pubkey::default()
                && get(2) == event.pool_state
                && matches(event.sender, 0)
                // Log events use canonical token0/token1 order, which can be
                // reversed relative to instruction input/output accounts.
                && ((matches(event.token_account_0, 3) && matches(event.token_account_1, 4))
                    || (matches(event.token_account_0, 4) && matches(event.token_account_1, 3)))
                && matches(event.input_mint, 11)
                && matches(event.output_mint, 12)
        })
    }))
}

// Match trade layout and mint before filling missing account fields. Creation
// instructions and other pools must never supply a trade's account context.
fn find_pumpfun_trade_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    event: &PumpFunTradeEvent,
) -> Option<&'a (i32, i32)> {
    let account_keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter(|invoke| {
        let data = if invoke.1 >= 0 {
            meta.inner_instructions
                .iter()
                .find(|g| g.index == invoke.0 as u32)
                .and_then(|g| g.instructions.get(invoke.1 as usize))
                .map(|ix| ix.data.as_slice())
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(invoke.0 as usize))
                .map(|ix| ix.data.as_slice())
        };
        use crate::instr::pump::discriminators::*;
        let (mint_idx, user_idx, is_buy) = match data.and_then(|data| data.get(..8)) {
            Some(disc) if disc == BUY || disc == BUY_EXACT_SOL_IN => (2, 6, true),
            Some(disc) if disc == SELL => (2, 6, false),
            Some(disc) if disc == BUY_V2 || disc == BUY_EXACT_QUOTE_IN_V2 => (1, 13, true),
            Some(disc) if disc == SELL_V2 => (1, 13, false),
            _ => return false,
        };
        is_buy == event.is_buy
            && get_instruction_account_getter(
                meta,
                transaction,
                account_keys,
                &meta.loaded_writable_addresses,
                &meta.loaded_readonly_addresses,
                invoke,
            )
            .is_some_and(|get| {
                event.mint != Pubkey::default()
                    && get(mint_idx) == event.mint
                    && (event.user == Pubkey::default() || get(user_idx) == event.user)
            })
    }))
}

fn find_pumpfun_create_invoke<'a>(
    invokes: &'a [(i32, i32)],
    transaction: &Option<Transaction>,
    ix_name: &str,
    meta: &TransactionStatusMeta,
    mint: Pubkey,
) -> Option<&'a (i32, i32)> {
    let discriminator = if ix_name == "create_v2" {
        crate::instr::pump::discriminators::CREATE_V2
    } else {
        crate::instr::pump::discriminators::CREATE
    };
    let account_keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter(|(outer_idx, inner_idx)| {
        *inner_idx < 0
            && instruction_has_discriminator(transaction, *outer_idx, discriminator)
            && (mint == Pubkey::default()
                || get_instruction_account_getter(
                    meta,
                    transaction,
                    account_keys,
                    &meta.loaded_writable_addresses,
                    &meta.loaded_readonly_addresses,
                    &(*outer_idx, *inner_idx),
                )
                .is_some_and(|get| get(0) == mint))
    }))
}

// Select the actual liquidity instruction by discriminator and known pool.
// Account count alone can select an unrelated swap/deposit in the same transaction.
fn find_pools_liquidity_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    pool: Pubkey,
    bootstrap: bool,
    ix_name: &str,
) -> Option<(&'a (i32, i32), &'static str)> {
    let keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter_map(|invoke| {
        let data = if invoke.1 >= 0 {
            meta.inner_instructions
                .iter()
                .find(|g| g.index == invoke.0 as u32)
                .and_then(|g| g.instructions.get(invoke.1 as usize))
                .map(|ix| ix.data.as_slice())
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(invoke.0 as usize))
                .map(|ix| ix.data.as_slice())
        }?;
        use crate::instr::meteora_amm::discriminators::*;
        let name = match data.get(..8)? {
            d if bootstrap && d == BOOTSTRAP_LIQUIDITY => "bootstrap_liquidity",
            d if !bootstrap && d == REMOVE_LIQUIDITY => "remove_balance_liquidity",
            d if !bootstrap && d == REMOVE_LIQUIDITY_SINGLE_SIDE => "remove_liquidity_single_side",
            _ => return None,
        };
        if !ix_name.is_empty() && ix_name != name {
            return None;
        }
        let get = get_instruction_account_getter(
            meta,
            transaction,
            keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )?;
        if pool != Pubkey::default() && get(0) != pool {
            return None;
        }
        Some((invoke, name))
    }))
}

fn find_pools_management_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    pool: Pubkey,
    creation: bool,
    ix_name: &str,
) -> Option<(&'a (i32, i32), &'static str)> {
    let keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    only_match(invokes.iter().filter_map(|invoke| {
        let data = if invoke.1 >= 0 {
            meta.inner_instructions
                .iter()
                .find(|g| g.index == invoke.0 as u32)
                .and_then(|g| g.instructions.get(invoke.1 as usize))
                .map(|ix| ix.data.as_slice())
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(invoke.0 as usize))
                .map(|ix| ix.data.as_slice())
        }?;
        use crate::instr::meteora_amm::discriminators::*;
        let name = match data.get(..8)? {
            d if creation && d == CREATE_POOL => {
                "initialize_permissionless_constant_product_pool_with_config"
            }
            d if creation && d == CREATE_POOL_WITH_CONFIG2 => {
                "initialize_permissionless_constant_product_pool_with_config2"
            }
            d if creation && d == INITIALIZE_PERMISSIONED_POOL => "initialize_permissioned_pool",
            d if creation && d == INITIALIZE_PERMISSIONLESS_POOL => {
                "initialize_permissionless_pool"
            }
            d if creation && d == INITIALIZE_PERMISSIONLESS_POOL_WITH_FEE_TIER => {
                "initialize_permissionless_pool_with_fee_tier"
            }
            d if creation && d == INITIALIZE_CUSTOMIZABLE_POOL => {
                "initialize_customizable_permissionless_constant_product_pool"
            }
            d if !creation && d == SET_POOL_FEES => "set_pool_fees",
            _ => return None,
        };
        if !ix_name.is_empty() && ix_name != name {
            return None;
        }
        let get = get_instruction_account_getter(
            meta,
            transaction,
            keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )?;
        if pool != Pubkey::default() && get(0) != pool {
            return None;
        }
        Some((invoke, name))
    }))
}

/// 通用填充辅助宏
macro_rules! fill_event_accounts {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get_invokes($program_id) {
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

// PumpSwap events must be enriched from their own direction and user. Pool
// alone is ambiguous when one transaction trades repeatedly in the same pool.
pub(crate) fn find_pumpswap_trade_invoke<'a>(
    invokes: &'a [(i32, i32)],
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    pool: Pubkey,
    user: Pubkey,
    buy: bool,
) -> Option<&'a (i32, i32)> {
    if pool == Pubkey::default() {
        return None;
    }
    let keys = transaction.as_ref()?.message.as_ref().map(|msg| &msg.account_keys);
    let mut matches = invokes.iter().filter(|invoke| {
        let data = if invoke.1 >= 0 {
            meta.inner_instructions
                .iter()
                .find(|group| group.index == invoke.0 as u32)
                .and_then(|group| group.instructions.get(invoke.1 as usize))
                .map(|ix| ix.data.as_slice())
        } else {
            transaction
                .as_ref()
                .and_then(|tx| tx.message.as_ref())
                .and_then(|msg| msg.instructions.get(invoke.0 as usize))
                .map(|ix| ix.data.as_slice())
        };
        use crate::instr::pump_amm::discriminators::{BUY, BUY_EXACT_QUOTE_IN, SELL};
        let direction_matches = match data.and_then(|data| data.get(..8)) {
            Some(disc) if buy => disc == BUY || disc == BUY_EXACT_QUOTE_IN,
            Some(disc) => disc == SELL,
            None => false,
        };
        if !direction_matches
            || instruction_account_count(meta, transaction, invoke) < if buy { 23 } else { 21 }
        {
            return false;
        }
        get_instruction_account_getter(
            meta,
            transaction,
            keys,
            &meta.loaded_writable_addresses,
            &meta.loaded_readonly_addresses,
            invoke,
        )
        .is_some_and(|get| get(0) == pool && (user == Pubkey::default() || get(1) == user))
    });
    let matched = matches.next()?;
    matches.next().is_none().then_some(matched)
}

fn fill_dlmm_swap_event(
    event: &mut MeteoraDlmmSwapEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    invokes: &[(i32, i32)],
) {
    if event.pool == Pubkey::default() { return; }
    let keys = transaction.as_ref().and_then(|tx| tx.message.as_ref()).map(|m| &m.account_keys);
    let matched = only_match(invokes.iter().filter_map(|invoke| {
        let (data, count) = if invoke.1 >= 0 {
            let ix = meta.inner_instructions.iter().find(|g| g.index == invoke.0 as u32)?
                .instructions.get(invoke.1 as usize)?;
            (ix.data.as_slice(), ix.accounts.len())
        } else {
            let ix = transaction.as_ref()?.message.as_ref()?.instructions.get(invoke.0 as usize)?;
            (ix.data.as_slice(), ix.accounts.len())
        };
        let start = crate::instr::meteora_dlmm::validate_swap_layout(data, count)?;
        let get = get_instruction_account_getter(meta, transaction, keys,
            &meta.loaded_writable_addresses, &meta.loaded_readonly_addresses, invoke)?;
        let matches = |key: Pubkey, index| key == Pubkey::default() || get(index) == key;
        if get(0) != event.pool || !matches(event.from, 10)
            || !matches(event.user_token_in, 4) || !matches(event.user_token_out, 5) { return None; }
        use crate::instr::meteora_dlmm::discriminators::*;
        let v2 = data.get(..8).is_some_and(|d| d == SWAP2 || d == SWAP_EXACT_OUT2 || d == SWAP_WITH_PRICE_IMPACT2);
        Some((invoke, start, count, v2))
    }));
    if let Some((invoke, start, count, v2)) = matched {
        if let Some(get) = get_instruction_account_getter(meta, transaction, keys,
            &meta.loaded_writable_addresses, &meta.loaded_readonly_addresses, invoke) {
            account_fillers::meteora::fill_dlmm_swap_accounts_with_layout(event, &get, start, count, v2);
        }
    }
}

fn instruction_account_count(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    invoke: &(i32, i32),
) -> usize {
    if invoke.1 >= 0 {
        meta.inner_instructions
            .iter()
            .find(|group| group.index == invoke.0 as u32)
            .and_then(|group| group.instructions.get(invoke.1 as usize))
            .map_or(0, |ix| ix.accounts.len())
    } else {
        transaction
            .as_ref()
            .and_then(|tx| tx.message.as_ref())
            .and_then(|msg| msg.instructions.get(invoke.0 as usize))
            .map_or(0, |ix| ix.accounts.len())
    }
}

macro_rules! fill_pumpswap_accounts_anchored {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $anchor:expr, $user:expr, $buy:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get_invokes($program_id) {
            let account_keys =
                $tx.as_ref().and_then(|tx| tx.message.as_ref()).map(|msg| &msg.account_keys);
            if let Some(invoke) =
                find_pumpswap_trade_invoke(invokes, $meta, $tx, *$anchor, $user, $buy)
            {
                if let Some(get_account) = get_instruction_account_getter(
                    $meta,
                    $tx,
                    account_keys,
                    &$meta.loaded_writable_addresses,
                    &$meta.loaded_readonly_addresses,
                    invoke,
                ) {
                    $filler(&get_account, instruction_account_count($meta, $tx, invoke));
                }
            }
        }
    };
}

/// Pool-anchored account filling for protocols whose pool is not account zero.
macro_rules! fill_event_accounts_anchored_at {
    ($event:expr, $meta:expr, $tx:expr, $invokes:expr, $program_id:expr, $anchor_index:expr, $anchor:expr, $filler:expr) => {
        if let Some(invokes) = $invokes.get_invokes($program_id) {
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
fn fill_accounts_with_lookup<L: InvokeLookup + ?Sized>(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &L,
) {
    use crate::grpc::program_ids::*;

    match event {
        // PumpFun
        DexEvent::PumpFunTrade(e)
        | DexEvent::PumpFunBuy(e)
        | DexEvent::PumpFunSell(e)
        | DexEvent::PumpFunBuyExactSolIn(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&PUMPFUN_PROGRAM) {
                if let Some(invoke) = find_pumpfun_trade_invoke(invokes, meta, transaction, e) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::pumpfun::fill_trade_accounts(e, get);
                        }
                    );
                }
            }
        }
        DexEvent::PumpFunCreate(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&PUMPFUN_PROGRAM) {
                if let Some(invoke) =
                    find_pumpfun_create_invoke(invokes, transaction, &e.ix_name, meta, e.mint)
                {
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
            fill_pumpswap_accounts_anchored!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                &pool,
                e.user,
                true,
                |get: &AccountGetter<'_>, count: usize| {
                    account_fillers::pumpswap::fill_buy_accounts_with_count(e, get, count);
                }
            );
        }
        DexEvent::PumpSwapSell(e) => {
            let pool = e.pool;
            fill_pumpswap_accounts_anchored!(
                e,
                meta,
                transaction,
                program_invokes,
                &PUMPSWAP_PROGRAM,
                &pool,
                e.user,
                false,
                |get: &AccountGetter<'_>, count: usize| {
                    account_fillers::pumpswap::fill_sell_accounts_with_count(e, get, count);
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

        // Raydium CLMM — pool_state is account index 2 on swap / swap_v2.
        // Must anchor: multi-hop routes often carry 2+ CLMM swaps with equal
        // account counts; length heuristic alone cross-fills amm_config.
        DexEvent::RaydiumClmmSwap(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&RAYDIUM_CLMM_PROGRAM) {
                if let Some(invoke) = find_clmm_swap_invoke(invokes, meta, transaction, e) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::raydium::fill_clmm_swap_accounts_with_count(
                                e,
                                get,
                                instruction_account_count(meta, transaction, invoke),
                            );
                        }
                    );
                }
            }
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
        // Raydium CPMM — poolState is account index 3.
        DexEvent::RaydiumCpmmSwap(e) => {
            // Collection/initialize also place a pool at index 3. Pool alone is
            // insufficient: the swap log carries its exact-in/exact-out kind.
            let discriminator = if e.base_input {
                crate::instr::raydium_cpmm::discriminators::SWAP_BASE_IN
            } else {
                crate::instr::raydium_cpmm::discriminators::SWAP_BASE_OUT
            };
            if let Some(invokes) = program_invokes.get_invokes(&RAYDIUM_CPMM_PROGRAM) {
                if let Some(invoke) =
                    find_cpmm_invoke(invokes, meta, transaction, e.pool_id, 3, discriminator)
                {
                    if instruction_account_count(meta, transaction, invoke) >= 13 {
                        fill_event_accounts_with_invoke!(
                            e,
                            meta,
                            transaction,
                            invoke,
                            |get: &AccountGetter<'_>| {
                                account_fillers::raydium::fill_cpmm_swap_accounts(e, get);
                            }
                        );
                    }
                }
            }
        }
        DexEvent::RaydiumCpmmDeposit(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&RAYDIUM_CPMM_PROGRAM) {
                if let Some(invoke) = find_cpmm_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    2,
                    crate::instr::raydium_cpmm::discriminators::DEPOSIT,
                ) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::raydium::fill_cpmm_deposit_accounts(e, get);
                        }
                    );
                }
            }
        }
        DexEvent::RaydiumCpmmWithdraw(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&RAYDIUM_CPMM_PROGRAM) {
                if let Some(invoke) = find_cpmm_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    2,
                    crate::instr::raydium_cpmm::discriminators::WITHDRAW,
                ) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::raydium::fill_cpmm_withdraw_accounts(e, get);
                        }
                    );
                }
            }
        }
        DexEvent::RaydiumCpmmInitialize(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&RAYDIUM_CPMM_PROGRAM) {
                if let Some(invoke) = find_cpmm_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    3,
                    crate::instr::raydium_cpmm::discriminators::INITIALIZE,
                ) {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::raydium::fill_cpmm_initialize_accounts(e, get);
                        }
                    );
                }
            }
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

        // Orca Whirlpool — whirlpool at index 4 (swap_v2) or 2 (swap v1).
        // Must try strict matches first: `find_instruction_invoke_anchored`
        // falls back to length heuristic, so chaining it with `or_else` would
        // never reach the v1 slot when v2 misses.
        DexEvent::OrcaWhirlpoolSwap(e) => {
            let pool = e.whirlpool;
            if let Some(invokes) = program_invokes.get_invokes(&ORCA_WHIRLPOOL_PROGRAM) {
                let account_keys = transaction
                    .as_ref()
                    .and_then(|tx| tx.message.as_ref())
                    .map(|msg| &msg.account_keys);
                let invoke = find_instruction_invoke_matching_anchor(
                    invokes,
                    meta,
                    transaction,
                    account_keys,
                    4,
                    &pool,
                )
                .or_else(|| {
                    find_instruction_invoke_matching_anchor(
                        invokes,
                        meta,
                        transaction,
                        account_keys,
                        2,
                        &pool,
                    )
                })
                .or_else(|| find_instruction_invoke(invokes, meta, transaction));
                if let Some(invoke) = invoke {
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::orca::fill_whirlpool_swap_accounts(e, get);
                        }
                    );
                }
            }
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
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_DAMM_V2_PROGRAM) {
                if let Some((invoke, account_count)) =
                    find_damm_v2_swap_invoke(invokes, meta, transaction, e.pool)
                {
                    let keys = transaction
                        .as_ref()
                        .and_then(|tx| tx.message.as_ref())
                        .map(|msg| &msg.account_keys);
                    if let Some(get) = get_instruction_account_getter(
                        meta,
                        transaction,
                        keys,
                        &meta.loaded_writable_addresses,
                        &meta.loaded_readonly_addresses,
                        invoke,
                    ) {
                        account_fillers::meteora::fill_damm_v2_swap_accounts(
                            e,
                            &get,
                            account_count,
                        );
                    }
                }
            }
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
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_POOLS_PROGRAM) {
                if let Some((invoke, name)) = find_pools_liquidity_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    false,
                    &e.ix_name,
                ) {
                    if e.ix_name.is_empty() {
                        e.ix_name = name.into();
                    }
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::meteora::fill_pools_remove_liquidity_accounts(e, get);
                        }
                    );
                }
            }
        }
        DexEvent::MeteoraPoolsBootstrapLiquidity(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_POOLS_PROGRAM) {
                if let Some((invoke, name)) = find_pools_liquidity_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    true,
                    &e.ix_name,
                ) {
                    if e.ix_name.is_empty() {
                        e.ix_name = name.into();
                    }
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::meteora::fill_pools_bootstrap_liquidity_accounts(
                                e, get,
                            );
                        }
                    );
                }
            }
        }

        DexEvent::MeteoraPoolsPoolCreated(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_POOLS_PROGRAM) {
                if let Some((invoke, name)) = find_pools_management_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    true,
                    &e.ix_name,
                ) {
                    if e.ix_name.is_empty() {
                        e.ix_name = name.into();
                    }
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::meteora::fill_pools_pool_created_accounts(e, get);
                        }
                    );
                }
            }
        }
        DexEvent::MeteoraPoolsSetPoolFees(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_POOLS_PROGRAM) {
                if let Some((invoke, name)) = find_pools_management_invoke(
                    invokes,
                    meta,
                    transaction,
                    e.pool,
                    false,
                    &e.ix_name,
                ) {
                    if e.ix_name.is_empty() {
                        e.ix_name = name.into();
                    }
                    fill_event_accounts_with_invoke!(
                        e,
                        meta,
                        transaction,
                        invoke,
                        |get: &AccountGetter<'_>| {
                            account_fillers::meteora::fill_pools_set_pool_fees_accounts(e, get);
                        }
                    );
                }
            }
        }
        // Meteora DLMM
        DexEvent::MeteoraDlmmSwap(e) => {
            if let Some(invokes) = program_invokes.get_invokes(&METEORA_DLMM_PROGRAM) {
                fill_dlmm_swap_event(e, meta, transaction, invokes);
            }
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

/// 从交易 meta 将缺失账户填入事件（`program_invokes`: program id → (outer, inner) 索引列表）
pub fn fill_accounts_with_owned_keys(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &HashMap<Pubkey, Vec<(i32, i32)>>,
) {
    fill_accounts_with_lookup(event, meta, transaction, program_invokes);
}

#[inline]
pub(crate) fn fill_accounts_with_invoke_context(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &InvokeContext,
) {
    fill_accounts_with_lookup(event, meta, transaction, program_invokes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{
        MeteoraDlmmSwapEvent, OrcaWhirlpoolSwapEvent, PumpSwapBuyEvent, PumpSwapSellEvent,
        RaydiumClmmSwapEvent, RaydiumCpmmSwapEvent, RaydiumLaunchlabTradeEvent,
    };
    use crate::grpc::program_ids::{
        METEORA_DLMM_PROGRAM, ORCA_WHIRLPOOL_PROGRAM, PUMPSWAP_PROGRAM, RAYDIUM_CLMM_PROGRAM,
        RAYDIUM_CPMM_PROGRAM, RAYDIUM_LAUNCHLAB_PROGRAM,
    };
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, Message, MessageHeader, Transaction, TransactionStatusMeta,
    };

    #[test]
    fn pumpswap_dispatcher_preserves_actual_account_count_with_missing_alt_tail() {
        for count in [25usize, 27] {
            let mut keys: Vec<Pubkey> = (0..count).map(|_| Pubkey::new_unique()).collect();
            if count == 27 {
                keys[24] = Pubkey::find_program_address(
                    &[b"pool-v2", keys[3].as_ref()],
                    &PUMPSWAP_PROGRAM,
                )
                .0;
            }
            let pool = keys[0];
            let recipient = keys[count - 2];
            let mut accounts: Vec<u8> = (0..count as u8).collect();
            accounts[count - 1] = 200; // unresolved ALT index, not a shorter instruction
            let transaction = Some(Transaction {
                message: Some(Message {
                    account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                    instructions: vec![CompiledInstruction {
                        accounts,
                        data: crate::instr::pump_amm::discriminators::BUY.to_vec(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            });
            let invokes = HashMap::from([(PUMPSWAP_PROGRAM, vec![(0, -1)])]);
            let mut event = DexEvent::PumpSwapBuy(PumpSwapBuyEvent { pool, ..Default::default() });
            fill_accounts_with_owned_keys(
                &mut event,
                &TransactionStatusMeta::default(),
                &transaction,
                &invokes,
            );
            let DexEvent::PumpSwapBuy(event) = event else { panic!("expected buy") };
            assert_eq!(event.fee_recipient, recipient);
            assert_eq!(event.fee_recipient_quote_token_account, Pubkey::default());
            assert_eq!(event.pool_v2, if count == 27 { keys[24] } else { Pubkey::default() });
        }
    }

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
                        data: crate::instr::pump_amm::discriminators::SELL.to_vec(),
                    },
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: buy_accounts,
                        data: crate::instr::pump_amm::discriminators::BUY.to_vec(),
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
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
    fn damm_swap_only_uses_matching_pool_and_real_swap_instruction() {
        let pools = [Pubkey::new_unique(), Pubkey::new_unique()];
        let mints = [Pubkey::new_unique(), Pubkey::new_unique()];
        let program = crate::grpc::program_ids::METEORA_DAMM_V2_PROGRAM;
        let keys: Vec<Vec<u8>> = [pools[0], pools[1], mints[0], mints[1], program]
            .iter()
            .map(|key| key.to_bytes().to_vec())
            .collect();
        let swap_ix = |pool_idx: u8, mint_idx: u8, disc: [u8; 8]| {
            let mut accounts = vec![4u8; 14];
            accounts[1] = pool_idx;
            accounts[6] = mint_idx;
            CompiledInstruction { program_id_index: 4, accounts, data: disc.to_vec() }
        };
        let transaction = Some(Transaction {
            signatures: vec![vec![0; 64]],
            message: Some(Message {
                header: Some(MessageHeader::default()),
                account_keys: keys,
                recent_blockhash: vec![0; 32],
                instructions: vec![
                    swap_ix(0, 2, crate::instr::meteora_damm::discriminators::SWAP),
                    swap_ix(1, 3, crate::instr::meteora_damm::discriminators::SWAP2),
                    swap_ix(0, 3, [0; 8]),
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(program, vec![(2, -1), (1, -1), (0, -1)])]);
        for (pool, mint) in pools.into_iter().zip(mints) {
            let mut event =
                DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent { pool, ..Default::default() });
            fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);
            let DexEvent::MeteoraDammV2Swap(swap) = event else { unreachable!() };
            assert_eq!(swap.token_a_mint, mint);
            assert_eq!(swap.pool, pool);
        }
        let mut missing = DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
            pool: Pubkey::new_unique(),
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut missing, &meta, &transaction, &invokes);
        let DexEvent::MeteoraDammV2Swap(swap) = missing else { unreachable!() };
        assert_eq!(swap.token_a_mint, Pubkey::default());

        let mut ambiguous = DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
            pool: pools[0],
            ..Default::default()
        });
        let repeated = HashMap::from([(program, vec![(0, -1), (0, -1)])]);
        fill_accounts_with_owned_keys(&mut ambiguous, &meta, &transaction, &repeated);
        let DexEvent::MeteoraDammV2Swap(swap) = ambiguous else { unreachable!() };
        assert_eq!(swap.token_a_mint, Pubkey::default());
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
    fn repeated_pumpswap_pool_matches_direction_user_and_rejects_ambiguity() {
        use yellowstone_grpc_proto::prelude::{InnerInstruction, InnerInstructions};
        for inner in [false, true] {
            let mut f = token_to_token_fixture();
            let message = f.transaction.as_mut().unwrap().message.as_mut().unwrap();
            message.instructions[1].accounts[0] = 0; // both directions now use the sell pool
            message.instructions[1].accounts[1] = 1; // buy user is a different known key
            let second_user = f.buy_pool;
            let mut duplicate = message.instructions[1].clone();
            duplicate.accounts[1] = 2; // another buy user
            message.instructions.push(duplicate);
            f.invokes.insert(PUMPSWAP_PROGRAM, vec![(2, -1), (1, -1), (0, -1)]);
            if inner {
                f.meta.inner_instructions = vec![InnerInstructions {
                    index: 0,
                    instructions: message
                        .instructions
                        .iter()
                        .map(|ix| InnerInstruction {
                            program_id_index: ix.program_id_index,
                            accounts: ix.accounts.clone(),
                            data: ix.data.clone(),
                            ..Default::default()
                        })
                        .collect(),
                }];
                message.instructions.clear();
                f.invokes.insert(PUMPSWAP_PROGRAM, vec![(0, 2), (0, 1), (0, 0)]);
            }
            let mut buy = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                pool: f.sell_pool,
                user: second_user,
                ..Default::default()
            });
            fill_accounts_with_owned_keys(&mut buy, &f.meta, &f.transaction, &f.invokes);
            let DexEvent::PumpSwapBuy(buy) = buy else { unreachable!() };
            assert_eq!(buy.base_mint, f.buy_mint);
            let mut sell = DexEvent::PumpSwapSell(PumpSwapSellEvent {
                pool: f.sell_pool,
                ..Default::default()
            });
            fill_accounts_with_owned_keys(&mut sell, &f.meta, &f.transaction, &f.invokes);
            let DexEvent::PumpSwapSell(sell) = sell else { unreachable!() };
            assert_eq!(sell.base_mint, f.sell_mint);
            let mut ambiguous =
                DexEvent::PumpSwapBuy(PumpSwapBuyEvent { pool: f.sell_pool, ..Default::default() });
            fill_accounts_with_owned_keys(&mut ambiguous, &f.meta, &f.transaction, &f.invokes);
            let DexEvent::PumpSwapBuy(ambiguous) = ambiguous else { unreachable!() };
            assert_eq!(ambiguous.base_mint, Pubkey::default());
            // Neither a different known user nor an unknown pool may select the first invoke.
            let mut unmatched = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                pool: f.sell_pool,
                user: Pubkey::new_unique(),
                ..Default::default()
            });
            fill_accounts_with_owned_keys(&mut unmatched, &f.meta, &f.transaction, &f.invokes);
            let DexEvent::PumpSwapBuy(unmatched) = unmatched else { unreachable!() };
            assert_eq!(unmatched.base_mint, Pubkey::default());
            if inner {
                f.meta.inner_instructions[0].instructions[2].accounts[1] = 1;
            } else {
                f.transaction.as_mut().unwrap().message.as_mut().unwrap().instructions[2]
                    .accounts[1] = 1;
            }
            let mut same_user = DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                pool: f.sell_pool,
                user: second_user,
                ..Default::default()
            });
            fill_accounts_with_owned_keys(&mut same_user, &f.meta, &f.transaction, &f.invokes);
            let DexEvent::PumpSwapBuy(same_user) = same_user else { unreachable!() };
            assert_eq!(same_user.base_mint, Pubkey::default());
        }
    }

    #[test]
    fn unmatched_pumpswap_pool_keeps_missing_accounts_unknown() {
        let f = token_to_token_fixture();
        // An unmatched pool must not inherit another pool's account context.
        let mut sell = DexEvent::PumpSwapSell(PumpSwapSellEvent {
            pool: Pubkey::new_unique(),
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut sell, &f.meta, &f.transaction, &f.invokes);
        match sell {
            DexEvent::PumpSwapSell(e) => assert_eq!(
                e.base_mint,
                Pubkey::default(),
                "an unmatched event must not inherit another pool's mint"
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
        let first_user_token_in = Pubkey::new_unique();
        let first_user_token_out = Pubkey::new_unique();
        let second_user_token_in = Pubkey::new_unique();
        let second_user_token_out = Pubkey::new_unique();
        let padding = Pubkey::new_unique();
        let static_pubkeys = [
            first_pool,
            second_pool,
            first_x_mint,
            first_y_mint,
            second_x_mint,
            second_y_mint,
            first_user_token_in,
            first_user_token_out,
            second_user_token_in,
            second_user_token_out,
            METEORA_DLMM_PROGRAM,
            padding,
        ];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();

        let dlmm_accounts =
            |len, pool_index, user_in_index, user_out_index, x_mint_index, y_mint_index| {
                let mut accounts = vec![11u8; len];
                accounts[0] = pool_index;
                accounts[4] = user_in_index;
                accounts[5] = user_out_index;
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
                        program_id_index: 10,
                        accounts: dlmm_accounts(20, 0, 6, 7, 2, 3),
                        data: [crate::instr::meteora_dlmm::discriminators::SWAP.as_slice(),
                               &1u64.to_le_bytes(), &0u64.to_le_bytes()].concat(),
                    },
                    CompiledInstruction {
                        program_id_index: 10,
                        accounts: dlmm_accounts(15, 1, 8, 9, 4, 5),
                        data: [crate::instr::meteora_dlmm::discriminators::SWAP.as_slice(),
                               &1u64.to_le_bytes(), &0u64.to_le_bytes()].concat(),
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(METEORA_DLMM_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);
        let swap_event = |pool| {
            DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
                metadata: EventMetadata::default(),
                token_x_mint: Pubkey::default(),
                token_y_mint: Pubkey::default(),
                user_token_in: Pubkey::default(),
                user_token_out: Pubkey::default(),
                min_amount_out: 0,
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
                ..Default::default()
            })
        };

        let mut first_event = swap_event(first_pool);
        fill_accounts_with_owned_keys(&mut first_event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraDlmmSwap(first_event) = first_event else {
            unreachable!();
        };
        assert_eq!(first_event.token_x_mint, first_x_mint);
        assert_eq!(first_event.token_y_mint, first_y_mint);
        assert_eq!(first_event.user_token_in, first_user_token_in);
        assert_eq!(first_event.user_token_out, first_user_token_out);

        let mut second_event = swap_event(second_pool);
        fill_accounts_with_owned_keys(&mut second_event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraDlmmSwap(second_event) = second_event else {
            unreachable!();
        };
        assert_eq!(second_event.token_x_mint, second_x_mint);
        assert_eq!(second_event.token_y_mint, second_y_mint);
        assert_eq!(second_event.user_token_in, second_user_token_in);
        assert_eq!(second_event.user_token_out, second_user_token_out);
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
                config: None,
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
            ..Default::default()
        });

        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);

        let DexEvent::RaydiumLaunchlabTrade(event) = event else {
            unreachable!();
        };
        assert_eq!(event.quote_mint, first_quote_mint);
        assert_ne!(event.quote_mint, second_quote_mint);
    }

    #[test]
    fn clmm_log_backfill_uses_complete_instruction_account_count() {
        use crate::grpc::program_ids::RAYDIUM_CLMM_PROGRAM;
        let count = 79;
        let mut keys: Vec<_> = (0..=count).map(|_| Pubkey::new_unique()).collect();
        keys[10] = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        keys[count] = RAYDIUM_CLMM_PROGRAM;
        let bitmap = account_fillers::raydium::tick_array_bitmap_extension_pda(&keys[2]);
        keys[75] = bitmap;
        keys[25] = Pubkey::default();
        let transaction = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![CompiledInstruction {
                    program_id_index: count as u32,
                    accounts: (0..count as u8).collect(),
                    data: crate::instr::raydium_clmm::discriminators::SWAP_V2.to_vec(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        let invokes = HashMap::from([(RAYDIUM_CLMM_PROGRAM, vec![(0, -1)])]);
        let mut event = DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            pool_state: keys[2],
            sender: keys[0],
            token_account_0: keys[3],
            token_account_1: keys[4],
            tick_array_bitmap_extension: Some(bitmap),
            ..Default::default()
        });
        fill_accounts_with_owned_keys(
            &mut event,
            &TransactionStatusMeta::default(),
            &transaction,
            &invokes,
        );
        let DexEvent::RaydiumClmmSwap(event) = event else { panic!("swap") };
        assert_eq!(
            event.tick_arrays,
            keys[13..count].iter().copied().filter(|key| *key != bitmap).collect::<Vec<_>>()
        );
        assert_eq!(event.tick_array_bitmap_extension, Some(bitmap));
    }

    #[test]
    fn clmm_same_pool_v2_without_ticks_does_not_inherit_legacy_ticks() {
        use crate::grpc::program_ids::RAYDIUM_CLMM_PROGRAM;
        let mut keys: Vec<_> = (0..14).map(|_| Pubkey::new_unique()).collect();
        keys[9] = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        keys[13] = RAYDIUM_CLMM_PROGRAM;
        let transaction = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 13,
                        accounts: vec![2, 1, 0, 3, 4, 5, 6, 7, 10, 8],
                        data: crate::instr::raydium_clmm::discriminators::SWAP.to_vec(),
                    },
                    CompiledInstruction {
                        program_id_index: 13,
                        accounts: vec![2, 1, 0, 3, 4, 5, 6, 7, 10, 10, 9, 11, 12],
                        data: crate::instr::raydium_clmm::discriminators::SWAP_V2.to_vec(),
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        });
        let invokes = HashMap::from([(RAYDIUM_CLMM_PROGRAM, vec![(0, -1), (1, -1)])]);
        let mut event = DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            pool_state: keys[0],
            sender: keys[2],
            token_account_0: keys[3],
            token_account_1: keys[4],
            input_mint: keys[11],
            output_mint: keys[12],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(
            &mut event,
            &TransactionStatusMeta::default(),
            &transaction,
            &invokes,
        );
        let DexEvent::RaydiumClmmSwap(event) = event else { panic!("swap") };
        assert_eq!(event.amm_config, keys[1]);
        assert!(event.tick_arrays.is_empty());
        assert!(event.tick_array_bitmap_extension.is_none());
        let mut reversed = DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            pool_state: keys[0],
            sender: keys[2],
            token_account_0: keys[4],
            token_account_1: keys[3],
            input_mint: keys[11],
            output_mint: keys[12],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(
            &mut reversed,
            &TransactionStatusMeta::default(),
            &transaction,
            &invokes,
        );
        let DexEvent::RaydiumClmmSwap(reversed) = reversed else { panic!("swap") };
        assert_eq!(reversed.amm_config, keys[1]);
        assert!(reversed.tick_arrays.is_empty());
    }

    #[test]
    fn clmm_multi_swap_backfills_amm_config_from_matching_pool() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let first_config = Pubkey::new_unique();
        let second_config = Pubkey::new_unique();
        let padding = Pubkey::new_unique();
        let static_pubkeys =
            [first_pool, second_pool, first_config, second_config, RAYDIUM_CLMM_PROGRAM, padding];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();
        // swap_v2 layout: 0 payer, 1 amm_config, 2 pool_state, ...
        let clmm_accounts = |config_index, pool_index| {
            let mut accounts = vec![5u8; 15];
            accounts[1] = config_index;
            accounts[2] = pool_index;
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
                        accounts: clmm_accounts(2, 0),
                        data: crate::instr::raydium_clmm::discriminators::SWAP_V2.to_vec(),
                    },
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: clmm_accounts(3, 1),
                        data: crate::instr::raydium_clmm::discriminators::SWAP_V2.to_vec(),
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(RAYDIUM_CLMM_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);

        let mut first = DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            pool_state: first_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut first, &meta, &transaction, &invokes);
        let DexEvent::RaydiumClmmSwap(first) = first else {
            unreachable!();
        };
        assert_eq!(first.amm_config, first_config);
        assert_ne!(first.amm_config, second_config);

        let mut second = DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            pool_state: second_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut second, &meta, &transaction, &invokes);
        let DexEvent::RaydiumClmmSwap(second) = second else {
            unreachable!();
        };
        assert_eq!(second.amm_config, second_config);
        assert_ne!(second.amm_config, first_config);
    }

    #[test]
    fn cpmm_lp_users_match_kind_pool_and_unique_invocation() {
        use crate::instr::raydium_cpmm::discriminators::{DEPOSIT, INITIALIZE, WITHDRAW};
        let keys: Vec<_> = (0..5).map(|_| Pubkey::new_unique()).collect();
        let mut tx = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![
                    CompiledInstruction {
                        accounts: vec![0, 0, 2],
                        data: DEPOSIT.to_vec(),
                        ..Default::default()
                    },
                    CompiledInstruction {
                        accounts: vec![1, 0, 3],
                        data: DEPOSIT.to_vec(),
                        ..Default::default()
                    },
                    CompiledInstruction {
                        accounts: vec![4, 0, 3],
                        data: WITHDRAW.to_vec(),
                        ..Default::default()
                    },
                    CompiledInstruction {
                        accounts: vec![4, 0, 0, 3],
                        data: INITIALIZE.to_vec(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        });
        let meta = TransactionStatusMeta::default();
        let mut invokes = vec![(0, -1), (1, -1), (2, -1), (3, -1)];
        let mut event = DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
            metadata: Default::default(),
            pool: keys[3],
            user: Pubkey::default(),
            token0_amount: 1,
            token1_amount: 2,
            lp_token_amount: 3,
        });
        fill_accounts_with_owned_keys(
            &mut event,
            &meta,
            &tx,
            &HashMap::from([(RAYDIUM_CPMM_PROGRAM, invokes.clone())]),
        );
        let DexEvent::RaydiumCpmmDeposit(e) = &event else { panic!("deposit") };
        assert_eq!(e.user, keys[1]);
        for (disc, index, expected) in [(WITHDRAW, 2, 2), (INITIALIZE, 3, 3)] {
            assert_eq!(
                find_cpmm_invoke(&invokes, &meta, &tx, keys[3], index, disc),
                Some(&invokes[expected])
            );
        }
        // Two identical kind/pool invocations cannot be assigned by guessing.
        let duplicate = tx.as_ref().unwrap().message.as_ref().unwrap().instructions[1].clone();
        tx.as_mut().unwrap().message.as_mut().unwrap().instructions.push(duplicate);
        invokes.push((4, -1));
        assert!(find_cpmm_invoke(&invokes, &meta, &tx, keys[3], 2, DEPOSIT).is_none());
        assert!(find_cpmm_invoke(&invokes, &meta, &tx, Pubkey::default(), 2, DEPOSIT).is_none());
    }

    #[test]
    fn cpmm_multi_swap_backfills_amm_config_from_matching_pool() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let first_config = Pubkey::new_unique();
        let second_config = Pubkey::new_unique();
        let padding = Pubkey::new_unique();
        let static_pubkeys =
            [first_pool, second_pool, first_config, second_config, RAYDIUM_CPMM_PROGRAM, padding];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();
        // swap_base_input: 0 payer, 1 authority, 2 amm_config, 3 pool_state, ...
        let cpmm_accounts = |config_index, pool_index| {
            let mut accounts = vec![5u8; 13];
            accounts[2] = config_index;
            accounts[3] = pool_index;
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
                        accounts: cpmm_accounts(2, 0),
                        data: crate::instr::raydium_cpmm::discriminators::SWAP_BASE_OUT.to_vec(),
                    },
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: cpmm_accounts(3, 1),
                        data: crate::instr::raydium_cpmm::discriminators::SWAP_BASE_OUT.to_vec(),
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(RAYDIUM_CPMM_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);

        let mut first = DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
            pool_id: first_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut first, &meta, &transaction, &invokes);
        let DexEvent::RaydiumCpmmSwap(first) = first else {
            unreachable!();
        };
        assert_eq!(first.amm_config, first_config);
        assert_ne!(first.amm_config, second_config);

        let mut second = DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
            pool_id: second_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut second, &meta, &transaction, &invokes);
        let DexEvent::RaydiumCpmmSwap(second) = second else {
            unreachable!();
        };
        assert_eq!(second.amm_config, second_config);
        assert_ne!(second.amm_config, first_config);
    }

    #[test]
    fn whirlpool_multi_swap_v2_backfills_vaults_from_matching_pool() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let first_vault_a = Pubkey::new_unique();
        let second_vault_a = Pubkey::new_unique();
        let memo = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        let padding = Pubkey::new_unique();
        // indices: 0 first_pool, 1 second_pool, 2 first_vault, 3 second_vault,
        // 4 program, 5 memo, 6 padding
        let static_pubkeys = [
            first_pool,
            second_pool,
            first_vault_a,
            second_vault_a,
            ORCA_WHIRLPOOL_PROGRAM,
            memo,
            padding,
        ];
        let account_keys = static_pubkeys.iter().map(|key| key.to_bytes().to_vec()).collect();
        // swap_v2: 0 tp_a, 1 tp_b, 2 memo, 3 authority, 4 whirlpool, ..., 8 vault_a
        let wp_accounts = |pool_index, vault_a_index| {
            let mut accounts = vec![6u8; 15];
            accounts[2] = 5; // memo
            accounts[4] = pool_index;
            accounts[8] = vault_a_index;
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
                        accounts: wp_accounts(0, 2),
                        data: vec![0],
                    },
                    // Second leg has MORE accounts so length heuristic would pick it
                    // if anchoring failed — pad with an extra remaining key.
                    CompiledInstruction {
                        program_id_index: 4,
                        accounts: {
                            let mut a = wp_accounts(1, 3);
                            a.push(6);
                            a
                        },
                        data: vec![0],
                    },
                ],
                versioned: false,
                address_table_lookups: Vec::new(),
                config: None,
            }),
        });
        let meta = TransactionStatusMeta::default();
        let invokes = HashMap::from([(ORCA_WHIRLPOOL_PROGRAM, vec![(0i32, -1i32), (1i32, -1i32)])]);

        let mut first = DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
            whirlpool: first_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut first, &meta, &transaction, &invokes);
        let DexEvent::OrcaWhirlpoolSwap(first) = first else {
            unreachable!();
        };
        assert_eq!(first.token_vault_a, first_vault_a);
        assert_ne!(first.token_vault_a, second_vault_a);

        let mut second = DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
            whirlpool: second_pool,
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut second, &meta, &transaction, &invokes);
        let DexEvent::OrcaWhirlpoolSwap(second) = second else {
            unreachable!();
        };
        assert_eq!(second.token_vault_a, second_vault_a);
        assert_ne!(second.token_vault_a, first_vault_a);
    }
}

#[cfg(test)]
mod pools_liquidity_dispatch_tests {
    use super::*;
    use crate::instr::meteora_amm::discriminators::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};

    #[test]
    fn liquidity_dispatch_uses_discriminator_and_pool_with_ambiguous_lengths() {
        let keys: Vec<_> = (0..64).map(|_| Pubkey::new_unique()).collect();
        let instructions: Vec<_> = [
            (SWAP, 0u8, 16u8),
            (REMOVE_LIQUIDITY, 16, 16),
            (REMOVE_LIQUIDITY_SINGLE_SIDE, 32, 15),
            (BOOTSTRAP_LIQUIDITY, 48, 16),
        ]
        .into_iter()
        .map(|(disc, start, count)| CompiledInstruction {
            data: disc.to_vec(),
            accounts: (start..start + count).collect(),
            ..Default::default()
        })
        .collect();
        let transaction = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions,
                ..Default::default()
            }),
            ..Default::default()
        });
        let invokes = HashMap::from([(
            crate::grpc::program_ids::METEORA_POOLS_PROGRAM,
            vec![(0, -1), (1, -1), (2, -1), (3, -1)],
        )]);
        let meta = TransactionStatusMeta::default();
        let mut event = DexEvent::MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent {
            pool: keys[32],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraPoolsRemoveLiquidity(e) = event else { panic!("remove") };
        assert_eq!(e.ix_name, "remove_liquidity_single_side");
        assert_eq!(e.user_destination_token, keys[43]);
        assert_eq!(e.user, keys[44]);
        assert_eq!(e.vault_program, keys[45]);
        assert_eq!(e.token_program, keys[46]);
        assert_eq!((e.user_a_token, e.user_b_token), (Pubkey::default(), Pubkey::default()));
        let mut event =
            DexEvent::MeteoraPoolsBootstrapLiquidity(MeteoraPoolsBootstrapLiquidityEvent {
                pool: keys[48],
                ..Default::default()
            });
        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraPoolsBootstrapLiquidity(e) = event else { panic!("bootstrap") };
        assert_eq!(e.ix_name, "bootstrap_liquidity");
        assert_eq!(e.user_a_token, keys[59]);
        assert_eq!(e.user_b_token, keys[60]);
        assert_eq!(e.user, keys[61]);
        assert_eq!(e.token_program, keys[63]);
        let invoke_list = [(0, -1), (1, -1), (2, -1), (3, -1)];
        assert!(find_pools_liquidity_invoke(
            &invoke_list,
            &meta,
            &transaction,
            keys[32],
            false,
            "remove_balance_liquidity"
        )
        .is_none());
        assert!(find_pools_liquidity_invoke(
            &invoke_list,
            &meta,
            &transaction,
            Pubkey::new_unique(),
            false,
            ""
        )
        .is_none());
        // Without a pool, type still excludes swaps and deposits.
        assert_eq!(
            find_pools_liquidity_invoke(
                &invoke_list,
                &meta,
                &transaction,
                Pubkey::default(),
                true,
                ""
            )
            .unwrap()
            .0,
            &(3, -1)
        );
    }
}

#[cfg(test)]
mod pools_management_dispatch_tests {
    use super::*;
    use crate::instr::meteora_amm::discriminators::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};

    #[test]
    fn mixed_operations_dispatch_by_type_and_known_pool() {
        let keys: Vec<_> = (0..56).map(|_| Pubkey::new_unique()).collect();
        let instructions: Vec<_> = [
            (SWAP, 0u8, 26u8),
            (CREATE_POOL_WITH_CONFIG2, 26, 26),
            (SET_POOL_FEES, 52, 2),
            (SET_POOL_FEES, 54, 2),
        ]
        .into_iter()
        .map(|(d, start, count)| CompiledInstruction {
            data: d.to_vec(),
            accounts: (start..start + count).collect(),
            ..Default::default()
        })
        .collect();
        let transaction = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions,
                ..Default::default()
            }),
            ..Default::default()
        });
        let invokes = HashMap::from([(
            crate::grpc::program_ids::METEORA_POOLS_PROGRAM,
            vec![(0, -1), (1, -1), (2, -1), (3, -1)],
        )]);
        let meta = TransactionStatusMeta::default();
        let mut event = DexEvent::MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent {
            pool: keys[26],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraPoolsPoolCreated(e) = event else { panic!("create") };
        assert_eq!(e.ix_name, "initialize_permissionless_constant_product_pool_with_config2");
        assert_eq!(
            (e.config, e.lp_mint, e.token_a_mint, e.token_b_mint),
            (keys[27], keys[28], keys[29], keys[30])
        );
        assert_eq!((e.payer, e.token_program, e.system_program), (keys[44], keys[49], keys[51]));
        let mut event = DexEvent::MeteoraPoolsSetPoolFees(MeteoraPoolsSetPoolFeesEvent {
            pool: keys[54],
            ..Default::default()
        });
        fill_accounts_with_owned_keys(&mut event, &meta, &transaction, &invokes);
        let DexEvent::MeteoraPoolsSetPoolFees(e) = event else { panic!("fees") };
        assert_eq!(e.ix_name, "set_pool_fees");
        assert_eq!(e.fee_operator, keys[55]);
        let invoke_list = [(0, -1), (1, -1), (2, -1), (3, -1)];
        assert!(find_pools_management_invoke(
            &invoke_list,
            &meta,
            &transaction,
            keys[54],
            true,
            ""
        )
        .is_none());
        assert!(find_pools_management_invoke(
            &invoke_list,
            &meta,
            &transaction,
            keys[26],
            true,
            "initialize_permissionless_constant_product_pool_with_config"
        )
        .is_none());
    }
}

#[cfg(test)]
mod remaining_creation_dispatch_tests {
    use super::*;
    use crate::instr::meteora_amm::discriminators::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};
    #[test]
    fn every_creation_layout_is_selected_from_its_own_instruction() {
        for (disc, name, count, lp_index, payer_index, token_index) in [
            (INITIALIZE_PERMISSIONED_POOL, "initialize_permissioned_pool", 24u8, 1, 15, 21),
            (INITIALIZE_PERMISSIONLESS_POOL, "initialize_permissionless_pool", 26, 1, 17, 23),
            (
                INITIALIZE_PERMISSIONLESS_POOL_WITH_FEE_TIER,
                "initialize_permissionless_pool_with_fee_tier",
                26,
                1,
                17,
                23,
            ),
            (
                INITIALIZE_CUSTOMIZABLE_POOL,
                "initialize_customizable_permissionless_constant_product_pool",
                25,
                1,
                17,
                22,
            ),
        ] {
            let keys: Vec<_> = (0..count + 27).map(|_| Pubkey::new_unique()).collect();
            let transaction = Some(Transaction {
                message: Some(Message {
                    account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                    instructions: vec![
                        CompiledInstruction {
                            data: CREATE_POOL.to_vec(),
                            accounts: (0..26).collect(),
                            ..Default::default()
                        },
                        CompiledInstruction {
                            data: disc.to_vec(),
                            accounts: (27..27 + count).collect(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            });
            let invokes = HashMap::from([(
                crate::grpc::program_ids::METEORA_POOLS_PROGRAM,
                vec![(0, -1), (1, -1)],
            )]);
            let mut event = DexEvent::MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent {
                pool: keys[27],
                ..Default::default()
            });
            fill_accounts_with_owned_keys(
                &mut event,
                &TransactionStatusMeta::default(),
                &transaction,
                &invokes,
            );
            let DexEvent::MeteoraPoolsPoolCreated(e) = event else { panic!("create") };
            assert_eq!(e.ix_name, name);
            assert_eq!(e.lp_mint, keys[27 + lp_index]);
            assert_eq!(e.token_program, keys[27 + token_index]);
            if name == "initialize_permissioned_pool" {
                assert_eq!(e.admin, keys[27 + payer_index]);
                assert_eq!(e.payer, Pubkey::default());
            } else {
                assert_eq!(e.payer, keys[27 + payer_index]);
                assert_eq!(e.admin, Pubkey::default());
            }
            assert_eq!(e.config, Pubkey::default());
        }
    }
}

#[cfg(test)]
mod review_matching_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};

    #[test]
    fn known_pool_never_falls_back_to_an_unrelated_or_ambiguous_invocation() {
        let pool = Pubkey::new_unique();
        let foreign = Pubkey::new_unique();
        let tx = Some(Transaction {
            message: Some(Message {
                account_keys: vec![pool.to_bytes().to_vec(), foreign.to_bytes().to_vec()],
                instructions: vec![
                    CompiledInstruction { accounts: vec![1], ..Default::default() },
                    CompiledInstruction { accounts: vec![0], ..Default::default() },
                    CompiledInstruction { accounts: vec![0], ..Default::default() },
                ],
                ..Default::default()
            }),
            ..Default::default()
        });
        let meta = TransactionStatusMeta::default();
        let keys = tx.as_ref().unwrap().message.as_ref().map(|m| &m.account_keys);
        assert!(find_instruction_invoke_anchored(&[(0, -1)], &meta, &tx, keys, 0, &pool).is_none());
        assert_eq!(
            find_instruction_invoke_anchored(&[(0, -1), (1, -1)], &meta, &tx, keys, 0, &pool),
            Some(&(1, -1))
        );
        assert!(find_instruction_invoke_anchored(&[(1, -1), (2, -1)], &meta, &tx, keys, 0, &pool)
            .is_none());
    }

    #[test]
    fn repeated_clmm_swaps_require_unique_context_and_the_right_discriminator() {
        let keys: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
        let swap = CompiledInstruction {
            accounts: (0..13).collect(),
            data: crate::instr::raydium_clmm::discriminators::SWAP_V2.to_vec(),
            ..Default::default()
        };
        let tx = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![swap.clone(), swap],
                ..Default::default()
            }),
            ..Default::default()
        });
        let meta = TransactionStatusMeta::default();
        let mut event =
            RaydiumClmmSwapEvent { pool_state: keys[2], sender: keys[0], ..Default::default() };
        assert!(find_clmm_swap_invoke(&[(0, -1), (1, -1)], &meta, &tx, &event).is_none());
        assert_eq!(find_clmm_swap_invoke(&[(0, -1)], &meta, &tx, &event), Some(&(0, -1)));
        event.ix_name = "swap".into();
        assert!(find_clmm_swap_invoke(&[(0, -1)], &meta, &tx, &event).is_none());
    }
}

#[cfg(test)]
mod cpmm_swap_matching_regressions {
    use super::*;
    use crate::grpc::program_ids::RAYDIUM_CPMM_PROGRAM;
    use crate::instr::raydium_cpmm::discriminators::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};

    #[test]
    fn swap_accounts_require_the_matching_kind_and_a_unique_complete_invocation() {
        let keys: Vec<_> = (0..17).map(|_| Pubkey::new_unique()).collect();
        let ix = |disc: [u8; 8], payer| {
            let mut accounts: Vec<u8> = (0..16).collect();
            accounts[0] = payer;
            CompiledInstruction { accounts, data: disc.to_vec(), ..Default::default() }
        };
        let mut tx = Some(Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![
                    ix(COLLECT_CREATOR_FEE_PERMISSIONLESS, 16),
                    ix(INITIALIZE, 16),
                    ix(SWAP_BASE_IN, 0),
                    ix(SWAP_BASE_OUT, 16),
                ],
                ..Default::default()
            }),
            ..Default::default()
        });
        let meta = TransactionStatusMeta::default();
        let fill = |tx: &Option<Transaction>, indices: Vec<(i32, i32)>, base_input| {
            let mut event = DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
                pool_id: keys[3],
                base_input,
                ..Default::default()
            });
            fill_accounts_with_owned_keys(
                &mut event,
                &meta,
                tx,
                &HashMap::from([(RAYDIUM_CPMM_PROGRAM, indices)]),
            );
            let DexEvent::RaydiumCpmmSwap(e) = event else { panic!("swap") };
            e
        };
        assert_eq!(fill(&tx, vec![(0, -1), (1, -1), (2, -1), (3, -1)], true).payer, keys[0]);
        assert_eq!(fill(&tx, vec![(0, -1), (1, -1), (2, -1), (3, -1)], false).payer, keys[16]);
        assert_eq!(fill(&tx, vec![(0, -1), (1, -1)], true).payer, Pubkey::default());
        let duplicate = tx.as_ref().unwrap().message.as_ref().unwrap().instructions[2].clone();
        tx.as_mut().unwrap().message.as_mut().unwrap().instructions.push(duplicate);
        assert_eq!(fill(&tx, vec![(2, -1), (4, -1)], true).payer, Pubkey::default());
        tx.as_mut().unwrap().message.as_mut().unwrap().instructions[2].accounts.truncate(4);
        assert_eq!(fill(&tx, vec![(2, -1)], true).payer, Pubkey::default());
    }
}

#[cfg(test)]
mod review_dlmm_context_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};
    #[test]
    fn dlmm_hook_boundaries_are_filled_only_from_unique_matching_swap() {
        let keys: Vec<_> = (0..41).map(|_| Pubkey::new_unique()).collect();
        let accounts_a: Vec<u8> = (1..21).collect();
        let mut accounts_b: Vec<u8> = (21..41).collect();
        accounts_b[0] = accounts_a[0];
        let mut data = crate::instr::meteora_dlmm::discriminators::SWAP2.to_vec();
        data.extend_from_slice(&[0;16]); data.extend_from_slice(&1u32.to_le_bytes()); data.extend_from_slice(&[0,2]);
        let transaction = Some(Transaction { message: Some(Message {
            account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
            instructions: vec![CompiledInstruction { accounts: accounts_a.clone(), data: data.clone(), ..Default::default() },
                CompiledInstruction { accounts: accounts_b, data, ..Default::default() }], ..Default::default()
        }), ..Default::default() });
        let meta = TransactionStatusMeta::default();
        let mut event = MeteoraDlmmSwapEvent { pool: keys[1], ..Default::default() };
        fill_dlmm_swap_event(&mut event, &meta, &transaction, &[(0,-1),(1,-1)]);
        assert!(event.bin_arrays.is_empty()); assert_eq!(event.user_token_in, Pubkey::default());
        event.from = keys[11];
        fill_dlmm_swap_event(&mut event, &meta, &transaction, &[(0,-1),(1,-1)]);
        assert_eq!(event.bin_arrays, vec![keys[19],keys[20]]);
        assert_eq!(event.user_token_in, keys[5]);
        // A non-swap invoke sharing the pool cannot supply account context.
        let mut invalid = transaction.clone();
        invalid.as_mut().unwrap().message.as_mut().unwrap().instructions[0].data.truncate(24);
        let mut event = MeteoraDlmmSwapEvent { pool: keys[1], from: keys[11], ..Default::default() };
        fill_dlmm_swap_event(&mut event, &meta, &invalid, &[(0,-1)]);
        assert!(event.bin_arrays.is_empty());
    }
}
