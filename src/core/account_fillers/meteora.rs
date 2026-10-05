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
/// 11: protocolTokenFee
/// 12: user
/// 13: vaultProgram
/// 14: tokenProgram
pub fn fill_pools_swap_accounts(e: &mut MeteoraPoolsSwapEvent, get: &AccountGetter<'_>) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.user_source_token == Pubkey::default() {
        e.user_source_token = get(1);
    }
    if e.user_destination_token == Pubkey::default() {
        e.user_destination_token = get(2);
    }
    if e.a_vault == Pubkey::default() {
        e.a_vault = get(3);
    }
    if e.b_vault == Pubkey::default() {
        e.b_vault = get(4);
    }
    if e.a_token_vault == Pubkey::default() {
        e.a_token_vault = get(5);
    }
    if e.b_token_vault == Pubkey::default() {
        e.b_token_vault = get(6);
    }
    if e.a_vault_lp_mint == Pubkey::default() {
        e.a_vault_lp_mint = get(7);
    }
    if e.b_vault_lp_mint == Pubkey::default() {
        e.b_vault_lp_mint = get(8);
    }
    if e.a_vault_lp == Pubkey::default() {
        e.a_vault_lp = get(9);
    }
    if e.b_vault_lp == Pubkey::default() {
        e.b_vault_lp = get(10);
    }
    if e.protocol_token_fee == Pubkey::default() {
        e.protocol_token_fee = get(11);
    }
    if e.user == Pubkey::default() {
        e.user = get(12);
    }
    if e.vault_program == Pubkey::default() {
        e.vault_program = get(13);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(14);
    }
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
/// 14: vaultProgram
/// 15: tokenProgram
pub fn fill_pools_add_liquidity_accounts(
    e: &mut MeteoraPoolsAddLiquidityEvent,
    get: &AccountGetter<'_>,
) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.lp_mint == Pubkey::default() {
        e.lp_mint = get(1);
    }
    if e.user_pool_lp == Pubkey::default() {
        e.user_pool_lp = get(2);
    }
    if e.a_vault_lp == Pubkey::default() {
        e.a_vault_lp = get(3);
    }
    if e.b_vault_lp == Pubkey::default() {
        e.b_vault_lp = get(4);
    }
    if e.a_vault == Pubkey::default() {
        e.a_vault = get(5);
    }
    if e.b_vault == Pubkey::default() {
        e.b_vault = get(6);
    }
    if e.a_vault_lp_mint == Pubkey::default() {
        e.a_vault_lp_mint = get(7);
    }
    if e.b_vault_lp_mint == Pubkey::default() {
        e.b_vault_lp_mint = get(8);
    }
    if e.a_token_vault == Pubkey::default() {
        e.a_token_vault = get(9);
    }
    if e.b_token_vault == Pubkey::default() {
        e.b_token_vault = get(10);
    }
    if e.user_a_token == Pubkey::default() {
        e.user_a_token = get(11);
    }
    if e.user_b_token == Pubkey::default() {
        e.user_b_token = get(12);
    }
    if e.user == Pubkey::default() {
        e.user = get(13);
    }
    if e.vault_program == Pubkey::default() {
        e.vault_program = get(14);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(15);
    }
}

/// Meteora Pools Remove Liquidity 账户填充
///
/// Balance removal uses 16 accounts. Single-side removal shares indices 0..10,
/// then destination token 11, user 12, vault program 13, token program 14.
/// `ix_name` selects the layout; the dispatcher derives it from the discriminator.
/// removeBalanceLiquidity instruction account mapping:
/// 0: pool
/// 1: lpMint
/// 2: userPoolLp
/// 3: aVaultLp
/// 4: bVaultLp
/// 5: aVault
/// 6: bVault
/// ...
pub fn fill_pools_remove_liquidity_accounts(
    e: &mut MeteoraPoolsRemoveLiquidityEvent,
    get: &AccountGetter<'_>,
) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.lp_mint == Pubkey::default() {
        e.lp_mint = get(1);
    }
    if e.user_pool_lp == Pubkey::default() {
        e.user_pool_lp = get(2);
    }
    if e.a_vault_lp == Pubkey::default() {
        e.a_vault_lp = get(3);
    }
    if e.b_vault_lp == Pubkey::default() {
        e.b_vault_lp = get(4);
    }
    if e.a_vault == Pubkey::default() {
        e.a_vault = get(5);
    }
    if e.b_vault == Pubkey::default() {
        e.b_vault = get(6);
    }
    if e.a_vault_lp_mint == Pubkey::default() {
        e.a_vault_lp_mint = get(7);
    }
    if e.b_vault_lp_mint == Pubkey::default() {
        e.b_vault_lp_mint = get(8);
    }
    if e.a_token_vault == Pubkey::default() {
        e.a_token_vault = get(9);
    }
    if e.b_token_vault == Pubkey::default() {
        e.b_token_vault = get(10);
    }
    let single_side = e.ix_name == "remove_liquidity_single_side";
    if single_side {
        if e.user_destination_token == Pubkey::default() {
            e.user_destination_token = get(11);
        }
    } else {
        if e.user_a_token == Pubkey::default() {
            e.user_a_token = get(11);
        }
        if e.user_b_token == Pubkey::default() {
            e.user_b_token = get(12);
        }
    }
    if e.user == Pubkey::default() {
        e.user = get(if single_side { 12 } else { 13 });
    }
    if e.vault_program == Pubkey::default() {
        e.vault_program = get(if single_side { 13 } else { 14 });
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(if single_side { 14 } else { 15 });
    }
}

