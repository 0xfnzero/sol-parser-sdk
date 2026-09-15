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
/// 1: pool
/// 2: input_token_account
/// 3: output_token_account
/// 4: token_a_vault
/// 5: token_b_vault
/// 6: token_a_mint
/// 7: token_b_mint
/// 8: payer
/// 9: token_a_program
/// 10: token_b_program
/// 11: optional referral_token_account (or program-id placeholder)
/// 12: event_authority (11 when optional account is omitted)
/// 13: program (12 when optional account is omitted)
pub fn fill_damm_v2_swap_accounts(
    e: &mut MeteoraDammV2SwapEvent,
    get: &AccountGetter<'_>,
    account_count: usize,
) {
    if !(13..=14).contains(&account_count) {
        return;
    }
    e.pool_authority = get(0);
    e.input_token_account = get(2);
    e.output_token_account = get(3);
    e.token_a_vault = get(4);
    e.token_b_vault = get(5);
    e.token_a_mint = get(6);
    e.token_b_mint = get(7);
    e.payer = get(8);
    e.token_a_program = get(9);
    e.token_b_program = get(10);
    let authority_index = account_count - 2;
    e.event_authority = get(authority_index);
    e.program = get(account_count - 1);
    e.referral_token_account = (account_count == 14 && get(11) != e.program).then(|| get(11));
}

#[cfg(test)]
mod damm_swap_tests {
    use super::*;

    #[test]
    fn optional_referral_is_not_the_program_placeholder() {
        let accounts: Vec<_> = (0..14).map(|_| Pubkey::new_unique()).collect();
        let mut event = MeteoraDammV2SwapEvent::default();
        fill_damm_v2_swap_accounts(
            &mut event,
            &|i| accounts.get(i).copied().unwrap_or_default(),
            14,
        );
        assert_eq!(event.token_a_mint, accounts[6]);
        assert_eq!(event.payer, accounts[8]);
        assert_eq!(event.referral_token_account, Some(accounts[11]));
        assert_eq!(event.event_authority, accounts[12]);
        assert_eq!(event.program, accounts[13]);

        let mut without_referral = accounts.clone();
        without_referral[11] = accounts[13];
        fill_damm_v2_swap_accounts(
            &mut event,
            &|i| without_referral.get(i).copied().unwrap_or_default(),
            14,
        );
        assert_eq!(event.referral_token_account, None);

        let mut omitted = MeteoraDammV2SwapEvent::default();
        fill_damm_v2_swap_accounts(
            &mut omitted,
            &|i| accounts.get(i).copied().unwrap_or_default(),
            13,
        );
        assert_eq!(omitted.referral_token_account, None);
        assert_eq!(omitted.event_authority, accounts[11]);
        assert_eq!(omitted.program, accounts[12]);
    }

    #[test]
    fn older_swap_json_defaults_new_accounts() {
        let mut json = serde_json::to_value(MeteoraDammV2SwapEvent::default()).unwrap();
        let map = json.as_object_mut().unwrap();
        for field in [
            "pool_authority",
            "input_token_account",
            "output_token_account",
            "payer",
            "referral_token_account",
            "event_authority",
            "program",
        ] {
            map.remove(field);
        }
        let decoded: MeteoraDammV2SwapEvent = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.pool_authority, Pubkey::default());
        assert_eq!(decoded.referral_token_account, None);
    }

    #[test]
    fn invalid_account_count_does_not_panic_or_fill() {
        let mut event = MeteoraDammV2SwapEvent::default();
        fill_damm_v2_swap_accounts(&mut event, &|_| Pubkey::new_unique(), 0);
        assert_eq!(event.token_a_mint, Pubkey::default());
    }
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
pub fn fill_dlmm_swap_accounts(e: &mut MeteoraDlmmSwapEvent, get: &AccountGetter<'_>) {
    if e.user_token_in == Pubkey::default() {
        e.user_token_in = get(4);
    }
    if e.user_token_out == Pubkey::default() {
        e.user_token_out = get(5);
    }
    if e.token_x_mint == Pubkey::default() {
        e.token_x_mint = get(6);
    }
    if e.token_y_mint == Pubkey::default() {
        e.token_y_mint = get(7);
    }
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
