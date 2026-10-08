//! Yellowstone `SubscribeUpdateTransaction` 单笔解析（logs ∥ instructions + 去重）。
//! 从 [`super::client`] 抽出，供 crate 内与下游 streamer 复用。

use smallvec::SmallVec;
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::prelude::{
    SubscribeUpdateTransaction, Transaction, TransactionStatusMeta,
};

use super::transaction_meta::try_yellowstone_signature;
use super::types::EventTypeFilter;
use crate::DexEvent;

const PROGRAM_DATA_PREFIX: &[u8] = b"Program data: ";

struct ActiveProgram<'a> {
    encoded: &'a str,
    pubkey: Pubkey,
    position: (i32, i32),
}

/// 解析成功的 Yellowstone 交易更新（含 meta）：并行 logs + enhanced instructions，再 log/ix 去重合并。
/// 失败交易整体回滚，即使含有 DEX 日志或指令，也返回空事件列表。
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

    // Logs and decoded instructions from a failed transaction describe rolled-back work.
    // Reject before signature decoding, allocation, or Rayon dispatch.
    if meta.err.is_some() {
        return Vec::new();
    }

    let Some(sig) = try_yellowstone_signature(&info.signature) else {
        return Vec::new();
    };
    let slot = tx.slot;
    let idx = info.index;
    let needs_pumpfun = filter.map(EventTypeFilter::includes_pumpfun).unwrap_or(true);
    let is_created_buy =
        needs_pumpfun && crate::logs::optimized_matcher::detect_pumpfun_create(&meta.log_messages);

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
                is_created_buy,
            )
        },
        || {
            parse_instructions(
                meta,
                &info.transaction,
                sig,
                slot,
                idx,
                block_us,
                grpc_us,
                filter,
                is_created_buy,
            )
        },
    );

    let mut events =
        crate::grpc::log_instr_dedup::dedupe_log_instruction_events(log_events, instr_events);
    crate::grpc::transaction_meta::fill_recent_blockhash(&mut events, &info.transaction);
    for event in &mut events {
        crate::core::common_filler::fill_token_balances(event, meta, &info.transaction);
    }
    if let Some(filter) = filter {
        events.into_iter().map(|e| filter.normalize_dex_event(e)).collect()
    } else {
        events
    }
}

