//! Raydium 账户填充模块（包含 CLMM, CPMM, AMM V4）

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

// ============================================================================
// Raydium CLMM
// ============================================================================

/// 填充 Raydium CLMM Swap 事件账户
///
/// Swap instruction account mapping (based on IDL):
/// 0: payer
/// 1: ammConfig
/// 2: poolState
/// 3: inputTokenAccount
/// 4: outputTokenAccount
/// 5: inputVault
/// 6: outputVault
/// 7: observationState
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
pub fn fill_cpmm_swap_accounts(_e: &mut RaydiumCpmmSwapEvent, _get: &AccountGetter<'_>) {
    // pool_id, input_amount, output_amount 已从事件数据解析
    // 其他字段不需要填充
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
