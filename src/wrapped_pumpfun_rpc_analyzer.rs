//! Offline RPC analysis only. This module is never called by streaming parsers.
//! Matching bytes are evidence for a candidate, not proof of a program's ABI.
use crate::core::events::{EventMetadata, RawInstructionEvent};
use crate::wrapped_pumpfun_parser::{pump_trade_name, WrappedPumpfunRule};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrappedPumpfunAnalysis {
    pub program_id: Pubkey,
    pub instruction_index: usize,
    pub pump_instruction: Option<String>,
    pub reason: String,
    pub candidate_rule: Option<WrappedPumpfunRule>,
}

/// Analyze an RPC transaction converted by convert_rpc_to_grpc. Runtime-derived
/// fields and ambiguous byte/account matches fail closed. No network access here;
/// the accompanying example owns the RPC request.
pub fn analyze_wrapped_pumpfun(
    transaction: &Transaction,
    meta: &TransactionStatusMeta,
) -> Vec<WrappedPumpfunAnalysis> {
    let Some(message) = &transaction.message else { return vec![] };
    if meta.err.is_some() {
        return vec![];
    }
    let keys: Vec<Option<Pubkey>> = message
        .account_keys
        .iter()
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .map(|k| Pubkey::try_from(k.as_slice()).ok())
        .collect();
    let get = |i: usize| keys.get(i).copied().flatten();
    let pump = crate::instr::pump::PROGRAM_ID_PUBKEY;
    let mut reports = vec![];
    for (index, outer) in message.instructions.iter().enumerate() {
        let Some(program_id) = get(outer.program_id_index as usize) else { continue };
        if !crate::core::events::raw_instruction_dexes(program_id, &[]).is_empty()
            || !outer.accounts.iter().any(|i| get(*i as usize) == Some(pump))
        {
            continue;
        }
        let trades: Vec<_> = meta
            .inner_instructions
            .iter()
            .filter(|g| g.index as usize == index)
            .flat_map(|g| &g.instructions)
            .filter(|ix| {
                get(ix.program_id_index as usize) == Some(pump)
                    && ix.data.get(..8).and_then(pump_trade_name).is_some()
            })
            .collect();
        let mut report = WrappedPumpfunAnalysis {
            program_id,
            instruction_index: index,
            pump_instruction: trades
                .first()
                .and_then(|ix| pump_trade_name(&ix.data[..8]))
                .map(str::to_owned),
            reason: String::new(),
            candidate_rule: None,
        };
        let other_dex = meta
            .inner_instructions
            .iter()
            .filter(|g| g.index as usize == index)
            .flat_map(|g| &g.instructions)
            .filter_map(|ix| get(ix.program_id_index as usize))
            .any(|pid| {
                pid != pump && !crate::core::events::raw_instruction_dexes(pid, &[]).is_empty()
            });
        if other_dex {
            report.reason = "multi_dex_route_not_supported".into();
            reports.push(report);
            continue;
        }
        if trades.len() != 1 {
            report.reason = if trades.is_empty() {
                "no_supported_pump_trade_cpi"
            } else {
                "multiple_pump_trade_cpis_not_supported"
            }
            .into();
            reports.push(report);
            continue;
        }
        let inner = trades[0];
        let raw = RawInstructionEvent {
            metadata: EventMetadata::default(),
            dexes: vec![],
            program_id,
            instruction_index: index,
            accounts: outer.accounts.iter().map(|i| get(*i as usize)).collect(),
            account_indices: outer.accounts.clone(),
            instruction_data: outer.data.clone(),
        };
        let mapped_accounts: Option<Vec<_>> =
            inner.accounts.iter().map(|i| get(*i as usize)).collect();
        match mapped_accounts.and_then(|accounts| infer_rule(&raw, &inner.data, &accounts)) {
            Some(rule) => {
                report.reason =
                    "candidate_only_validate_independent_transactions_before_use".into();
                report.candidate_rule = Some(rule);
            }
            None => report.reason = "cannot_match_unambiguous_input_parameters_or_accounts".into(),
        }
        reports.push(report);
    }
    reports
}

fn infer_rule(
    raw: &RawInstructionEvent,
    data: &[u8],
    inner_accounts: &[Pubkey],
) -> Option<WrappedPumpfunRule> {
    if data.len() != 24 || raw.instruction_data.len() < 17 {
        return None;
    }
    let accounts = raw.accounts.iter().copied().collect::<Option<Vec<_>>>()?;
    let mut offsets = [0; 2];
    let mut pattern: Vec<_> = raw.instruction_data.iter().copied().map(Some).collect();
    for i in 0..2 {
        let bytes = &data[8 + i * 8..16 + i * 8];
        // At least one opcode byte precedes arguments. Never infer literal
        // constants from the observed CPI or search at sub-field byte granularity.
        let candidates: Vec<_> = raw
            .instruction_data
            .windows(8)
            .enumerate()
            .filter(|(offset, window)| *offset > 0 && *window == bytes)
            .map(|(offset, _)| offset)
            .collect();
        if candidates.len() != 1 {
            return None;
        }
        offsets[i] = candidates[0];
        for b in &mut pattern[offsets[i]..offsets[i] + 8] {
            *b = None;
        }
    }
    if offsets[0].abs_diff(offsets[1]) < 8 {
        return None;
    }
    let map = inner_accounts
        .iter()
        .map(|key| accounts.iter().position(|a| a == key))
        .collect::<Option<Vec<_>>>()?;
    let mut aliases = vec![];
    for i in 0..accounts.len() {
        if let Some(j) = accounts[..i].iter().position(|a| *a == accounts[i]) {
            aliases.push((j, i));
        }
    }
    // Bind canonical program/sysvar/global identities; variable mints and wallets
    // remain variable. Aliases guard ambiguous mappings in the training sample.
    let fixed = accounts
        .iter()
        .enumerate()
        .filter(|(_, k)| {
            **k == crate::instr::pump::PROGRAM_ID_PUBKEY
                || **k == solana_sdk::pubkey!("11111111111111111111111111111111")
                || **k == solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                || **k == solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
                || **k == solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
                || **k == solana_sdk::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf")
                || **k == solana_sdk::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1")
                || **k == solana_sdk::pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ")
        })
        .map(|(i, k)| (i, *k))
        .collect();
    Some(WrappedPumpfunRule {
        rule_id: format!("{}:{}:candidate", raw.program_id, raw.instruction_index),
        program_id: raw.program_id,
        outer_account_count: accounts.len(),
        data_pattern: pattern,
        pump_discriminator: data[..8].try_into().ok()?,
        argument_offsets: offsets,
        account_map: map,
        fixed_accounts: fixed,
        equal_accounts: aliases,
    })
}
