//! Raydium 账户填充模块（包含 CLMM, CPMM, AMM V4）

use crate::core::events::*;
use crate::instr::program_ids::RAYDIUM_CLMM_PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// Official Memo program — present at index 10 on CLMM `swap_v2`.
const CLMM_MEMO_PROGRAM: Pubkey =
    solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

/// PDA seeds from Raydium CLMM IDL: `["pool_tick_array_bitmap_extension", pool_state]`.
#[inline]
pub fn tick_array_bitmap_extension_pda(pool_state: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"pool_tick_array_bitmap_extension", pool_state.as_ref()],
        &RAYDIUM_CLMM_PROGRAM_ID,
    )
    .0
}

/// Split remaining accounts into optional bitmap extension + tick arrays.
///
/// Matches on-chain `swap_v2` logic: any remaining account whose key equals the
/// pool's `TickArrayBitmapExtension` PDA is the extension; the rest are tick arrays
/// (order preserved).
#[inline]
pub fn split_clmm_remaining_accounts(
    pool_state: &Pubkey,
    remaining: &[Pubkey],
) -> (Option<Pubkey>, Vec<Pubkey>) {
    if remaining.is_empty() {
        return (None, Vec::new());
    }
    let bitmap_pda = tick_array_bitmap_extension_pda(pool_state);
    let mut bitmap = None;
    let mut ticks = Vec::with_capacity(remaining.len());
    for &key in remaining {
        if key == bitmap_pda {
            bitmap = Some(key);
        } else {
            ticks.push(key);
        }
    }
    (bitmap, ticks)
}

// ============================================================================
// Raydium CLMM
// ============================================================================

/// 填充 Raydium CLMM Swap 事件账户
///
/// `swap` (v1) fixed accounts (IDL):
/// 0: payer … 7: observationState, 8: tokenProgram, 9: tickArray,
/// remaining: optional bitmap + more tick arrays.
///
/// `swap_v2` fixed accounts (IDL):
/// 0: payer, 1: ammConfig, 2: poolState, 3: inputTokenAccount, 4: outputTokenAccount,
/// 5: inputVault, 6: outputVault, 7: observationState,
/// 8: tokenProgram, 9: tokenProgram2022, 10: memoProgram, 11: inputMint, 12: outputMint
/// Remaining: optional tickArrayBitmapExtension (any position), then tick arrays.
pub fn fill_clmm_swap_accounts(e: &mut RaydiumClmmSwapEvent, get: &AccountGetter<'_>) {
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(2);
    }
    if e.sender == Pubkey::default() {
        e.sender = get(0);
    }
    if e.token_account_0 == Pubkey::default() {
        e.token_account_0 = get(3);
    }
    if e.token_account_1 == Pubkey::default() {
        e.token_account_1 = get(4);
    }
    if e.amm_config == Pubkey::default() {
        e.amm_config = get(1);
    }
    if e.input_vault == Pubkey::default() {
        e.input_vault = get(5);
    }
    if e.output_vault == Pubkey::default() {
        e.output_vault = get(6);
    }
    if e.observation_state == Pubkey::default() {
        e.observation_state = get(7);
    }

    let is_swap_v2 = get(10) == CLMM_MEMO_PROGRAM;
    let remaining_start = if is_swap_v2 {
        if e.input_mint == Pubkey::default() {
            e.input_mint = get(11);
        }
        if e.output_mint == Pubkey::default() {
            e.output_mint = get(12);
        }
        13usize
    } else {
        // v1: first tick array is fixed account #9; do not treat #11/#12 as mints.
        9usize
    };

    if e.tick_arrays.is_empty() && e.tick_array_bitmap_extension.is_none() {
        let mut remaining = Vec::new();
        let mut idx = remaining_start;
        while idx < remaining_start + 16 {
            let key = get(idx);
            if key == Pubkey::default() {
                break;
            }
            remaining.push(key);
            idx += 1;
        }
        if e.pool_state != Pubkey::default() {
            let (bitmap, ticks) = split_clmm_remaining_accounts(&e.pool_state, &remaining);
            e.tick_array_bitmap_extension = bitmap;
            e.tick_arrays = ticks;
        } else {
            e.tick_arrays = remaining;
        }
    }
}

