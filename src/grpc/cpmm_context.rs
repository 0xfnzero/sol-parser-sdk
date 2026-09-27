//! Conservative CPMM log-to-instruction binding. Pool/occurrence matching is
//! insufficient when a transaction repeats a pool or loses a log. Validate the
//! complete invocation trace against compiled outer/inner positions first.
use crate::core::events::{
    CpmmSwapAmountsSource, DexEvent, InstructionIndex, RaydiumCpmmSwapEvent,
};
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, str::FromStr};
use yellowstone_grpc_proto::prelude::{Message, Transaction, TransactionStatusMeta};

pub(crate) fn fill_transaction_status(event: &mut DexEvent, meta: &TransactionStatusMeta) {
    match event {
        DexEvent::RaydiumCpmmSwap(swap) => swap.transaction_success = Some(meta.err.is_none()),
        DexEvent::RaydiumLaunchlabMigrateCpmm(migration) => {
            migration.transaction_success = Some(meta.err.is_none())
        }
        _ => {}
    }
}

fn key(message: &Message, meta: &TransactionStatusMeta, index: usize) -> Option<Pubkey> {
    let bytes = message
        .account_keys
        .iter()
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .nth(index)?;
    Pubkey::try_from(bytes.as_slice()).ok()
}

pub(crate) struct InvocationTrace {
    pub log_indices: HashMap<usize, InstructionIndex>,
    outcomes: HashMap<InstructionIndex, bool>,
}

pub(crate) fn fill_migration_outcomes(
    events: &mut [DexEvent],
    transaction: &Option<Transaction>,
    meta: &TransactionStatusMeta,
    logs: &[String],
) {
    if !events.iter().any(|event| matches!(event, DexEvent::RaydiumLaunchlabMigrateCpmm(_))) {
        return;
    }
    let Some(trace) = log_instruction_indices(transaction, meta, logs) else {
        return;
    };
    for event in events {
        if let DexEvent::RaydiumLaunchlabMigrateCpmm(migration) = event {
            migration.invocation_success =
                migration.instruction_index.and_then(|index| trace.outcomes.get(&index).copied());
        }
    }
}

/// No partial maps: missing invokes/completions, heights, malformed keys, or
/// truncated logs leave every log unbound. This intentionally favors safety.
pub(crate) fn log_instruction_indices(
    transaction: &Option<Transaction>,
    meta: &TransactionStatusMeta,
    logs: &[String],
) -> Option<InvocationTrace> {
    let message = transaction.as_ref()?.message.as_ref()?;
    let mut expected = Vec::new();
    for (outer, ix) in message.instructions.iter().enumerate() {
        let outer = u32::try_from(outer).ok()?;
        expected.push((
            key(message, meta, ix.program_id_index as usize)?,
            1,
            InstructionIndex { outer, inner: None },
        ));
        let mut groups = meta.inner_instructions.iter().filter(|g| g.index == outer);
        if let Some(group) = groups.next() {
            if groups.next().is_some() {
                return None;
            }
            for (inner, ix) in group.instructions.iter().enumerate() {
                expected.push((
                    key(message, meta, ix.program_id_index as usize)?,
                    usize::try_from(ix.stack_height?).ok()?,
                    InstructionIndex { outer, inner: Some(u32::try_from(inner).ok()?) },
                ));
            }
        }
    }
    if meta.inner_instructions.iter().any(|g| g.index as usize >= message.instructions.len()) {
        return None;
    }
    let mut next = 0;
    let mut stack = Vec::new();
    let mut result = HashMap::new();
    let mut outcomes = HashMap::new();
    for (line, log) in logs.iter().enumerate() {
        if let Some((program, depth)) = crate::logs::optimized_matcher::parse_invoke_info(log) {
            let program = Pubkey::from_str(program).ok()?;
            let &(expected_program, expected_depth, index) = expected.get(next)?;
            if program != expected_program || depth != expected_depth || depth != stack.len() + 1 {
                return None;
            }
            stack.push((program, index, next));
            next += 1;
        } else if let Some(program) =
            crate::logs::optimized_matcher::parse_program_complete_info(log)
        {
            let (active_program, _, start) = stack.pop()?;
            if active_program != Pubkey::from_str(program).ok()? {
                return None;
            }
            // A caught failure rolls back successful child invocations as well.
            let success = log.ends_with(" success");
            for &(_, _, index) in &expected[start..next] {
                *outcomes.entry(index).or_insert(true) &= success;
            }
        } else if log.starts_with("Program data: ") {
            if let Some((program, index, _)) = stack.last() {
                if *program == crate::instr::raydium_cpmm::PROGRAM_ID_PUBKEY {
                    result.insert(line, *index);
                }
            }
        } else if log.contains("Log truncated") {
            return None;
        }
    }
    // Multiple data records within one CPMM invocation are ambiguous. Do not
    // authorize two copy fills (or guess which record is the swap).
    let mut counts = HashMap::new();
    for index in result.values() {
        *counts.entry(*index).or_insert(0usize) += 1;
    }
    result.retain(|_, index| counts.get(index) == Some(&1) && outcomes.get(index) == Some(&true));
    (next == expected.len() && stack.is_empty())
        .then_some(InvocationTrace { log_indices: result, outcomes })
}