/// 成功交易解析：**顺序**执行 logs → instructions 再合并；失败交易返回空列表。
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

    // Logs and decoded instructions from a failed transaction describe rolled-back work.
    // Reject before signature decoding, allocation, or Rayon dispatch.
    if meta.err.is_some() {
        return Vec::new();
    }

    let Some(sig) = try_yellowstone_signature(&info.signature) else {
        return Vec::new();
    };
    let slot = tx.slot;
    let idx = info.index;
    let needs_pumpfun = filter.map(EventTypeFilter::includes_pumpfun).unwrap_or(true);
    let is_created_buy =
        needs_pumpfun && crate::logs::optimized_matcher::detect_pumpfun_create(&meta.log_messages);

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
        is_created_buy,
    );
    let instr_events = parse_instructions(
        meta,
        &info.transaction,
        sig,
        slot,
        idx,
        block_us,
        grpc_us,
        filter,
        is_created_buy,
    );

    let mut events =
        crate::grpc::log_instr_dedup::dedupe_log_instruction_events(log_events, instr_events);
    crate::grpc::transaction_meta::fill_recent_blockhash(&mut events, &info.transaction);
    for event in &mut events {
        crate::core::common_filler::fill_token_balances(event, meta, &info.transaction);
    }
    if let Some(filter) = filter {
        events.into_iter().map(|e| filter.normalize_dex_event(e)).collect()
    } else {
        events
    }
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
    is_created_buy: bool,
) -> Vec<DexEvent> {
    let mut outer_idx: i32 = -1;
    let mut inner_idx: i32 = -1;
    let mut invokes = crate::core::invoke_context::InvokeContext::default();
    let mut active_program_stack: SmallVec<[ActiveProgram<'_>; 8]> = SmallVec::new();
    let mut result = Vec::with_capacity(4);

    for log in logs {
        if log.as_bytes().starts_with(PROGRAM_DATA_PREFIX) || log.contains("ray_log: ") {
            let current_program = active_program_stack.last().map(|active| &active.pubkey);
            if let Some(mut e) = crate::logs::parse_log_with_program_id(
                log,
                sig,
                slot,
                tx_idx,
                block_us,
                grpc_us,
                filter,
                is_created_buy,
                None,
                current_program,
            ) {
                if matches!(&e, DexEvent::RaydiumAmmV4Swap(_)) {
                    let mut scoped = crate::core::invoke_context::InvokeContext::default();
                    if let Some(active) = active_program_stack.last() {
                        scoped.push(active.pubkey, active.position);
                    }
                    crate::core::account_dispatcher::fill_accounts_with_invoke_context(
                        &mut e,
                        meta,
                        transaction,
                        &scoped,
                    );
                } else {
                    crate::core::account_dispatcher::fill_accounts_with_invoke_context(
                        &mut e,
                        meta,
                        transaction,
                        &invokes,
                    );
                }
                crate::core::common_filler::fill_data_with_invoke_context(
                    &mut e,
                    meta,
                    transaction,
                    &invokes,
                );
                result.push(e);
            }
            continue;
        }

        if let Some((pid, depth)) = crate::logs::optimized_matcher::parse_invoke_info(log) {
            if depth == 1 {
                inner_idx = -1;
                outer_idx = next_logged_outer_index(transaction, outer_idx);
            } else {
                inner_idx += 1;
            }
            let pk = crate::grpc::program_ids::known_program_id(pid).unwrap_or_default();
            active_program_stack.truncate(depth - 1);
            active_program_stack.push(ActiveProgram {
                encoded: pid,
                pubkey: pk,
                position: (outer_idx, inner_idx),
            });
            if crate::grpc::program_ids::needs_invoke_context(&pk) {
                invokes.push(pk, (outer_idx, inner_idx));
            }
            continue;
        }

        if let Some(pid) = crate::logs::optimized_matcher::parse_program_complete_info(log) {
            if let Some(pos) = active_program_stack.iter().rposition(|active| active.encoded == pid)
            {
                active_program_stack.truncate(pos);
            }
        }
    }
    result
}

/// Next outer instruction index that emits an `invoke [1]` log line.
#[inline]
pub(crate) fn next_logged_outer_index(transaction: &Option<Transaction>, current: i32) -> i32 {
    let mut next = current + 1;
    while is_logless_outer(transaction, next) {
        next += 1;
    }
    next
}

#[inline]
fn is_logless_outer(transaction: &Option<Transaction>, outer_idx: i32) -> bool {
    let Some(msg) = transaction.as_ref().and_then(|tx| tx.message.as_ref()) else {
        return false;
    };
    usize::try_from(outer_idx)
        .ok()
        .and_then(|idx| msg.instructions.get(idx))
        .and_then(|ix| msg.account_keys.get(ix.program_id_index as usize))
        .is_some_and(|key| {
            crate::grpc::program_ids::LOGLESS_PRECOMPILES
                .iter()
                .any(|precompile| precompile.as_ref() == key.as_slice())
        })
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
    is_created_buy: bool,
) -> Vec<DexEvent> {
    crate::grpc::instruction_parser::parse_instructions_enhanced_with_created_buy(
        meta,
        transaction,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        filter,
        is_created_buy,
    )
}

#[cfg(test)]
mod logless_outer_tests {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};

    #[test]
    fn log_outer_index_skips_precompiles() {
        let program = Pubkey::new_unique();
        for precompile in crate::grpc::program_ids::LOGLESS_PRECOMPILES {
            let account_keys =
                [program, precompile].iter().map(|k| k.to_bytes().to_vec()).collect();
            let ix = |program_id_index: u32| CompiledInstruction {
                program_id_index,
                accounts: Vec::new(),
                data: Vec::new(),
            };
            let transaction = Some(Transaction {
                signatures: vec![vec![0u8; 64]],
                message: Some(Message {
                    account_keys,
                    instructions: vec![ix(0), ix(1), ix(1), ix(0)],
                    ..Default::default()
                }),
            });
            assert_eq!(next_logged_outer_index(&transaction, -1), 0);
            assert_eq!(next_logged_outer_index(&transaction, 0), 3);
            assert_eq!(next_logged_outer_index(&None, 0), 1);
            let mut leading = transaction.clone();
            leading.as_mut().unwrap().message.as_mut().unwrap().instructions =
                vec![ix(1), ix(1), ix(0)];
            assert_eq!(next_logged_outer_index(&leading, -1), 2);
        }
    }
}
