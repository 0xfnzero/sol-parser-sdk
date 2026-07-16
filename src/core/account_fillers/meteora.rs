//! Meteora 账户填充模块（包含 DAMM V2, Pools, DLMM）

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

// ============================================================================
// Meteora DAMM V2
// ============================================================================

/// Meteora DAMM V2 Swap 账户填充
///
/// swap/swap2 instruction account mapping (based on IDL):
/// 0: pool_authority
/// 1: config
/// 2: pool
/// 3: input_token_account
/// 4: output_token_account
/// 5: base_vault
/// 6: quote_vault
/// 7: base_mint
/// 8: quote_mint
/// 9: payer
/// 10: token_base_program
/// 11: token_quote_program
/// 12: referral_token_account
/// 13: event_authority
/// 14: program
pub fn fill_damm_v2_swap_accounts(_e: &mut MeteoraDammV2SwapEvent, _get: &AccountGetter<'_>) {
    // 大部分字段已从事件数据解析
    // DAMM V2 使用虚拟池，账户字段较少
}

pub fn fill_damm_v2_create_position_accounts(
    _e: &mut MeteoraDammV2CreatePositionEvent,
    _get: &AccountGetter<'_>,
) {
    // DAMM V2 是动态 AMM，没有传统的 position 概念
    // 此事件类型可能不适用
}

pub fn fill_damm_v2_close_position_accounts(
    _e: &mut MeteoraDammV2ClosePositionEvent,
    _get: &AccountGetter<'_>,
) {
    // DAMM V2 是动态 AMM，没有传统的 position 概念
    // 此事件类型可能不适用
}

pub fn fill_damm_v2_add_liquidity_accounts(
    _e: &mut MeteoraDammV2AddLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // DAMM V2 流动性操作通过 initialize_virtual_pool 等指令
    // 事件数据已包含主要信息
}

pub fn fill_damm_v2_remove_liquidity_accounts(
    _e: &mut MeteoraDammV2RemoveLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // DAMM V2 流动性移除操作
    // 事件数据已包含主要信息
}

pub fn fill_damm_v2_initialize_pool_accounts(
    e: &mut MeteoraDammV2InitializePoolEvent,
    get: &AccountGetter<'_>,
) {
    if e.creator == Pubkey::default() {
        e.creator = get(0);
    }
    if e.position_nft_mint == Pubkey::default() {
        e.position_nft_mint = get(1);
    }
    if e.pool == Pubkey::default() {
        e.pool = get(6);
    }
    if e.position == Pubkey::default() {
        e.position = get(7);
    }
    if e.token_a_mint == Pubkey::default() {
        e.token_a_mint = get(8);
    }
    if e.token_b_mint == Pubkey::default() {
        e.token_b_mint = get(9);
    }
}

// ============================================================================
// Meteora Pools
// ============================================================================

/// Meteora Pools Swap 账户填充
///
/// swap instruction account mapping (based on IDL):
/// 0: pool
/// 1: userSourceToken
/// 2: userDestinationToken
/// 3: aVault
/// 4: bVault
/// 5: aTokenVault
/// 6: bTokenVault
/// 7: aVaultLpMint
/// 8: bVaultLpMint
/// 9: aVaultLp
/// 10: bVaultLp
/// 11: adminTokenFee
/// 12: user
/// 13: vaultProgram
/// 14: tokenProgram
pub fn fill_pools_swap_accounts(_e: &mut MeteoraPoolsSwapEvent, _get: &AccountGetter<'_>) {
    // 事件数据已包含主要信息
}

/// Meteora Pools Add Liquidity 账户填充
///
/// addBalanceLiquidity/addImbalanceLiquidity instruction account mapping:
/// 0: pool
/// 1: lpMint
/// 2: userPoolLp
/// 3: aVaultLp
/// 4: bVaultLp
/// 5: aVault
/// 6: bVault
/// 7: aVaultLpMint
/// 8: bVaultLpMint
/// 9: aTokenVault
/// 10: bTokenVault
/// 11: userAToken
/// 12: userBToken
/// 13: user
/// ...
pub fn fill_pools_add_liquidity_accounts(
    _e: &mut MeteoraPoolsAddLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // 事件数据已包含主要信息
}

/// Meteora Pools Remove Liquidity 账户填充
///
/// removeBalanceLiquidity/removeLiquiditySingleSide instruction account mapping:
/// 0: pool
/// 1: lpMint
/// 2: userPoolLp
/// 3: aVaultLp
/// 4: bVaultLp
/// 5: aVault
/// 6: bVault
/// ...
pub fn fill_pools_remove_liquidity_accounts(
    _e: &mut MeteoraPoolsRemoveLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // 事件数据已包含主要信息
}

// ============================================================================
// Meteora DLMM
// ============================================================================

/// Meteora DLMM Swap 账户填充
///
/// swap instruction account mapping (based on IDL):
/// 0: lbPair
/// 1: binArrayBitmapExtension
/// 2: reserveX
/// 3: reserveY
/// 4: userTokenIn
/// 5: userTokenOut
/// 6: tokenXMint
/// 7: tokenYMint
/// 8: oracle
/// 9: hostFeeIn
/// 10: user
/// 11: tokenXProgram
/// 12: tokenYProgram
/// 13: eventAuthority
/// 14: program
pub fn fill_dlmm_swap_accounts(_e: &mut MeteoraDlmmSwapEvent, _get: &AccountGetter<'_>) {
    // 事件数据已包含主要信息
}

/// Meteora DLMM Add Liquidity 账户填充
///
/// addLiquidity instruction account mapping (based on IDL):
/// 0: position
/// 1: lbPair
/// 2: binArrayBitmapExtension
/// 3: userTokenX
/// 4: userTokenY
/// 5: reserveX
/// 6: reserveY
/// 7: tokenXMint
/// 8: tokenYMint
/// 9: binArrayLower
/// 10: binArrayUpper
/// 11: sender
/// ...
pub fn fill_dlmm_add_liquidity_accounts(
    _e: &mut MeteoraDlmmAddLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // 事件数据已包含主要信息
}

/// Meteora DLMM Remove Liquidity 账户填充
///
/// removeLiquidity instruction account mapping (based on IDL):
/// 0: position
/// 1: lbPair
/// 2: binArrayBitmapExtension
/// 3: userTokenX
/// 4: userTokenY
/// 5: reserveX
/// 6: reserveY
/// 7: tokenXMint
/// 8: tokenYMint
/// 9: binArrayLower
/// 10: binArrayUpper
/// 11: sender
/// ...
pub fn fill_dlmm_remove_liquidity_accounts(
    _e: &mut MeteoraDlmmRemoveLiquidityEvent,
    _get: &AccountGetter<'_>,
) {
    // 事件数据已包含主要信息
}