/// Raydium CLMM Create Pool 账户填充
///
/// createPool instruction account mapping (based on IDL):
/// 0: poolCreator
/// 1: ammConfig
/// 2: poolState
/// 3: tokenMint0
/// 4: tokenMint1
/// 5: tokenVault0
/// 6: tokenVault1
/// 7: observationState
/// 8: tokenProgram
/// 9: systemProgram
/// 10: rent
pub fn fill_clmm_create_pool_accounts(e: &mut RaydiumClmmCreatePoolEvent, get: &AccountGetter<'_>) {
    if e.creator == Pubkey::default() {
        e.creator = get(0);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(2);
    }
    if e.token_0_mint == Pubkey::default() {
        e.token_0_mint = get(3);
    }
    if e.token_1_mint == Pubkey::default() {
        e.token_1_mint = get(4);
    }
    if e.token_vault_0 == Pubkey::default() {
        e.token_vault_0 = get(5);
    }
    if e.token_vault_1 == Pubkey::default() {
        e.token_vault_1 = get(6);
    }
}

/// Raydium CLMM Open Position 账户填充
///
/// openPosition instruction account mapping (based on IDL):
/// 0: payer
/// 1: positionNftOwner
/// 2: positionNftMint
/// 3: positionNftAccount
/// 4: metadataAccount
/// 5: poolState
/// 6: protocolPosition
/// 7: tickArrayLower
/// 8: tickArrayUpper
/// 9: personalPosition
/// 10: tokenAccount0
/// 11: tokenAccount1
/// 12: tokenVault0
/// 13: tokenVault1
/// ...
pub fn fill_clmm_open_position_accounts(
    e: &mut RaydiumClmmOpenPositionEvent,
    get: &AccountGetter<'_>,
) {
    if e.user == Pubkey::default() {
        e.user = get(1);
    }
    if e.position_nft_mint == Pubkey::default() {
        e.position_nft_mint = get(2);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(5);
    }
}

/// Raydium CLMM Close Position 账户填充
///
/// closePosition instruction account mapping (based on IDL):
/// 0: nftOwner
/// 1: positionNftMint
/// 2: positionNftAccount
/// 3: personalPosition
/// 4: systemProgram
/// 5: tokenProgram
pub fn fill_clmm_close_position_accounts(
    e: &mut RaydiumClmmClosePositionEvent,
    get: &AccountGetter<'_>,
) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.position_nft_mint == Pubkey::default() {
        e.position_nft_mint = get(1);
    }
    // pool 已从事件数据解析
}

/// Raydium CLMM Increase Liquidity 账户填充
///
/// increaseLiquidity instruction account mapping (based on IDL):
/// 0: nftOwner
/// 1: nftAccount
/// 2: poolState
/// 3: protocolPosition
/// 4: personalPosition
/// 5: tickArrayLower
/// 6: tickArrayUpper
/// 7: tokenAccount0
/// 8: tokenAccount1
/// 9: tokenVault0
/// 10: tokenVault1
/// 11: tokenProgram
pub fn fill_clmm_increase_liquidity_accounts(
    e: &mut RaydiumClmmIncreaseLiquidityEvent,
    get: &AccountGetter<'_>,
) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.position_nft_mint == Pubkey::default() {
        e.position_nft_mint = get(1);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(2);
    }
}

/// Raydium CLMM Decrease Liquidity 账户填充
///
/// decreaseLiquidity instruction account mapping (based on IDL):
/// 0: nftOwner
/// 1: nftAccount
/// 2: personalPosition
/// 3: poolState
/// 4: protocolPosition
/// 5: tokenVault0
/// 6: tokenVault1
/// 7: tickArrayLower
/// 8: tickArrayUpper
/// 9: recipientTokenAccount0
/// 10: recipientTokenAccount1
/// 11: tokenProgram
pub fn fill_clmm_decrease_liquidity_accounts(
    e: &mut RaydiumClmmDecreaseLiquidityEvent,
    get: &AccountGetter<'_>,
) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.position_nft_mint == Pubkey::default() {
        e.position_nft_mint = get(1);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(3);
    }
}

// ============================================================================
// Raydium CPMM
// ============================================================================

