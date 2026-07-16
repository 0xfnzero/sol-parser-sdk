//! Yellowstone `SubscribeUpdateTransaction` 单笔解析（logs ∥ instructions + 去重）。
//! 从 [`super::client`] 抽出，供 crate 内与下游 streamer 复用。

use std::collections::HashMap;
use std::str::FromStr;

use memchr::memmem;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::prelude::{
    SubscribeUpdateTransaction, Transaction, TransactionStatusMeta,
};

use super::transaction_meta::try_yellowstone_signature;
use super::types::EventTypeFilter;
use crate::DexEvent;

static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: "));

/// 解析单笔 Yellowstone 交易更新（含 meta）：并行 logs + enhanced instructions，再 log/ix 去重合并。
#[inline]
pub fn parse_subscribe_update_transaction(
    tx: &SubscribeUpdateTransaction,
    grpc_recv_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    parse_transaction_core(tx, grpc_recv_us, block_us, filter)
}

#[inline]
pub(crate) fn parse_transaction_core(
    tx: &SubscribeUpdateTransaction,
    grpc_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let Some(info) = &tx.transaction else { return Vec::new() };
    let Some(meta) = &info.meta else { return Vec::new() };

    let sig = extract_signature(&info.signature);
    let slot = tx.slot;
    let idx = info.index;

    let (log_events, instr_events) = rayon::join(
        || {
            parse_logs(
                meta,
                &info.transaction,
                &meta.log_messages,
                sig,
                slot,
                idx,
                block_us,
                grpc_us,
                filter,
            )
        },
        || parse_instructions(meta, &info.transaction, sig, slot, idx, block_us, grpc_us, filter),
    );

    let events =
        crate::grpc::log_instr_dedup::dedupe_log_instruction_events(log_events, instr_events);
    if let Some(filter) = filter {
        events.into_iter().map(|e| filter.normalize_dex_event(e)).collect()
    } else {
        events
    }
}

/// 单笔交易解析：**顺序**执行 logs → instructions 再合并。
///
/// 与 [`parse_subscribe_update_transaction`]（内部 `rayon::join` 并行）算法一致，但避免工作窃取与线程池调度，
/// 在「单笔极低延迟」场景通常更快；适合嵌入 latency-sensitive 的订阅流水线。
#[inline]
pub fn parse_subscribe_update_transaction_low_latency(
    tx: &SubscribeUpdateTransaction,
    grpc_recv_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    parse_transaction_core_sequential(tx, grpc_recv_us, block_us, filter)
}

#[inline]
fn parse_transaction_core_sequential(
    tx: &SubscribeUpdateTransaction,
    grpc_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let Some(info) = &tx.transaction else {
        return Vec::new();
    };
    let Some(meta) = &info.meta else {
        return Vec::new();
    };

    let sig = extract_signature(&info.signature);
    let slot = tx.slot;
    let idx = info.index;

    let log_events = parse_logs(
        meta,
        &info.transaction,
        &meta.log_messages,
        sig,
        slot,
        idx,
        block_us,
        grpc_us,
        filter,
    );
    let instr_events =
        parse_instructions(meta, &info.transaction, sig, slot, idx, block_us, grpc_us, filter);

    let events =
        crate::grpc::log_instr_dedup::dedupe_log_instruction_events(log_events, instr_events);
    if let Some(filter) = filter {
        events.into_iter().map(|e| filter.normalize_dex_event(e)).collect()
    } else {
        events
    }
}

#[inline(always)]
pub(crate) fn extract_signature(bytes: &[u8]) -> solana_sdk::signature::Signature {
    try_yellowstone_signature(bytes).expect("yellowstone signature must be 64 bytes")
}

#[inline]
fn parse_logs(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    logs: &[String],
    sig: solana_sdk::signature::Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let recent_blockhash = transaction.as_ref().and_then(|t| t.message.as_ref()).and_then(|m| {
        if m.recent_blockhash.is_empty() {
            None
        } else {
            Some(m.recent_blockhash.clone())
        }
    });

    let needs_pumpfun = filter.map(|f| f.includes_pumpfun()).unwrap_or(true);
    let has_create = needs_pumpfun && crate::logs::optimized_matcher::detect_pumpfun_create(logs);

    let mut outer_idx: i32 = -1;
    let mut inner_idx: i32 = -1;
    let mut invokes: HashMap<Pubkey, Vec<(i32, i32)>> = HashMap::with_capacity(8);
    let mut active_program_stack: Vec<Pubkey> = Vec::with_capacity(8);
    let mut result = Vec::with_capacity(4);

    for log in logs {
        if let Some((pid, depth)) = crate::logs::optimized_matcher::parse_invoke_info(log) {
            if depth == 1 {
                inner_idx = -1;
                outer_idx += 1;
            } else {
                inner_idx += 1;
            }
            if let Ok(pk) = Pubkey::from_str(pid) {
                active_program_stack.truncate(depth.saturating_sub(1));
                active_program_stack.push(pk);
                invokes.entry(pk).or_default().push((outer_idx, inner_idx));
            }
        }

        if PROGRAM_DATA_FINDER.find(log.as_bytes()).is_some() {
            let current_program = active_program_stack.last();
            if let Some(mut e) = crate::logs::parse_log_with_program_id(
                log,
                sig,
                slot,
                tx_idx,
                block_us,
                grpc_us,
                filter,
                has_create,
                recent_blockhash.as_deref(),
                current_program,
            ) {
                crate::core::account_dispatcher::fill_accounts_with_owned_keys(
                    &mut e,
                    meta,
                    transaction,
                    &invokes,
                );
                crate::core::common_filler::fill_data(&mut e, meta, transaction, &invokes);
                result.push(e);
            }
        }

        if let Some(pid) = crate::logs::optimized_matcher::parse_program_complete_info(log) {
            if let Ok(pk) = Pubkey::from_str(pid) {
                if let Some(pos) = active_program_stack.iter().rposition(|active| *active == pk) {
                    active_program_stack.truncate(pos);
                }
            }
        }
    }
    result
}

#[inline]
fn parse_instructions(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    sig: solana_sdk::signature::Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    crate::grpc::instruction_parser::parse_instructions_enhanced(
        meta,
        transaction,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        filter,
    )
}
