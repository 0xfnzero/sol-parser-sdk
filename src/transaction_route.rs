//! Opt-in instruction-level route inspection for RPC and Yellowstone transactions.
//!
//! Keeps execution order, CPI positions, transfers, and unknown programs. A list
//! of swaps is not assumed to be a linear route. Graduated StonkFun attribution
//! requires pool identities established by the caller (e.g. from migration).
use crate::{convert_rpc_to_grpc, ParseError};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

// Force base58 literals into const evaluation, including in builds whose
// dependency optimization settings do not inline Address::from_str_const.
const ROUTE_SPL_TOKEN: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ROUTE_TOKEN_2022: Pubkey = solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const ROUTE_WSOL: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
const ROUTE_COMPUTE_BUDGET: Pubkey =
    solana_sdk::pubkey!("ComputeBudget111111111111111111111111111111");
const ROUTE_ASSOCIATED_TOKEN: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const ROUTE_MEMO: Pubkey = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionPosition {
    pub outer_index: u32,
    pub inner_index: Option<u32>,
    pub stack_height: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapProtocol {
    PumpSwap,
    LaunchLab,
    RaydiumCpmm,
    RaydiumAmmV4,
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDlmm,
    PumpFun,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteSwapLeg {
    pub position: InstructionPosition,
    pub program: Pubkey,
    pub protocol: SwapProtocol,
    pub pool: Pubkey,
    pub trader: Pubkey,
    pub input_account: Pubkey,
    pub output_account: Pubkey,
    pub input_mint: Option<Pubkey>,
    pub output_mint: Option<Pubkey>,
    /// Instruction arguments, not executed amounts or a fresh quote.
    pub amount_specified_is_input: bool,
    pub specified_amount: u64,
    pub other_amount_threshold: u64,
    /// Actual debits/credits observed in this invocation's token transfers.
    /// None when execution failed, transfer context is missing, or CPI depth is unknown.
    pub actual_input_amount: Option<u64>,
    pub actual_output_amount: Option<u64>,
    pub stonkfun_mode: Option<crate::core::events::StonkFunMode>,
    /// Caller supplied this CPMM pool as a verified graduated StonkFun pool.
    pub stonkfun_graduated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteTokenTransfer {
    pub position: InstructionPosition,
    pub program: Pubkey,
    pub source: Pubkey,
    pub destination: Pubkey,
    pub mint: Option<Pubkey>,
    /// Gross transfer argument. Token-2022 transfer fees may reduce the credit.
    pub amount: u64,
    pub withheld_fee: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteUnknownInvocation {
    pub position: InstructionPosition,
    pub program: Pubkey,
    /// Nested token transfers identify opaque/custom liquidity or router activity.
    pub has_token_transfers: bool,
    /// True for opaque routers wrapping decoded swap legs. False plus token
    /// transfers marks an unresolved economic operation (possibly a custom pool).
    pub has_known_swap_descendants: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionRoute {
    pub signature: Signature,
    pub succeeded: bool,
    pub legs: Vec<RouteSwapLeg>,
    pub transfers: Vec<RouteTokenTransfer>,
    /// Instruction evidence only. Failed transactions do not change balances,
    /// and WSOL usage alone never establishes the caller's desired SOL/WSOL asset.
    #[serde(default)]
    pub native_token_actions: Vec<RouteNativeTokenAction>,
    /// Preserved even when a program cannot be decoded. Do not treat legs as
    /// complete coverage when unknown invocations carry token transfers.
    pub unknown_invocations: Vec<RouteUnknownInvocation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NativeTokenAction {
    /// System transfer to an account identified as WSOL in this transaction.
    Fund {
        source: Pubkey,
        lamports: u64,
    },
    SyncNative,
    /// Closing a WSOL account returns its lamports (including rent). This is
    /// not an output amount quote and the destination need not be the trader.
    Close {
        destination: Pubkey,
        authority: Pubkey,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteNativeTokenAction {
    pub position: InstructionPosition,
    pub account: Pubkey,
    pub action: NativeTokenAction,
}

fn native_action(
    ix: &Invocation<'_>,
    keys: &[Pubkey],
    mints: &HashMap<Pubkey, Pubkey>,
) -> Option<RouteNativeTokenAction> {
    if !ix.program_resolved {
        return None;
    }
    let wsol = ROUTE_WSOL;
    let (target, action) = if ix.program == Pubkey::default()
        && ix.data.get(..4) == Some(&2u32.to_le_bytes())
        && ix.data.len() == 12
        && ix.accounts.len() >= 2
    {
        (
            account(ix, keys, 1),
            NativeTokenAction::Fund { source: account(ix, keys, 0), lamports: u64_at(ix.data, 4)? },
        )
    } else if token_program(ix.program) && ix.data == [17] && !ix.accounts.is_empty() {
        (account(ix, keys, 0), NativeTokenAction::SyncNative)
    } else if token_program(ix.program) && ix.data == [9] && ix.accounts.len() >= 3 {
        (
            account(ix, keys, 0),
            NativeTokenAction::Close {
                destination: account(ix, keys, 1),
                authority: account(ix, keys, 2),
            },
        )
    } else {
        return None;
    };
    if ix.accounts.iter().any(|&index| index as usize >= keys.len())
        || mints.get(&target) != Some(&wsol)
    {
        return None;
    }
    Some(RouteNativeTokenAction { position: ix.position, account: target, action })
}

struct Invocation<'a> {
    position: InstructionPosition,
    program: Pubkey,
    program_resolved: bool,
    accounts: &'a [u8],
    data: &'a [u8],
}

fn key(keys: &[Pubkey], index: u32) -> Pubkey {
    keys.get(index as usize).copied().unwrap_or_default()
}
fn account(ix: &Invocation<'_>, keys: &[Pubkey], index: usize) -> Pubkey {
    ix.accounts.get(index).map(|i| key(keys, u32::from(*i))).unwrap_or_default()
}
fn u64_at(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(data.get(offset..offset + 8)?.try_into().ok()?))
}
fn token_program(program: Pubkey) -> bool {
    program == ROUTE_SPL_TOKEN || program == ROUTE_TOKEN_2022
}
fn checked_transfer(ix: &Invocation<'_>) -> bool {
    token_program(ix.program)
        && ix.data.first() == Some(&12)
        && ix.data.len() >= 10
        && ix.accounts.len() >= 4
}
fn checked_transfer_with_fee(ix: &Invocation<'_>) -> bool {
    ix.program == ROUTE_TOKEN_2022
        && ix.data.get(..2) == Some(&[26, 1])
        && ix.data.len() >= 19
        && ix.accounts.len() >= 4
}
fn descendant(parent: InstructionPosition, child: InstructionPosition) -> bool {
    if parent.outer_index != child.outer_index {
        return false;
    }
    match (parent.inner_index, child.inner_index) {
        (None, Some(_)) => true,
        (Some(a), Some(b)) if b > a => match (parent.stack_height, child.stack_height) {
            (Some(a), Some(b)) => b > a,
            _ => false,
        },
        _ => false,
    }
}
#[cfg(test)]
fn descendants<'a>(index: usize, invocations: &'a [Invocation<'_>]) -> &'a [Invocation<'a>] {
    let parent = invocations[index].position;
    let end = (index + 1..invocations.len())
        .find(|&i| !descendant(parent, invocations[i].position))
        .unwrap_or(invocations.len());
    &invocations[index + 1..end]
}

// Route decisions and transfer decoding are computed once. Reverse subtree
// jumps reuse child boundaries and flags instead of rescanning each ancestor.
struct InvocationAnalysis {
    transfer: Option<RouteTokenTransfer>,
    swap_index: Option<usize>,
    descendant_end: usize,
    has_token_transfers: bool,
    has_known_swap_descendants: bool,
    skip_route: bool,
}

fn invocation_analysis(
    invocations: &[Invocation<'_>],
    keys: &[Pubkey],
    mints: &HashMap<Pubkey, Pubkey>,
    graduated: &[Pubkey],
    legs: &mut Vec<RouteSwapLeg>,
) -> Vec<InvocationAnalysis> {
    let mut analysis: Vec<_> = invocations
        .iter()
        .enumerate()
        .map(|(i, ix)| {
            // Unresolved indexes use the default-key sentinel; they are not SystemProgram.
            let skip_route = ix.program_resolved
                && (token_program(ix.program)
                    || ix.program == Pubkey::default()
                    || ix.program == ROUTE_COMPUTE_BUDGET
                    || ix.program == ROUTE_ASSOCIATED_TOKEN
                    || ix.program == ROUTE_MEMO);
            let swap_index = if skip_route {
                None
            } else {
                swap(ix, keys, mints, graduated).map(|leg| {
                    let index = legs.len();
                    legs.push(leg);
                    index
                })
            };
            InvocationAnalysis {
                transfer: transfer(ix, keys, mints),
                swap_index,
                descendant_end: i + 1,
                has_token_transfers: false,
                has_known_swap_descendants: false,
                skip_route,
            }
        })
        .collect();
    for i in (0..invocations.len()).rev() {
        let mut end = i + 1;
        let mut has_transfers = false;
        let mut has_swaps = false;
        while end < invocations.len()
            && descendant(invocations[i].position, invocations[end].position)
        {
            let child = &analysis[end];
            has_transfers |= child.transfer.is_some() || child.has_token_transfers;
            has_swaps |= child.swap_index.is_some() || child.has_known_swap_descendants;
            end = child.descendant_end;
        }
        analysis[i].descendant_end = end;
        analysis[i].has_token_transfers = has_transfers;
        analysis[i].has_known_swap_descendants = has_swaps;
    }
    analysis
}

/// Resolve mints from pre/post balances and checked transfers, including accounts
/// created and closed within the transaction. Plain transfers propagate only
/// already known mints; an arbitrary account key is never treated as a mint.
#[cfg(test)]
fn transaction_token_mints(
    transaction: &Transaction,
    meta: &TransactionStatusMeta,
) -> HashMap<Pubkey, Pubkey> {
    let keys = transaction_keys(transaction, meta);
    let invocations = transaction_invocations(transaction, meta, &keys);
    token_mints_from_invocations(meta, &keys, &invocations)
}

fn token_mints_from_invocations(
    meta: &TransactionStatusMeta,
    keys: &[Pubkey],
    invocations: &[Invocation<'_>],
) -> HashMap<Pubkey, Pubkey> {
    let mut mints = HashMap::new();
    let record = |mints: &mut HashMap<Pubkey, Pubkey>, account, mint| {
        // Default is the missing-account sentinel, not evidence of a token mint.
        if account != Pubkey::default() && mint != Pubkey::default() {
            use std::collections::hash_map::Entry;
            match mints.entry(account) {
                Entry::Vacant(entry) => {
                    entry.insert(mint);
                }
                Entry::Occupied(mut entry) if *entry.get() != mint => {
                    // Retain an ambiguity marker through propagation. A closed
                    // and reused account can have different mints in one tx.
                    entry.insert(Pubkey::default());
                }
                _ => {}
            }
        }
    };
    for balance in meta.pre_token_balances.iter().chain(&meta.post_token_balances) {
        if let Ok(mint) = balance.mint.parse() {
            record(&mut mints, key(&keys, balance.account_index), mint);
        }
    }
    for ix in invocations {
        if checked_transfer(ix) {
            let mint = account(ix, &keys, 1);
            record(&mut mints, account(ix, &keys, 0), mint);
            record(&mut mints, account(ix, &keys, 2), mint);
        }
        // Token-2022 TransferFeeExtension::TransferCheckedWithFee.
        if checked_transfer_with_fee(ix) {
            let mint = account(ix, &keys, 1);
            record(&mut mints, account(ix, &keys, 0), mint);
            record(&mut mints, account(ix, &keys, 2), mint);
        }
        // InitializeAccount / InitializeAccount2 / InitializeAccount3 also
        // identify ephemeral accounts without pre/post token balances.
        if token_program(ix.program)
            && ix.accounts.len() >= 2
            && matches!((ix.data.first(), ix.data.len()), (Some(1), 1) | (Some(16 | 18), 33))
        {
            record(&mut mints, account(ix, &keys, 0), account(ix, &keys, 1));
        }
    }
    if mints.is_empty() {
        return mints;
    }
    // Fixed point rather than a fixed number of hops; each pass adds information.
    loop {
        let before = mints.len();
        for ix in invocations {
            if token_program(ix.program)
                && ix.data.first() == Some(&3)
                && ix.data.len() >= 9
                && ix.accounts.len() >= 3
            {
                let source = account(ix, &keys, 0);
                let destination = account(ix, &keys, 1);
                if source == Pubkey::default() || destination == Pubkey::default() {
                    continue;
                }
                let source_mint = mints.get(&source).copied();
                let destination_mint = mints.get(&destination).copied();
                if source_mint == Some(Pubkey::default())
                    || destination_mint == Some(Pubkey::default())
                    || matches!((source_mint, destination_mint), (Some(a), Some(b)) if a != b)
                {
                    continue;
                }
                if let Some(mint) = source_mint.or(destination_mint) {
                    mints.entry(source).or_insert(mint);
                    mints.entry(destination).or_insert(mint);
                }
            }
        }
        if mints.len() == before {
            break;
        }
    }
    mints.retain(|_, mint| *mint != Pubkey::default());
    mints
}
fn transaction_keys(transaction: &Transaction, meta: &TransactionStatusMeta) -> Vec<Pubkey> {
    transaction
        .message
        .iter()
        .flat_map(|m| &m.account_keys)
        .chain(&meta.loaded_writable_addresses)
        .chain(&meta.loaded_readonly_addresses)
        .map(|bytes| crate::instr::read_pubkey_fast(bytes))
        .collect()
}
fn transaction_invocations<'a>(
    transaction: &'a Transaction,
    meta: &'a TransactionStatusMeta,
    keys: &[Pubkey],
) -> Vec<Invocation<'a>> {
    let Some(message) = &transaction.message else {
        return Vec::new();
    };
    if message.instructions.is_empty() {
        return Vec::new();
    }
    let count = message.instructions.len()
        + meta
            .inner_instructions
            .iter()
            .filter(|group| (group.index as usize) < message.instructions.len())
            .map(|group| group.instructions.len())
            .sum::<usize>();
    // Protobuf bytes are not fixed-size Pubkeys. Keep malformed keys unresolved.
    // Direct indexes preserve static / writable ALT / readonly ALT order without allocation.
    let resolve_program = |index: u32| {
        let index = index as usize;
        let bytes = message.account_keys.get(index).or_else(|| {
            let index = index.checked_sub(message.account_keys.len())?;
            meta.loaded_writable_addresses.get(index).or_else(|| {
                meta.loaded_readonly_addresses
                    .get(index.checked_sub(meta.loaded_writable_addresses.len())?)
            })
        })?;
        if bytes.len() != 32 {
            return None;
        }
        keys.get(index).copied()
    };
    let mut result = Vec::with_capacity(count);
    // Canonical metadata is already sorted. Only unordered input needs an index;
    // original ordinal breaks ties so duplicate groups retain their old order.
    let order = if meta.inner_instructions.windows(2).all(|pair| pair[0].index <= pair[1].index) {
        None
    } else {
        let mut order: Vec<_> = (0..meta.inner_instructions.len()).collect();
        order.sort_unstable_by_key(|&i| (meta.inner_instructions[i].index, i));
        Some(order)
    };
    let group_at = |i: usize| {
        let index = if let Some(order) = &order { *order.get(i)? } else { i };
        meta.inner_instructions.get(index)
    };
    let mut cursor = 0;
    for (i, ix) in message.instructions.iter().enumerate() {
        let program = resolve_program(ix.program_id_index);
        result.push(Invocation {
            position: InstructionPosition {
                outer_index: i as u32,
                inner_index: None,
                stack_height: Some(1),
            },
            program: program.unwrap_or_default(),
            program_resolved: program.is_some(),
            accounts: &ix.accounts,
            data: &ix.data,
        });
        while let Some(group) = group_at(cursor) {
            if group.index > i as u32 {
                break;
            }
            cursor += 1;
            if group.index != i as u32 {
                continue;
            }
            for (j, ix) in group.instructions.iter().enumerate() {
                let program = resolve_program(ix.program_id_index);
                result.push(Invocation {
                    position: InstructionPosition {
                        outer_index: i as u32,
                        inner_index: Some(j as u32),
                        stack_height: ix.stack_height,
                    },
                    program: program.unwrap_or_default(),
                    program_resolved: program.is_some(),
                    accounts: &ix.accounts,
                    data: &ix.data,
                });
            }
        }
    }
    result
}
fn transfer(
    ix: &Invocation<'_>,
    keys: &[Pubkey],
    mints: &HashMap<Pubkey, Pubkey>,
) -> Option<RouteTokenTransfer> {
    if !token_program(ix.program) {
        return None;
    }
    let (destination_index, amount_offset, fee) = match ix.data.first()? {
        3 if ix.accounts.len() >= 3 && ix.data.len() >= 9 => {
            (1, 1, if ix.program == ROUTE_SPL_TOKEN { Some(0) } else { None })
        }
        12 if checked_transfer(ix) => {
            (2, 1, if ix.program == ROUTE_SPL_TOKEN { Some(0) } else { None })
        }
        26 if checked_transfer_with_fee(ix) => (2, 2, Some(u64_at(ix.data, 11)?)),
        _ => return None,
    };
    let source = account(ix, keys, 0);
    let destination = account(ix, keys, destination_index);
    if source == Pubkey::default()
        || destination == Pubkey::default()
        || (destination_index == 2 && account(ix, keys, 1) == Pubkey::default())
    {
        return None;
    }
    Some(RouteTokenTransfer {
        position: ix.position,
        program: ix.program,
        source,
        destination,
        mint: mints.get(&source).copied(),
        amount: u64_at(ix.data, amount_offset)?,
        withheld_fee: fee,
    })
}

fn swap(
    ix: &Invocation<'_>,
    keys: &[Pubkey],
    mints: &HashMap<Pubkey, Pubkey>,
    graduated: &[Pubkey],
) -> Option<RouteSwapLeg> {
    use crate::instr::program_ids::*;
    let a = |i| account(ix, keys, i);
    let disc = ix.data.get(..8);
    let swap_disc = Some(&[248, 198, 158, 145, 225, 117, 135, 200][..]);
    let swap_v2 = Some(&[43, 4, 237, 11, 26, 201, 30, 98][..]);
    let mut mode = None;
    let (protocol, pool, trader, input, output, exact_in, amount, threshold) = if ix.program
        == RAYDIUM_CLMM_PROGRAM_ID
        && (disc == swap_disc || disc == swap_v2)
        && ix.accounts.len() >= if disc == swap_v2 { 13 } else { 10 }
    {
        (
            SwapProtocol::RaydiumClmm,
            a(2),
            a(0),
            a(3),
            a(4),
            crate::instr::utils::read_option_bool_idl(ix.data, 40)?,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == ORCA_WHIRLPOOL_PROGRAM_ID && (disc == swap_disc || disc == swap_v2) {
        let v2 = disc == swap_v2;
        let direction = crate::instr::utils::read_option_bool_idl(ix.data, 41)?;
        let (input, output) = if v2 {
            if ix.accounts.len() < 15 {
                return None;
            }
            (a(if direction { 7 } else { 9 }), a(if direction { 9 } else { 7 }))
        } else {
            if ix.accounts.len() < 11 {
                return None;
            }
            (a(if direction { 3 } else { 5 }), a(if direction { 5 } else { 3 }))
        };
        (
            SwapProtocol::OrcaWhirlpool,
            a(if v2 { 4 } else { 2 }),
            a(if v2 { 3 } else { 1 }),
            input,
            output,
            crate::instr::utils::read_option_bool_idl(ix.data, 40)?,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == RAYDIUM_CPMM_PROGRAM_ID && ix.accounts.len() >= 13 {
        use crate::instr::raydium_cpmm::discriminators::*;
        let exact_in = if disc == Some(&SWAP_BASE_IN[..]) {
            true
        } else if disc == Some(&SWAP_BASE_OUT[..]) {
            false
        } else {
            return None;
        };
        let first = u64_at(ix.data, 8)?;
        let second = u64_at(ix.data, 16)?;
        (
            SwapProtocol::RaydiumCpmm,
            a(3),
            a(0),
            a(4),
            a(5),
            exact_in,
            if exact_in { first } else { second },
            if exact_in { second } else { first },
        )
    } else if ix.program == METEORA_DLMM_PROGRAM_ID {
        crate::instr::meteora_dlmm::validate_swap_layout(ix.data, ix.accounts.len())?;
        use crate::instr::meteora_dlmm::discriminators::*;
        let exact_in = if disc == Some(&SWAP[..]) || disc == Some(&SWAP2[..]) {
            true
        } else if disc == Some(&SWAP_EXACT_OUT[..]) || disc == Some(&SWAP_EXACT_OUT2[..]) {
            false
        } else {
            return None;
        };
        let first = u64_at(ix.data, 8)?;
        let second = u64_at(ix.data, 16)?;
        (
            SwapProtocol::MeteoraDlmm,
            a(0),
            a(10),
            a(4),
            a(5),
            exact_in,
            if exact_in { first } else { second },
            if exact_in { second } else { first },
        )
    } else if ix.program == RAYDIUM_LAUNCHLAB_PROGRAM_ID && ix.accounts.len() >= 18 {
        use crate::instr::raydium_launchlab::discriminators::*;
        let (buy, exact_in) = if disc == Some(&BUY_EXACT_IN[..]) {
            (true, true)
        } else if disc == Some(&SELL_EXACT_IN[..]) {
            (false, true)
        } else if disc == Some(&BUY_EXACT_OUT[..]) {
            (true, false)
        } else if disc == Some(&SELL_EXACT_OUT[..]) {
            (false, false)
        } else {
            return None;
        };
        mode = crate::core::events::stonkfun_mode_from_platform_config(a(3));
        (
            SwapProtocol::LaunchLab,
            a(4),
            a(0),
            a(if buy { 6 } else { 5 }),
            a(if buy { 5 } else { 6 }),
            exact_in,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == PUMPFUN_PROGRAM_ID && ix.accounts.len() == 17 {
        use crate::instr::pump::discriminators::*;
        let buy = disc != Some(&SELL_V3[..]);
        let exact = disc != Some(&BUY_V3[..]);
        if ![BUY_V3, BUY_EXACT_QUOTE_IN_V3, SELL_V3].iter().any(|d| disc == Some(&d[..])) {
            return None;
        }
        let quote = a(2);
        let source = if buy {
            if quote == ROUTE_WSOL {
                a(8)
            } else {
                a(10)
            }
        } else {
            a(9)
        };
        let dest = if buy {
            a(9)
        } else if quote == ROUTE_WSOL {
            a(8)
        } else {
            a(10)
        };
        (
            SwapProtocol::PumpFun,
            a(5),
            a(8),
            source,
            dest,
            exact,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == PUMPSWAP_PROGRAM_ID
        && (ix.accounts.len() >= 21 || ix.accounts.len() == 17)
    {
        use crate::instr::pump_amm::discriminators::*;
        let (buy, exact_in) =
            if disc == Some(&BUY_EXACT_QUOTE_IN[..]) || disc == Some(&BUY_EXACT_QUOTE_IN_V2[..]) {
                (true, true)
            } else if disc == Some(&BUY[..]) || disc == Some(&BUY_V2[..]) {
                (true, false)
            } else if disc == Some(&SELL[..]) || disc == Some(&SELL_V2[..]) {
                (false, true)
            } else {
                return None;
            };
        if buy && ix.data.len() > 24 {
            crate::instr::utils::read_option_bool_idl(ix.data, 24)?;
        }
        (
            SwapProtocol::PumpSwap,
            a(0),
            a(1),
            a(if buy { 6 } else { 5 }),
            a(if buy { 5 } else { 6 }),
            exact_in,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == RAYDIUM_AMM_V4_PROGRAM_ID
        && matches!(ix.data.first(), Some(16 | 17))
        && ix.accounts.len() >= 8
    {
        let exact_in = ix.data[0] == 16;
        let first = u64_at(ix.data, 1)?;
        let second = u64_at(ix.data, 9)?;
        (
            SwapProtocol::RaydiumAmmV4,
            a(1),
            a(7),
            a(5),
            a(6),
            exact_in,
            if exact_in { first } else { second },
            if exact_in { second } else { first },
        )
    } else if ix.program == RAYDIUM_AMM_V4_PROGRAM_ID
        && matches!(ix.data.first(), Some(9 | 11))
        && ix.accounts.len() >= 17
    {
        // Optional target_orders account was removed from the modern layout.
        let shift = usize::from(ix.accounts.len() == 17);
        let exact_in = ix.data[0] == 9;
        let first = u64_at(ix.data, 1)?;
        let second = u64_at(ix.data, 9)?;
        (
            SwapProtocol::RaydiumAmmV4,
            a(1),
            a(17 - shift),
            a(15 - shift),
            a(16 - shift),
            exact_in,
            if exact_in { first } else { second },
            if exact_in { second } else { first },
        )
    } else {
        return None;
    };
    let explicit_pair = match protocol {
        SwapProtocol::PumpFun => {
            Some(if exact_in && disc == Some(&crate::instr::pump::discriminators::SELL_V3[..]) {
                (a(1), a(2))
            } else {
                (a(2), a(1))
            })
        }
        SwapProtocol::LaunchLab => Some(if input == a(6) { (a(10), a(9)) } else { (a(9), a(10)) }),
        SwapProtocol::RaydiumCpmm => Some((a(10), a(11))),
        SwapProtocol::RaydiumClmm if disc == swap_v2 && ix.accounts.len() >= 13 => {
            Some((a(11), a(12)))
        }
        SwapProtocol::OrcaWhirlpool if disc == swap_v2 => {
            Some(if input == a(7) { (a(5), a(6)) } else { (a(6), a(5)) })
        }
        SwapProtocol::PumpSwap => Some(if input == a(6) { (a(4), a(3)) } else { (a(3), a(4)) }),
        _ => None,
    };
    Some(RouteSwapLeg {
        position: ix.position,
        program: ix.program,
        protocol,
        pool,
        trader,
        input_account: input,
        output_account: output,
        input_mint: mints
            .get(&input)
            .copied()
            .or_else(|| explicit_pair.map(|pair| pair.0).filter(|mint| *mint != Pubkey::default())),
        output_mint: mints
            .get(&output)
            .copied()
            .or_else(|| explicit_pair.map(|pair| pair.1).filter(|mint| *mint != Pubkey::default())),
        amount_specified_is_input: exact_in,
        specified_amount: amount,
        other_amount_threshold: threshold,
        actual_input_amount: None,
        actual_output_amount: None,
        stonkfun_mode: mode,
        stonkfun_graduated: protocol == SwapProtocol::RaydiumCpmm && graduated.contains(&pool),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_actions_require_wsol_identity_and_preserve_failure_status() {
        use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message, TransactionError};
        let token = ROUTE_SPL_TOKEN;
        let wsol = ROUTE_WSOL;
        let keys = [Pubkey::default(), token, Pubkey::new_unique(), Pubkey::new_unique(), wsol];
        let mut init = vec![18];
        init.extend_from_slice(keys[2].as_ref());
        let mut funding = 2u32.to_le_bytes().to_vec();
        funding.extend_from_slice(&123u64.to_le_bytes());
        let mut tx = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![
                    CompiledInstruction { program_id_index: 1, accounts: vec![3, 4], data: init },
                    CompiledInstruction {
                        program_id_index: 0,
                        accounts: vec![2, 3],
                        data: funding,
                    },
                    CompiledInstruction { program_id_index: 1, accounts: vec![3], data: vec![17] },
                    CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![3, 2, 2],
                        data: vec![9],
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut meta = TransactionStatusMeta::default();
        let route = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
        assert!(route.succeeded);
        assert_eq!(route.native_token_actions.len(), 3);
        assert_eq!(
            route.native_token_actions[0].action,
            NativeTokenAction::Fund { source: keys[2], lamports: 123 }
        );
        meta.err = Some(TransactionError { err: vec![1] });
        let failed = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
        assert!(!failed.succeeded);
        assert_eq!(failed.native_token_actions.len(), 3);
        tx.message.as_mut().unwrap().account_keys[4] = Pubkey::new_unique().to_bytes().to_vec();
        assert!(analyze_yellowstone_transaction_routes(&tx, &meta, &[])
            .native_token_actions
            .is_empty());
    }
    #[test]
    fn ephemeral_initialized_accounts_propagate_mints_without_balance_snapshots() {
        use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message};
        let program = ROUTE_SPL_TOKEN;
        let keys = [
            program,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let mut initialize = vec![18];
        initialize.extend_from_slice(keys[4].as_ref());
        let mut plain = vec![3];
        plain.extend_from_slice(&100u64.to_le_bytes());
        let transaction = Transaction {
            signatures: vec![],
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 0,
                        accounts: vec![1, 2],
                        data: initialize,
                    },
                    CompiledInstruction {
                        program_id_index: 0,
                        accounts: vec![1, 3, 4],
                        data: plain,
                    },
                ],
                ..Default::default()
            }),
        };
        let meta = TransactionStatusMeta::default();
        let mints = transaction_token_mints(&transaction, &meta);
        assert_eq!(mints.get(&keys[1]), Some(&keys[2]));
        assert_eq!(mints.get(&keys[3]), Some(&keys[2]));
        let route = analyze_yellowstone_transaction_routes(&transaction, &meta, &[]);
        assert_eq!(route.transfers[0].mint, Some(keys[2]));
    }
    #[test]
    fn malformed_checked_transfers_do_not_invent_amounts_or_mints() {
        let keys: Vec<_> = (0..4).map(|_| Pubkey::new_unique()).collect();
        let mut data = vec![12];
        data.extend_from_slice(&123u64.to_le_bytes());
        let mut ix = Invocation {
            position: InstructionPosition {
                outer_index: 0,
                inner_index: None,
                stack_height: Some(1),
            },
            program: ROUTE_SPL_TOKEN,
            program_resolved: true,
            accounts: &[0, 1, 2, 3],
            data: &data,
        };
        assert!(!checked_transfer(&ix));
        assert!(transfer(&ix, &keys, &HashMap::new()).is_none());
        let mut complete_data = data.clone();
        complete_data.push(6);
        ix.data = &complete_data;
        assert!(checked_transfer(&ix));
        assert_eq!(transfer(&ix, &keys, &HashMap::new()).unwrap().amount, 123);
        let mut fee_data = vec![26, 1];
        fee_data.extend_from_slice(&123u64.to_le_bytes());
        fee_data.push(6);
        fee_data.extend_from_slice(&2u64.to_le_bytes());
        ix.data = &fee_data;
        assert!(transfer(&ix, &keys, &HashMap::new()).is_none());
        ix.program = ROUTE_TOKEN_2022;
        assert_eq!(transfer(&ix, &keys, &HashMap::new()).unwrap().withheld_fee, Some(2));
    }
}

/// Analyze instruction and transfer context without altering the normal event API.
/// Unknown invocations and failed transaction status are preserved explicitly.
/// `graduated_stonkfun_pools` must be verified pool identities, not all stock pairs.
pub fn analyze_yellowstone_transaction_routes(
    transaction: &Transaction,
    meta: &TransactionStatusMeta,
    graduated_stonkfun_pools: &[Pubkey],
) -> TransactionRoute {
    let keys = transaction_keys(transaction, meta);
    let invocations = transaction_invocations(transaction, meta, &keys);
    let mints = token_mints_from_invocations(meta, &keys, &invocations);
    let mut legs = Vec::new();
    let analysis =
        invocation_analysis(&invocations, &keys, &mints, graduated_stonkfun_pools, &mut legs);
    let succeeded = meta.err.is_none();
    let mut unknown = Vec::new();
    for (i, ix) in invocations.iter().enumerate() {
        let row = &analysis[i];
        if row.skip_route {
            continue;
        }
        if let Some(index) = row.swap_index {
            let leg = &mut legs[index];
            if succeeded {
                let mut input_sum = Some(0u64);
                let mut output_sum = Some(0u64);
                let mut has_input = false;
                let mut has_output = false;
                for transfer in analysis[i + 1..row.descendant_end]
                    .iter()
                    .filter_map(|row| row.transfer.as_ref())
                {
                    if transfer.source == leg.input_account {
                        has_input = true;
                        input_sum = input_sum.and_then(|sum| sum.checked_add(transfer.amount));
                    }
                    if transfer.destination == leg.output_account {
                        has_output = true;
                        output_sum = output_sum.and_then(|sum| {
                            sum.checked_add(transfer.amount.checked_sub(transfer.withheld_fee?)?)
                        });
                    }
                }
                leg.actual_input_amount = has_input.then_some(input_sum).flatten();
                leg.actual_output_amount = has_output.then_some(output_sum).flatten();
            }
        } else {
            // Event CPI self-invocations have no token transfers and remain visible.
            unknown.push(RouteUnknownInvocation {
                position: ix.position,
                program: ix.program,
                has_token_transfers: row.has_token_transfers,
                has_known_swap_descendants: row.has_known_swap_descendants,
            });
        }
    }
    let transfers = analysis.into_iter().filter_map(|row| row.transfer).collect();
    TransactionRoute {
        signature: transaction
            .signatures
            .first()
            .and_then(|b| Signature::try_from(b.as_slice()).ok())
            .unwrap_or_default(),
        succeeded,
        legs,
        transfers,
        native_token_actions: invocations
            .iter()
            .filter_map(|ix| native_action(ix, &keys, &mints))
            .collect(),
        unknown_invocations: unknown,
    }
}

pub fn analyze_rpc_transaction_routes(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
    graduated_stonkfun_pools: &[Pubkey],
) -> Result<TransactionRoute, ParseError> {
    let (meta, transaction) = convert_rpc_to_grpc(transaction)?;
    Ok(analyze_yellowstone_transaction_routes(&transaction, &meta, graduated_stonkfun_pools))
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, InnerInstruction, InnerInstructions, Message,
    };

    fn cpmm_with_output(program: Pubkey, fee: Option<u64>) -> (Transaction, TransactionStatusMeta) {
        let mut keys: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
        keys.push(crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID);
        keys.push(program);
        let mut swap_data = crate::instr::raydium_cpmm::discriminators::SWAP_BASE_IN.to_vec();
        swap_data.extend_from_slice(&100u64.to_le_bytes());
        swap_data.extend_from_slice(&50u64.to_le_bytes());
        let mut transfer_data = if fee.is_some() { vec![26, 1] } else { vec![3] };
        transfer_data.extend_from_slice(&80u64.to_le_bytes());
        if let Some(fee) = fee {
            transfer_data.push(6);
            transfer_data.extend_from_slice(&fee.to_le_bytes());
        }
        let tx = Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![CompiledInstruction {
                    program_id_index: 13,
                    accounts: (0..13).collect(),
                    data: swap_data,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let meta = TransactionStatusMeta {
            inner_instructions: vec![InnerInstructions {
                index: 0,
                instructions: vec![InnerInstruction {
                    program_id_index: 14,
                    accounts: if fee.is_some() { vec![7, 11, 5, 1] } else { vec![7, 5, 1] },
                    data: transfer_data,
                    stack_height: Some(2),
                }],
            }],
            ..Default::default()
        };
        (tx, meta)
    }

    #[test]
    fn output_credit_requires_known_transfer_fee_and_failure_never_reports_credit() {
        let spl = ROUTE_SPL_TOKEN;
        let token22 = ROUTE_TOKEN_2022;
        for (program, fee, expected) in [
            (spl, None, Some(80)),
            (token22, None, None),
            (token22, Some(3), Some(77)),
            (token22, Some(81), None),
        ] {
            let (tx, mut meta) = cpmm_with_output(program, fee);
            let route = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
            assert_eq!(route.legs.len(), 1);
            assert_eq!(route.legs[0].actual_output_amount, expected);
            assert_eq!(route.transfers[0].amount, 80);
            meta.err = Some(yellowstone_grpc_proto::prelude::TransactionError { err: vec![1] });
            assert_eq!(
                analyze_yellowstone_transaction_routes(&tx, &meta, &[]).legs[0]
                    .actual_output_amount,
                None
            );
        }
    }

    #[test]
    fn amm_remaining_accounts_do_not_shift_user_accounts() {
        let keys: Vec<_> = (0..21).map(|_| Pubkey::new_unique()).collect();
        let mut data = vec![9];
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&80u64.to_le_bytes());
        for count in [17, 18, 20] {
            let accounts: Vec<u8> = (0..count).collect();
            let ix = Invocation {
                position: InstructionPosition {
                    outer_index: 0,
                    inner_index: None,
                    stack_height: Some(1),
                },
                program: crate::instr::program_ids::RAYDIUM_AMM_V4_PROGRAM_ID,
                program_resolved: true,
                accounts: &accounts,
                data: &data,
            };
            let leg = swap(&ix, &keys, &HashMap::new(), &[]).unwrap();
            let shift = usize::from(count == 17);
            assert_eq!(leg.input_account, keys[15 - shift]);
            assert_eq!(leg.output_account, keys[16 - shift]);
            assert_eq!(leg.trader, keys[17 - shift]);
        }
    }

    #[test]
    #[ignore = "manual local timing; excludes transport and ALT resolution"]
    fn route_parser_local_timing() {
        let (tx, meta) = cpmm_with_output(ROUTE_SPL_TOKEN, None);
        for _ in 0..100 {
            std::hint::black_box(analyze_yellowstone_transaction_routes(&tx, &meta, &[]));
        }
        let mut samples = Vec::with_capacity(21);
        for _ in 0..21 {
            let start = std::time::Instant::now();
            for _ in 0..2000 {
                std::hint::black_box(analyze_yellowstone_transaction_routes(
                    std::hint::black_box(&tx),
                    std::hint::black_box(&meta),
                    &[],
                ));
            }
            samples.push(start.elapsed().as_nanos() / 2000);
        }
        samples.sort_unstable();
        eprintln!(
            "CPMM route (one swap, one transfer): median={} ns/tx, max_batch_mean={} ns/tx",
            samples[10], samples[20]
        );
    }
}

#[cfg(test)]
mod review_missing_mint_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message, TokenBalance};

    #[test]
    fn missing_accounts_never_seed_or_propagate_a_mint() {
        let keys =
            [Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique(), ROUTE_SPL_TOKEN];
        let mint = Pubkey::new_unique();
        let mut data = vec![3];
        data.extend_from_slice(&10u64.to_le_bytes());
        let tx = Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: vec![
                    CompiledInstruction {
                        program_id_index: 3,
                        accounts: vec![255, 0, 2],
                        data: data.clone(),
                    },
                    CompiledInstruction { program_id_index: 3, accounts: vec![0, 1, 2], data },
                ],
                ..Default::default()
            }),
            ..Default::default()
        };
        let meta = TransactionStatusMeta {
            pre_token_balances: vec![TokenBalance {
                account_index: u32::MAX,
                mint: mint.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(transaction_token_mints(&tx, &meta).is_empty());
        let route = analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
        assert_eq!(route.transfers.len(), 1);
        assert_eq!(route.transfers[0].source, keys[0]);
        assert_eq!(route.transfers[0].mint, None);
    }
}

#[cfg(test)]
mod review_missing_program_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message, TokenBalance};

    #[test]
    fn invalid_program_or_source_index_cannot_masquerade_as_system_funding() {
        let keys = [Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::default()];
        let mut data = 2u32.to_le_bytes().to_vec();
        data.extend_from_slice(&100u64.to_le_bytes());
        let mut tx = Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                instructions: vec![CompiledInstruction {
                    program_id_index: 255,
                    accounts: vec![0, 1],
                    data,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let meta = TransactionStatusMeta {
            pre_token_balances: vec![TokenBalance {
                account_index: 1,
                mint: ROUTE_WSOL.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(analyze_yellowstone_transaction_routes(&tx, &meta, &[])
            .native_token_actions
            .is_empty());
        tx.message.as_mut().unwrap().instructions[0].program_id_index = 2;
        assert_eq!(
            analyze_yellowstone_transaction_routes(&tx, &meta, &[]).native_token_actions.len(),
            1
        );
        tx.message.as_mut().unwrap().instructions[0].accounts[0] = 255;
        assert!(analyze_yellowstone_transaction_routes(&tx, &meta, &[])
            .native_token_actions
            .is_empty());
    }
}

#[cfg(test)]
mod review_route_scope_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{CompiledInstruction, Message, TokenBalance};

    #[test]
    fn pumpswap_invalid_optional_bool_is_not_a_known_route_swap() {
        let mut keys: Vec<_> = (0..21).map(|_| Pubkey::new_unique()).collect();
        keys.push(crate::instr::program_ids::PUMPSWAP_PROGRAM_ID);
        for discriminator in [
            crate::instr::pump_amm::discriminators::BUY,
            crate::instr::pump_amm::discriminators::BUY_EXACT_QUOTE_IN,
        ] {
            let mut data = discriminator.to_vec();
            data.extend_from_slice(&100u64.to_le_bytes());
            data.extend_from_slice(&50u64.to_le_bytes());
            for flag in [None, Some(0), Some(1), Some(2), Some(255)] {
                let mut wire = data.clone();
                if let Some(flag) = flag {
                    wire.push(flag);
                }
                let tx = Transaction {
                    message: Some(Message {
                        account_keys: keys.iter().map(|key| key.to_bytes().to_vec()).collect(),
                        instructions: vec![CompiledInstruction {
                            program_id_index: 21,
                            accounts: (0..21).collect(),
                            data: wire,
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let meta = TransactionStatusMeta::default();
                let invocations = transaction_invocations(&tx, &meta, &keys);
                let mut legs = Vec::new();
                let rows =
                    invocation_analysis(&invocations, &keys, &HashMap::new(), &[], &mut legs);
                assert_eq!(legs.len(), usize::from(!matches!(flag, Some(2 | 255))));
                assert_eq!(rows[0].swap_index.is_some(), !matches!(flag, Some(2 | 255)));
            }
        }
    }

    #[test]
    fn dlmm_route_requires_v2_remaining_account_info() {
        let mut keys: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        keys.push(crate::instr::program_ids::METEORA_DLMM_PROGRAM_ID);
        let mut wire = crate::instr::meteora_dlmm::discriminators::SWAP2.to_vec();
        wire.extend_from_slice(&100u64.to_le_bytes());
        wire.extend_from_slice(&50u64.to_le_bytes());
        for tail in [vec![], vec![1, 0, 0, 0, 0, 1], vec![0, 0, 0, 0]] {
            let mut data = wire.clone();
            data.extend_from_slice(&tail);
            let tx = Transaction {
                message: Some(Message {
                    account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                    instructions: vec![CompiledInstruction {
                        program_id_index: 16,
                        accounts: (0..16).collect(),
                        data,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let route =
                analyze_yellowstone_transaction_routes(&tx, &TransactionStatusMeta::default(), &[]);
            assert_eq!(route.legs.len(), usize::from(tail == [0, 0, 0, 0]));
            assert_eq!(route.unknown_invocations.len(), usize::from(tail != [0, 0, 0, 0]));
        }
    }

    #[test]
    fn conflicting_mint_evidence_is_sticky_and_cannot_spread_to_neighbours() {
        let keys = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            ROUTE_SPL_TOKEN,
        ];
        let balance = |index, mint: Pubkey| TokenBalance {
            account_index: index,
            mint: mint.to_string(),
            ..Default::default()
        };
        let mut plain = vec![3];
        plain.extend_from_slice(&10u64.to_le_bytes());
        let mut checked = vec![12];
        checked.extend_from_slice(&10u64.to_le_bytes());
        checked.push(6);
        let transaction = |with_checked| {
            let mut instructions = vec![];
            if with_checked {
                instructions.push(CompiledInstruction {
                    program_id_index: 5,
                    accounts: vec![0, 4, 1, 2],
                    data: checked.clone(),
                });
            }
            for accounts in [vec![0, 1, 2], vec![1, 2, 3]] {
                instructions.push(CompiledInstruction {
                    program_id_index: 5,
                    accounts,
                    data: plain.clone(),
                });
            }
            Transaction {
                message: Some(Message {
                    account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                    instructions,
                    ..Default::default()
                }),
                ..Default::default()
            }
        };
        for with_checked in [false, true] {
            for reverse in [false, true] {
                let (before, after) = if reverse { (keys[4], keys[3]) } else { (keys[3], keys[4]) };
                let meta = TransactionStatusMeta {
                    pre_token_balances: vec![balance(0, before), balance(1, keys[4])],
                    post_token_balances: vec![balance(0, after)],
                    ..Default::default()
                };
                let tx = transaction(with_checked);
                let mints = transaction_token_mints(&tx, &meta);
                assert_eq!(mints.get(&keys[0]), None);
                assert_eq!(mints.get(&keys[1]), Some(&keys[4]));
                assert_eq!(mints.get(&keys[2]), Some(&keys[4]));
                assert_eq!(
                    analyze_yellowstone_transaction_routes(&tx, &meta, &[]).transfers[0].mint,
                    None
                );
            }
        }
        let meta = TransactionStatusMeta {
            pre_token_balances: vec![balance(0, keys[3])],
            ..Default::default()
        };
        // A checked transfer cannot silently overwrite an authoritative balance mint.
        assert_eq!(transaction_token_mints(&transaction(true), &meta).get(&keys[0]), None);
    }

    #[test]
    fn cached_subtree_boundaries_and_flags_match_exhaustive_scans() {
        let keys: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
        let opaque = Pubkey::new_unique();
        let mut swap_data = crate::instr::raydium_cpmm::discriminators::SWAP_BASE_IN.to_vec();
        swap_data.extend_from_slice(&100u64.to_le_bytes());
        swap_data.extend_from_slice(&50u64.to_le_bytes());
        let swap_accounts: Vec<u8> = (0..13).collect();
        let mut transfer_data = vec![3];
        transfer_data.extend_from_slice(&70u64.to_le_bytes());
        let transfer_accounts = [4, 5, 0];
        let mut state = 0x1234_5678u64;
        for seed in 0..128 {
            let mut invocations = Vec::new();
            for outer in 0..3 {
                invocations.push(Invocation {
                    position: InstructionPosition {
                        outer_index: outer,
                        inner_index: None,
                        stack_height: Some(1),
                    },
                    program: opaque,
                    program_resolved: true,
                    accounts: &[],
                    data: &[],
                });
                for j in 0..24 {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    let (program, accounts, data): (_, &[u8], &[u8]) = match state % 3 {
                        0 => (opaque, &[], &[]),
                        1 => (
                            crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID,
                            &swap_accounts,
                            &swap_data,
                        ),
                        _ => (ROUTE_SPL_TOKEN, &transfer_accounts, &transfer_data),
                    };
                    invocations.push(Invocation {
                        position: InstructionPosition {
                            outer_index: outer,
                            inner_index: Some(if seed % 7 == 0 { j / 2 } else { j }),
                            stack_height: if state % 5 == 0 {
                                None
                            } else {
                                Some(2 + (state % 6) as u32)
                            },
                        },
                        program,
                        program_resolved: true,
                        accounts,
                        data,
                    });
                }
            }
            let mints = HashMap::new();
            let analysis = invocation_analysis(&invocations, &keys, &mints, &[], &mut Vec::new());
            for (i, row) in analysis.iter().enumerate() {
                let reference = descendants(i, &invocations);
                assert_eq!(row.descendant_end, i + 1 + reference.len());
                assert_eq!(
                    row.has_token_transfers,
                    reference.iter().any(|ix| transfer(ix, &keys, &mints).is_some())
                );
                assert_eq!(
                    row.has_known_swap_descendants,
                    reference.iter().any(|ix| swap(ix, &keys, &mints, &[]).is_some())
                );
            }
        }
    }
}

#[cfg(test)]
mod review_invocation_join_regressions {
    use super::*;
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, InnerInstruction, InnerInstructions, Message,
    };
    fn exhaustive_invocations<'a>(
        transaction: &'a Transaction,
        meta: &'a TransactionStatusMeta,
        keys: &[Pubkey],
    ) -> Vec<Invocation<'a>> {
        let mut result = Vec::new();
        if let Some(message) = &transaction.message {
            for (i, ix) in message.instructions.iter().enumerate() {
                result.push(Invocation {
                    position: InstructionPosition {
                        outer_index: i as u32,
                        inner_index: None,
                        stack_height: Some(1),
                    },
                    program: key(keys, ix.program_id_index),
                    program_resolved: (ix.program_id_index as usize) < keys.len(),
                    accounts: &ix.accounts,
                    data: &ix.data,
                });
                for group in meta.inner_instructions.iter().filter(|g| g.index == i as u32) {
                    for (j, ix) in group.instructions.iter().enumerate() {
                        result.push(Invocation {
                            position: InstructionPosition {
                                outer_index: i as u32,
                                inner_index: Some(j as u32),
                                stack_height: ix.stack_height,
                            },
                            program: key(keys, ix.program_id_index),
                            program_resolved: (ix.program_id_index as usize) < keys.len(),
                            accounts: &ix.accounts,
                            data: &ix.data,
                        });
                    }
                }
            }
        }
        result
    }

    #[test]
    fn ordered_join_preserves_unsorted_duplicate_and_orphan_group_semantics() {
        let keys = [Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique()];
        let tx = Transaction {
            message: Some(Message {
                account_keys: keys.iter().map(|k| k.to_bytes().to_vec()).collect(),
                instructions: (0..3)
                    .map(|i| CompiledInstruction {
                        program_id_index: i,
                        data: vec![i as u8],
                        accounts: vec![0, 1],
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let indices = [2, 0, u32::MAX, 1, 0, 2];
        let mut groups: Vec<_> = indices
            .into_iter()
            .enumerate()
            .map(|(i, index)| InnerInstructions {
                index,
                instructions: vec![InnerInstruction {
                    program_id_index: if i % 2 == 0 { 1 } else { 255 },
                    accounts: vec![i as u8],
                    data: vec![i as u8],
                    stack_height: if i % 3 == 0 { None } else { Some(2) },
                }],
            })
            .collect();
        for permutation in 0..12 {
            if permutation % 2 == 0 {
                groups.rotate_left(1);
            } else {
                groups.reverse();
            }
            let meta =
                TransactionStatusMeta { inner_instructions: groups.clone(), ..Default::default() };
            let expected = exhaustive_invocations(&tx, &meta, &keys);
            let actual = transaction_invocations(&tx, &meta, &keys);
            assert_eq!(actual.len(), 8);
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert_eq!(actual.position, expected.position);
                assert_eq!(actual.program, expected.program);
                assert_eq!(actual.program_resolved, expected.program_resolved);
                assert_eq!(actual.accounts, expected.accounts);
                assert_eq!(actual.data, expected.data);
            }
        }
        groups.sort_by_key(|g| g.index);
        let meta = TransactionStatusMeta { inner_instructions: groups, ..Default::default() };
        let expected = exhaustive_invocations(&tx, &meta, &keys);
        let actual = transaction_invocations(&tx, &meta, &keys);
        assert_eq!(
            actual.iter().map(|ix| (ix.position, ix.data)).collect::<Vec<_>>(),
            expected.iter().map(|ix| (ix.position, ix.data)).collect::<Vec<_>>()
        );
        assert!(transaction_invocations(&Transaction::default(), &meta, &keys).is_empty());
    }
}
