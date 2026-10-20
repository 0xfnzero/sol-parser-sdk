//! Explicitly opted-in, offline-validated wrapper rules. No RPC, clocks, or I/O.
//! A rule describes an observed input layout, not proof of future CPI execution.
use crate::core::events::{DexEvent, RawInstructionEvent};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Compatibility name for the raw outer instruction payload. Rule identifiers
/// belong to the caller's rule registry, not the transaction's provenance.
pub type ThirdProgram = crate::core::events::RawInstructionPayload;

/// Candidate rules must be reviewed and tested on independent transactions
/// before the caller opts in. Never install analyzer output automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedPumpfunRule {
    pub rule_id: String,
    pub program_id: Pubkey,
    pub outer_account_count: usize,
    /// Exact byte guards; None denotes a copied argument byte.
    pub data_pattern: Vec<Option<u8>>,
    pub pump_discriminator: [u8; 8],
    /// One outer offset for each u64 Pump argument, in IDL order.
    pub argument_offsets: [usize; 2],
    /// Pump account index -> outer account index, retaining duplicates.
    pub account_map: Vec<usize>,
    pub fixed_accounts: Vec<(usize, Pubkey)>,
    /// Aliases observed during inference must stay equal, otherwise the inferred
    /// mapping is ambiguous (e.g. base/quote token programs happened to match).
    pub equal_accounts: Vec<(usize, usize)>,
}

/// Narrow initial scope: fixed two-u64 v2 trades. Multi-hop and runtime-derived
/// amounts are deliberately unsupported. Expand only with IDL-backed tests.
pub fn pump_trade_name(disc: &[u8]) -> Option<&'static str> {
    use crate::instr::pump::discriminators::*;
    match disc {
        d if d == BUY_V2 => Some("buy_v2"),
        d if d == BUY_EXACT_QUOTE_IN_V2 => Some("buy_exact_quote_in_v2"),
        d if d == SELL_V2 => Some("sell_v2"),
        _ => None,
    }
}

/// Pure reconstruction. Missing ALT accounts, changed bytes or changed account
/// constraints return None; the caller retains the original RawInstruction.
pub fn restore_instruction(
    raw: &RawInstructionEvent,
    rule: &WrappedPumpfunRule,
) -> Option<([u8; 24], Vec<Pubkey>)> {
    if raw.program_id != rule.program_id
        || raw.accounts.len() != rule.outer_account_count
        || raw.instruction_data.len() != rule.data_pattern.len()
        || rule.data_pattern.is_empty()
        || rule.data_pattern.iter().all(Option::is_none)
        || pump_trade_name(&rule.pump_discriminator).is_none()
        || raw
            .instruction_data
            .iter()
            .zip(&rule.data_pattern)
            .any(|(actual, expected)| expected.is_some_and(|v| v != *actual))
    {
        return None;
    }
    if raw.accounts.iter().any(Option::is_none) {
        return None;
    }
    let accounts = &raw.accounts;
    for &(index, key) in &rule.fixed_accounts {
        if accounts.get(index) != Some(&Some(key)) {
            return None;
        }
    }
    for &(a, b) in &rule.equal_accounts {
        if accounts.get(a).is_none() || accounts.get(a) != accounts.get(b) {
            return None;
        }
    }
    let mut data = [0u8; 24];
    data[..8].copy_from_slice(&rule.pump_discriminator);
    for (i, offset) in rule.argument_offsets.iter().copied().enumerate() {
        data[8 + i * 8..16 + i * 8]
            .copy_from_slice(raw.instruction_data.get(offset..offset.checked_add(8)?)?);
    }
    let mapped = rule
        .account_map
        .iter()
        .map(|i| accounts.get(*i).copied().flatten())
        .collect::<Option<Vec<_>>>()?;
    Some((data, mapped))
}

/// Use from a Shred RawInstruction callback with a caller-owned rule registry.
/// Returns PumpFunTrade with input parameters, never fabricated executed amounts.
/// Existing gRPC parsing does not call this function.
pub fn parse_wrapped_pumpfun_instruction(
    raw: &RawInstructionEvent,
    rule: &WrappedPumpfunRule,
    filter: Option<&crate::grpc::EventTypeFilter>,
) -> Option<DexEvent> {
    if filter.is_some_and(|f| !f.includes_pumpfun()) {
        return None;
    }
    let (data, accounts) = restore_instruction(raw, rule)?;
    let m = &raw.metadata;
    let event = crate::instr::pump::parse_instruction(
        &data,
        &accounts,
        m.signature,
        m.slot,
        m.tx_index,
        Some(m.block_time_us),
        m.grpc_recv_us,
    )?;
    let mut trade = match event {
        DexEvent::PumpFunTrade(t)
        | DexEvent::PumpFunBuy(t)
        | DexEvent::PumpFunSell(t)
        | DexEvent::PumpFunBuyExactSolIn(t) => t,
        _ => return None,
    };
    trade.metadata = m.clone();
    trade.sol_amount = 0;
    trade.quote_amount = 0;
    trade.token_amount = 0;
    trade.third_program = Some(raw.into());
    trade.parse_status = crate::core::events::PumpFunParseStatus::InstructionDecoded;
    let event = DexEvent::PumpFunTrade(trade);
    filter.is_none_or(|f| f.should_include_dex_event(&event)).then_some(event)
}