/// Raydium CPMM Swap 账户填充
///
/// swapBaseInput/swapBaseOutput instruction account mapping:
/// 0: payer
/// 1: authority
/// 2: ammConfig
/// 3: poolState
/// 4: inputTokenAccount
/// 5: outputTokenAccount
/// 6: inputVault
/// 7: outputVault
/// 8: inputTokenProgram
/// 9: outputTokenProgram
/// 10: inputTokenMint
/// 11: outputTokenMint
/// 12: observationState
pub fn fill_cpmm_swap_accounts(e: &mut RaydiumCpmmSwapEvent, get: &AccountGetter<'_>) {
    if e.pool_id == Pubkey::default() {
        e.pool_id = get(3);
    }
    if e.amm_config == Pubkey::default() {
        e.amm_config = get(2);
    }
    if e.input_vault == Pubkey::default() {
        e.input_vault = get(6);
    }
    if e.output_vault == Pubkey::default() {
        e.output_vault = get(7);
    }
    if e.input_token_program == Pubkey::default() {
        e.input_token_program = get(8);
    }
    if e.output_token_program == Pubkey::default() {
        e.output_token_program = get(9);
    }
    if e.input_token_mint == Pubkey::default() {
        e.input_token_mint = get(10);
    }
    if e.output_token_mint == Pubkey::default() {
        e.output_token_mint = get(11);
    }
    if e.observation_state == Pubkey::default() {
        e.observation_state = get(12);
    }
}

/// Raydium CPMM Deposit 账户填充
///
/// deposit instruction account mapping:
/// 0: owner
/// 1: authority
/// 2: poolState
/// 3: ownerLpToken
/// 4: token0Account
/// 5: token1Account
/// 6: token0Vault
/// 7: token1Vault
/// ...
pub fn fill_cpmm_deposit_accounts(e: &mut RaydiumCpmmDepositEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(0); // owner
    }
}

/// Raydium CPMM Withdraw 账户填充
///
/// withdraw instruction account mapping:
/// 0: owner
/// 1: authority
/// 2: poolState
/// 3: ownerLpToken
/// 4: token0Account
/// 5: token1Account
/// ...
pub fn fill_cpmm_withdraw_accounts(e: &mut RaydiumCpmmWithdrawEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(0); // owner
    }
}

/// Raydium CPMM Initialize 账户填充
///
/// initialize instruction account mapping:
/// 0: creator
/// 1: ammConfig
/// 2: authority
/// 3: poolState
/// ...
pub fn fill_cpmm_initialize_accounts(e: &mut RaydiumCpmmInitializeEvent, get: &AccountGetter<'_>) {
    if e.creator == Pubkey::default() {
        e.creator = get(0);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(3);
    }
}

// ============================================================================
// Raydium AMM V4
// ============================================================================

/// 填充 Raydium AMM V4 Swap 事件账户
///
/// Swap instruction account mapping (based on IDL):
/// 0: tokenProgram
/// 1: amm
/// 2: ammAuthority
/// 3: ammOpenOrders
/// 4: ammTargetOrders (optional)
/// 5: poolCoinTokenAccount
/// 6: poolPcTokenAccount
/// 7: serumProgramId
/// 8: serumMarket
/// 9: serumBids
/// 10: serumAsks
/// 11: serumEventQueue
/// 12: serumCoinVaultAccount
/// 13: serumPcVaultAccount
/// 14: serumVaultSigner
/// 15: userSourceTokenAccount
/// 16: userDestTokenAccount
/// 17: userSourceOwner
pub fn fill_amm_v4_swap_accounts(e: &mut RaydiumAmmV4SwapEvent, get: &AccountGetter<'_>) {
    if e.amm == Pubkey::default() {
        e.amm = get(1);
    }
}

/// Raydium AMM V4 Deposit 账户填充
///
/// deposit instruction account mapping (based on IDL):
/// 0: tokenProgram
/// 1: amm
/// 2: ammAuthority
/// 3: ammOpenOrders
/// 4: ammTargetOrders
/// 5: lpMintAddress
/// 6: poolCoinTokenAccount
/// 7: poolPcTokenAccount
/// 8: serumMarket
/// 9: userCoinTokenAccount
/// 10: userPcTokenAccount
/// 11: userLpTokenAccount
/// 12: userOwner
/// 13: serumEventQueue
pub fn fill_amm_v4_deposit_accounts(e: &mut RaydiumAmmV4DepositEvent, get: &AccountGetter<'_>) {
    if e.token_program == Pubkey::default() {
        e.token_program = get(0);
    }
    if e.amm_authority == Pubkey::default() {
        e.amm_authority = get(2);
    }
    // amm, max_coin_amount, max_pc_amount 已从事件数据解析
}