pub fn fill_pools_bootstrap_liquidity_accounts(
    e: &mut MeteoraPoolsBootstrapLiquidityEvent,
    get: &AccountGetter<'_>,
) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.lp_mint == Pubkey::default() {
        e.lp_mint = get(1);
    }
    if e.user_pool_lp == Pubkey::default() {
        e.user_pool_lp = get(2);
    }
    if e.a_vault_lp == Pubkey::default() {
        e.a_vault_lp = get(3);
    }
    if e.b_vault_lp == Pubkey::default() {
        e.b_vault_lp = get(4);
    }
    if e.a_vault == Pubkey::default() {
        e.a_vault = get(5);
    }
    if e.b_vault == Pubkey::default() {
        e.b_vault = get(6);
    }
    if e.a_vault_lp_mint == Pubkey::default() {
        e.a_vault_lp_mint = get(7);
    }
    if e.b_vault_lp_mint == Pubkey::default() {
        e.b_vault_lp_mint = get(8);
    }
    if e.a_token_vault == Pubkey::default() {
        e.a_token_vault = get(9);
    }
    if e.b_token_vault == Pubkey::default() {
        e.b_token_vault = get(10);
    }
    if e.user_a_token == Pubkey::default() {
        e.user_a_token = get(11);
    }
    if e.user_b_token == Pubkey::default() {
        e.user_b_token = get(12);
    }
    if e.user == Pubkey::default() {
        e.user = get(13);
    }
    if e.vault_program == Pubkey::default() {
        e.vault_program = get(14);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(15);
    }
}

// ============================================================================
// Meteora DLMM
// ============================================================================

/// Meteora DLMM Swap 账户填充
///
/// `swap` (v1) fixed accounts (IDL):
/// 0..12 shared, 13: eventAuthority, 14: program, remaining: bin arrays
///
/// `swap2` fixed accounts (IDL):
/// 0: lbPair, 1: binArrayBitmapExtension (optional), 2: reserveX, 3: reserveY,
/// 4: userTokenIn, 5: userTokenOut, 6: tokenXMint, 7: tokenYMint,
/// 8: oracle, 9: hostFeeIn (optional), 10: user, 11: tokenXProgram, 12: tokenYProgram,
/// 13: memoProgram, 14: eventAuthority, 15: program, remaining: bin arrays
/// Without instruction data, v2 hook boundaries are unknown; leave bin arrays unset.
pub fn fill_dlmm_swap_accounts(e: &mut MeteoraDlmmSwapEvent, get: &AccountGetter<'_>) {
    const MEMO_PROGRAM: Pubkey = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
    let v2 = get(13) == MEMO_PROGRAM;
    let start = if v2 { 16 } else { 15 };
    fill_dlmm_swap_accounts_with_layout(e, get, start, if v2 { start } else { start + 16 }, v2);
}

