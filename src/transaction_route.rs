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
    let wsol = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
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
    if mints.get(&target) != Some(&wsol) {
        return None;
    }
    Some(RouteNativeTokenAction { position: ix.position, account: target, action })
}

struct Invocation<'a> {
    position: InstructionPosition,
    program: Pubkey,
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
    program == solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        || program == solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
}
fn checked_transfer(ix: &Invocation<'_>) -> bool {
    token_program(ix.program)
        && ix.data.first() == Some(&12)
        && ix.data.len() >= 10
        && ix.accounts.len() >= 4
}
fn checked_transfer_with_fee(ix: &Invocation<'_>) -> bool {
    ix.program == solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
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
fn descendants<'a>(index: usize, invocations: &'a [Invocation<'_>]) -> &'a [Invocation<'a>] {
    let parent = invocations[index].position;
    let end = (index + 1..invocations.len())
        .find(|&i| !descendant(parent, invocations[i].position))
        .unwrap_or(invocations.len());
    &invocations[index + 1..end]
}

/// Resolve mints from pre/post balances and checked transfers, including accounts
/// created and closed within the transaction. Plain transfers propagate only
/// already known mints; an arbitrary account key is never treated as a mint.
pub(crate) fn transaction_token_mints(
    transaction: &Transaction,
    meta: &TransactionStatusMeta,
) -> HashMap<Pubkey, Pubkey> {
    let keys = transaction_keys(transaction, meta);
    let invocations = transaction_invocations(transaction, meta, &keys);
    let mut mints = HashMap::new();
    for balance in meta.pre_token_balances.iter().chain(&meta.post_token_balances) {
        if let Ok(mint) = balance.mint.parse() {
            mints.insert(key(&keys, balance.account_index), mint);
        }
    }
    for ix in &invocations {
        if checked_transfer(ix) {
            let mint = account(ix, &keys, 1);
            mints.insert(account(ix, &keys, 0), mint);
            mints.insert(account(ix, &keys, 2), mint);
        }
        // Token-2022 TransferFeeExtension::TransferCheckedWithFee.
        if checked_transfer_with_fee(ix) {
            let mint = account(ix, &keys, 1);
            mints.insert(account(ix, &keys, 0), mint);
            mints.insert(account(ix, &keys, 2), mint);
        }
        // InitializeAccount / InitializeAccount2 / InitializeAccount3 also
        // identify ephemeral accounts without pre/post token balances.
        if token_program(ix.program)
            && ix.accounts.len() >= 2
            && matches!((ix.data.first(), ix.data.len()), (Some(1), 1) | (Some(16 | 18), 33))
        {
            mints.insert(account(ix, &keys, 0), account(ix, &keys, 1));
        }
    }
    // Fixed point rather than a fixed number of hops; each pass adds information.
    loop {
        let before = mints.len();
        for ix in &invocations {
            if token_program(ix.program)
                && ix.data.first() == Some(&3)
                && ix.data.len() >= 9
                && ix.accounts.len() >= 3
            {
                let source = account(ix, &keys, 0);
                let destination = account(ix, &keys, 1);
                if let Some(mint) = mints.get(&source).or_else(|| mints.get(&destination)).copied()
                {
                    mints.entry(source).or_insert(mint);
                    mints.entry(destination).or_insert(mint);
                }
            }
        }
        if mints.len() == before {
            break;
        }
    }
    mints.remove(&Pubkey::default());
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
                        accounts: &ix.accounts,
                        data: &ix.data,
                    });
                }
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
        3 if ix.accounts.len() >= 3 && ix.data.len() >= 9 => (1, 1, Some(0)),
        12 if checked_transfer(ix) => (
            2,
            1,
            if ix.program == solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") {
                Some(0)
            } else {
                None
            },
        ),
        26 if checked_transfer_with_fee(ix) => (2, 2, Some(u64_at(ix.data, 11)?)),
        _ => return None,
    };
    let source = account(ix, keys, 0);
    Some(RouteTokenTransfer {
        position: ix.position,
        program: ix.program,
        source,
        destination: account(ix, keys, destination_index),
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
        && ix.accounts.len() >= 10
    {
        (
            SwapProtocol::RaydiumClmm,
            a(2),
            a(0),
            a(3),
            a(4),
            *ix.data.get(40)? != 0,
            u64_at(ix.data, 8)?,
            u64_at(ix.data, 16)?,
        )
    } else if ix.program == ORCA_WHIRLPOOL_PROGRAM_ID && (disc == swap_disc || disc == swap_v2) {
        let v2 = disc == swap_v2;
        let direction = *ix.data.get(41)? != 0;
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
            *ix.data.get(40)? != 0,
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
    } else if ix.program == METEORA_DLMM_PROGRAM_ID && ix.accounts.len() >= 11 {
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
    } else if ix.program == PUMPSWAP_PROGRAM_ID && ix.accounts.len() >= 21 {
        use crate::instr::pump_amm::discriminators::*;
        let (buy, exact_in) = if disc == Some(&BUY_EXACT_QUOTE_IN[..]) {
            (true, true)
        } else if disc == Some(&BUY[..]) {
            (true, false)
        } else if disc == Some(&SELL[..]) {
            (false, true)
        } else {
            return None;
        };
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
        let n = ix.accounts.len();
        let exact_in = ix.data[0] == 9;
        let first = u64_at(ix.data, 1)?;
        let second = u64_at(ix.data, 9)?;
        (
            SwapProtocol::RaydiumAmmV4,
            a(1),
            a(n - 1),
            a(n - 3),
            a(n - 2),
            exact_in,
            if exact_in { first } else { second },
            if exact_in { second } else { first },
        )
    } else {
        return None;
    };
    let explicit_pair = match protocol {
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
        let token = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let wsol = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
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
        let program = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
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
            program: solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
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
        ix.program = solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
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
    let mints = transaction_token_mints(transaction, meta);
    let transfers: Vec<_> =
        invocations.iter().filter_map(|ix| transfer(ix, &keys, &mints)).collect();
    let succeeded = meta.err.is_none();
    let mut legs = Vec::new();
    let mut unknown = Vec::new();
    for (i, ix) in invocations.iter().enumerate() {
        let children = descendants(i, &invocations);
        let nested: Vec<_> =
            children.iter().filter_map(|child| transfer(child, &keys, &mints)).collect();
        if let Some(mut leg) = swap(ix, &keys, &mints, graduated_stonkfun_pools) {
            if succeeded && !nested.is_empty() {
                let inputs: Vec<_> =
                    nested.iter().filter(|t| t.source == leg.input_account).collect();
                let outputs: Vec<_> =
                    nested.iter().filter(|t| t.destination == leg.output_account).collect();
                if !inputs.is_empty() {
                    leg.actual_input_amount =
                        inputs.iter().try_fold(0u64, |sum, t| sum.checked_add(t.amount));
                }
                if !outputs.is_empty() {
                    leg.actual_output_amount = outputs.iter().try_fold(0u64, |sum, t| {
                        sum.checked_add(t.amount.checked_sub(t.withheld_fee?)?)
                    });
                }
            }
            legs.push(leg);
        } else if !token_program(ix.program)
            && ix.program != Pubkey::default()
            && ix.program != solana_sdk::pubkey!("ComputeBudget111111111111111111111111111111")
            && ix.program != solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            && ix.program != solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
        {
            // Event CPI self-invocations have no token transfers and remain visible.
            unknown.push(RouteUnknownInvocation {
                position: ix.position,
                program: ix.program,
                has_token_transfers: !nested.is_empty(),
                has_known_swap_descendants: children
                    .iter()
                    .any(|child| swap(child, &keys, &mints, graduated_stonkfun_pools).is_some()),
            });
        }
    }
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