/// Raydium AMM V4 Withdraw 账户填充
///
/// withdraw instruction account mapping (based on IDL):
/// 0: tokenProgram
/// 1: amm
/// 2: ammAuthority
/// 3: ammOpenOrders
/// 4: ammTargetOrders
/// 5: lpMintAddress
/// 6: poolCoinTokenAccount
/// 7: poolPcTokenAccount
/// 8: poolWithdrawQueue
/// 9: poolTempLpTokenAccount
/// 10: serumProgram
/// 11: serumMarket
/// 12: serumCoinVaultAccount
/// 13: serumPcVaultAccount
/// 14: serumVaultSigner
/// 15: userLpTokenAccount
/// 16: userCoinTokenAccount
/// 17: userPcTokenAccount
/// 18: userOwner
/// 19: serumEventQ
/// 20: serumBids
/// 21: serumAsks
pub fn fill_amm_v4_withdraw_accounts(e: &mut RaydiumAmmV4WithdrawEvent, get: &AccountGetter<'_>) {
    if e.token_program == Pubkey::default() {
        e.token_program = get(0);
    }
    if e.amm_authority == Pubkey::default() {
        e.amm_authority = get(2);
    }
    if e.amm_open_orders == Pubkey::default() {
        e.amm_open_orders = get(3);
    }
    // amm, amount 已从事件数据解析
}

#[cfg(test)]
mod clmm_remaining_tests {
    use super::*;

    #[test]
    fn bitmap_pda_matches_idl_seeds() {
        let pool = Pubkey::new_unique();
        let (expected, _) = Pubkey::find_program_address(
            &[b"pool_tick_array_bitmap_extension", pool.as_ref()],
            &RAYDIUM_CLMM_PROGRAM_ID,
        );
        assert_eq!(tick_array_bitmap_extension_pda(&pool), expected);
    }

    #[test]
    fn split_pulls_bitmap_from_any_remaining_slot() {
        let pool = Pubkey::new_unique();
        let bitmap = tick_array_bitmap_extension_pda(&pool);
        let t0 = Pubkey::new_unique();
        let t1 = Pubkey::new_unique();
        let (ext, ticks) = split_clmm_remaining_accounts(&pool, &[t0, bitmap, t1]);
        assert_eq!(ext, Some(bitmap));
        assert_eq!(ticks, vec![t0, t1]);
    }

    #[test]
    fn fill_swap_v2_separates_bitmap_and_ticks() {
        let memo = CLMM_MEMO_PROGRAM;
        let pool = Pubkey::new_unique();
        let bitmap = tick_array_bitmap_extension_pda(&pool);
        let t0 = Pubkey::new_unique();
        let t1 = Pubkey::new_unique();
        let mut accounts: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        accounts[2] = pool;
        accounts[10] = memo;
        accounts[13] = bitmap;
        accounts[14] = t0;
        accounts[15] = t1;
        let mut e = RaydiumClmmSwapEvent::default();
        fill_clmm_swap_accounts(&mut e, &|i| accounts.get(i).copied().unwrap_or_default());
        assert_eq!(e.tick_array_bitmap_extension, Some(bitmap));
        assert_eq!(e.tick_arrays, vec![t0, t1]);
        assert_eq!(e.input_mint, accounts[11]);
        assert_eq!(e.output_mint, accounts[12]);
    }

    #[test]
    fn fill_swap_v1_starts_remaining_at_tick_array_slot() {
        let pool = Pubkey::new_unique();
        let t0 = Pubkey::new_unique();
        let t1 = Pubkey::new_unique();
        let mut accounts: Vec<_> = (0..11).map(|_| Pubkey::new_unique()).collect();
        accounts[2] = pool;
        // index 10 is NOT memo → v1; only two remaining slots (9, 10)
        accounts[9] = t0;
        accounts[10] = t1;
        let mut e = RaydiumClmmSwapEvent::default();
        fill_clmm_swap_accounts(&mut e, &|i| accounts.get(i).copied().unwrap_or_default());
        assert!(e.tick_array_bitmap_extension.is_none());
        assert_eq!(e.tick_arrays, vec![t0, t1]);
        assert_eq!(e.input_mint, Pubkey::default());
    }
}