pub(crate) fn fill_dlmm_swap_accounts_with_layout(
    e: &mut MeteoraDlmmSwapEvent, get: &AccountGetter<'_>, bins_start: usize, bins_end: usize, is_swap2: bool,
) {

    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
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
    if e.reserve_x == Pubkey::default() {
        e.reserve_x = get(2);
    }
    if e.reserve_y == Pubkey::default() {
        e.reserve_y = get(3);
    }
    if e.oracle == Pubkey::default() {
        e.oracle = get(8);
    }
    if e.token_x_program == Pubkey::default() {
        e.token_x_program = get(11);
    }
    if e.token_y_program == Pubkey::default() {
        e.token_y_program = get(12);
    }

    // swap2 inserts memo_program at index 13; swap (v1) has event_authority there.
    let program_idx = if is_swap2 { 15 } else { 14 };

    if e.bitmap_extension.is_none() {
        let ext = get(1);
        // Unused optional accounts are often the program id placeholder.
        let program = get(program_idx);
        if ext != Pubkey::default() && ext != e.pool && ext != program {
            e.bitmap_extension = Some(ext);
        }
    }
    if e.bin_arrays.is_empty() {
        if bins_start >= bins_end || get(bins_start) == Pubkey::default() { return; }
        let mut bins = Vec::with_capacity(bins_end.saturating_sub(bins_start));
        let mut idx = bins_start;
        while idx < bins_end {
            let key = get(idx);
            if key == Pubkey::default() {
                break;
            }
            bins.push(key);
            idx += 1;
        }
        e.bin_arrays = bins;
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

#[cfg(test)]
mod dlmm_swap_tests {
    use super::*;

    #[test]
    fn swap2_bin_arrays_start_after_memo_and_program() {
        let memo = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        let mut accounts: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
        accounts[13] = memo;
        let bin0 = accounts[16];
        let bin1 = accounts[17];
        let mut e = MeteoraDlmmSwapEvent::default();
        fill_dlmm_swap_accounts_with_layout(&mut e, &|i| accounts.get(i).copied().unwrap_or_default(), 16, accounts.len(), true);
        assert_eq!(e.bin_arrays, vec![bin0, bin1]);
        assert_eq!(e.reserve_x, accounts[2]);
        assert_eq!(e.oracle, accounts[8]);
    }

    #[test]
    fn swap_v1_bin_arrays_start_at_15() {
        let accounts: Vec<_> = (0..17).map(|_| Pubkey::new_unique()).collect();
        // index 13 is NOT memo → v1 layout
        let bin0 = accounts[15];
        let bin1 = accounts[16];
        let mut e = MeteoraDlmmSwapEvent::default();
        fill_dlmm_swap_accounts(&mut e, &|i| accounts.get(i).copied().unwrap_or_default());
        assert_eq!(e.bin_arrays, vec![bin0, bin1]);
    }
}

/// IDL initialization layouts selected by the instruction name.
/// The dispatcher supplies this name from the matched discriminator.
pub fn fill_pools_pool_created_accounts(
    e: &mut MeteoraPoolsPoolCreatedEvent,
    get: &AccountGetter<'_>,
) {
    macro_rules! fill {
        ($($field:ident = $index:expr),* $(,)?) => {
            $(if e.$field == Pubkey::default() { e.$field = get($index); })*
        };
    }
    match e.ix_name.as_str() {
        "initialize_permissioned_pool" => {
            fill!(
                pool = 0,
                lp_mint = 1,
                token_a_mint = 2,
                token_b_mint = 3,
                a_vault = 4,
                b_vault = 5,
                a_vault_lp_mint = 6,
                b_vault_lp_mint = 7,
                a_vault_lp = 8,
                b_vault_lp = 9,
                admin_token_a = 10,
                admin_token_b = 11,
                admin_pool_lp = 12,
                protocol_token_a_fee = 13,
                protocol_token_b_fee = 14,
                admin = 15,
                fee_owner = 16,
                rent = 17,
                mint_metadata = 18,
                metadata_program = 19,
                vault_program = 20,
                token_program = 21,
                associated_token_program = 22,
                system_program = 23
            );
        }
        "initialize_permissionless_pool" | "initialize_permissionless_pool_with_fee_tier" => {
            fill!(
                pool = 0,
                lp_mint = 1,
                token_a_mint = 2,
                token_b_mint = 3,
                a_vault = 4,
                b_vault = 5,
                a_token_vault = 6,
                b_token_vault = 7,
                a_vault_lp_mint = 8,
                b_vault_lp_mint = 9,
                a_vault_lp = 10,
                b_vault_lp = 11,
                payer_token_a = 12,
                payer_token_b = 13,
                payer_pool_lp = 14,
                protocol_token_a_fee = 15,
                protocol_token_b_fee = 16,
                payer = 17,
                fee_owner = 18,
                rent = 19,
                mint_metadata = 20,
                metadata_program = 21,
                vault_program = 22,
                token_program = 23,
                associated_token_program = 24,
                system_program = 25
            );
        }
        "initialize_customizable_permissionless_constant_product_pool" => {
            fill!(
                pool = 0,
                lp_mint = 1,
                token_a_mint = 2,
                token_b_mint = 3,
                a_vault = 4,
                b_vault = 5,
                a_token_vault = 6,
                b_token_vault = 7,
                a_vault_lp_mint = 8,
                b_vault_lp_mint = 9,
                a_vault_lp = 10,
                b_vault_lp = 11,
                payer_token_a = 12,
                payer_token_b = 13,
                payer_pool_lp = 14,
                protocol_token_a_fee = 15,
                protocol_token_b_fee = 16,
                payer = 17,
                rent = 18,
                mint_metadata = 19,
                metadata_program = 20,
                vault_program = 21,
                token_program = 22,
                associated_token_program = 23,
                system_program = 24
            );
        }
        _ => {
            fill!(
                pool = 0,
                config = 1,
                lp_mint = 2,
                token_a_mint = 3,
                token_b_mint = 4,
                a_vault = 5,
                b_vault = 6,
                a_token_vault = 7,
                b_token_vault = 8,
                a_vault_lp_mint = 9,
                b_vault_lp_mint = 10,
                a_vault_lp = 11,
                b_vault_lp = 12,
                payer_token_a = 13,
                payer_token_b = 14,
                payer_pool_lp = 15,
                protocol_token_a_fee = 16,
                protocol_token_b_fee = 17,
                payer = 18,
                rent = 19,
                mint_metadata = 20,
                metadata_program = 21,
                vault_program = 22,
                token_program = 23,
                associated_token_program = 24,
                system_program = 25
            );
        }
    }
}

pub fn fill_pools_set_pool_fees_accounts(
    e: &mut MeteoraPoolsSetPoolFeesEvent,
    get: &AccountGetter<'_>,
) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.fee_operator == Pubkey::default() {
        e.fee_operator = get(1);
    }
}