pub(crate) fn bind_log(
    event: &mut RaydiumCpmmSwapEvent,
    index: InstructionIndex,
    transaction: &Option<Transaction>,
    meta: &TransactionStatusMeta,
) -> Option<()> {
    if event.amounts_source != CpmmSwapAmountsSource::SwapEvent {
        return None;
    }
    let message = transaction.as_ref()?.message.as_ref()?;
    let (program, data, indices) = if let Some(inner) = index.inner {
        let ix = meta
            .inner_instructions
            .iter()
            .find(|g| g.index == index.outer)?
            .instructions
            .get(inner as usize)?;
        (ix.program_id_index, &ix.data, &ix.accounts)
    } else {
        let ix = message.instructions.get(index.outer as usize)?;
        (ix.program_id_index, &ix.data, &ix.accounts)
    };
    if key(message, meta, program as usize)? != crate::instr::raydium_cpmm::PROGRAM_ID_PUBKEY {
        return None;
    }
    let accounts: Vec<_> =
        indices.iter().map(|&i| key(message, meta, i as usize)).collect::<Option<_>>()?;
    let DexEvent::RaydiumCpmmSwap(instruction) = crate::instr::raydium_cpmm::parse_instruction(
        data,
        &accounts,
        event.metadata.signature,
        event.metadata.slot,
        event.metadata.tx_index,
        Some(event.metadata.block_time_us),
    )?
    else {
        return None;
    };
    if event.pool_id == Pubkey::default()
        || event.pool_id != instruction.pool_id
        || event.base_input != instruction.base_input
    {
        return None;
    }
    if let Some(mints) = event.log_mints {
        if accounts.get(10) != Some(&mints.input)
            || accounts.get(11) != Some(&mints.output)
            || mints.input == Pubkey::default()
            || mints.output == Pubkey::default()
        {
            return None;
        }
    }
    event.instruction_index = Some(index);
    event.context = instruction.context;
    Some(())
}

pub(super) fn set_instruction_index(event: &mut DexEvent, outer: usize, inner: Option<usize>) {
    let Ok(outer) = u32::try_from(outer) else {
        return;
    };
    let inner = match inner {
        Some(i) => match u32::try_from(i) {
            Ok(i) => Some(i),
            Err(_) => return,
        },
        None => None,
    };
    if let DexEvent::RaydiumLaunchlabMigrateCpmm(migration) = event {
        migration.instruction_index = Some(InstructionIndex { outer, inner });
    }
    if let DexEvent::RaydiumCpmmSwap(swap) = event {
        // Old event-CPI payloads are not ordinary swap instructions.
        if swap.amounts_source == CpmmSwapAmountsSource::InstructionOnly {
            swap.instruction_index = Some(InstructionIndex { outer, inner });
        }
    }
}
