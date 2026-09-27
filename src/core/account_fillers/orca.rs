//! Orca Whirlpool 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

#[inline]
fn fill_if_default(to: &mut Pubkey, from: Pubkey) {
    if *to == Pubkey::default() && from != Pubkey::default() {
        *to = from;
    }
}

/// Orca Whirlpool Swap 账户填充
///
/// swap (v1) account mapping (IDL):
/// 0: tokenProgram, 1: tokenAuthority, 2: whirlpool,
/// 3: tokenOwnerAccountA, 4: tokenVaultA, 5: tokenOwnerAccountB, 6: tokenVaultB,
/// 7: tickArray0, 8: tickArray1, 9: tickArray2, 10: oracle
///
/// swap_v2 account mapping (IDL):
/// 0: tokenProgramA, 1: tokenProgramB, 2: memoProgram, 3: tokenAuthority, 4: whirlpool,
/// 5: tokenMintA, 6: tokenMintB, 7: tokenOwnerAccountA, 8: tokenVaultA,
/// 9: tokenOwnerAccountB, 10: tokenVaultB, 11: tickArray0, 12: tickArray1, 13: tickArray2,
/// 14: oracle
pub fn fill_whirlpool_swap_accounts(e: &mut OrcaWhirlpoolSwapEvent, get: &AccountGetter<'_>) {
    /// Official Memo program — present at index 2 on `swap_v2`.
    const MEMO_PROGRAM: Pubkey =
        solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

    let is_v2 = get(2) == MEMO_PROGRAM
        || (e.whirlpool != Pubkey::default() && get(4) == e.whirlpool);
    if is_v2 {
        if e.whirlpool == Pubkey::default() {
            e.whirlpool = get(4);
        }
        fill_if_default(&mut e.token_program_a, get(0));
        fill_if_default(&mut e.token_program_b, get(1));
        fill_if_default(&mut e.token_mint_a, get(5));
        fill_if_default(&mut e.token_mint_b, get(6));
        fill_if_default(&mut e.token_vault_a, get(8));
        fill_if_default(&mut e.token_vault_b, get(10));
        fill_if_default(&mut e.tick_array_0, get(11));
        fill_if_default(&mut e.tick_array_1, get(12));
        fill_if_default(&mut e.tick_array_2, get(13));
        fill_if_default(&mut e.oracle, get(14));
    } else {
        if e.whirlpool == Pubkey::default() {
            e.whirlpool = get(2);
        }
        // v1: single token program for both sides; mints are not in the account list.
        let tp = get(0);
        fill_if_default(&mut e.token_program_a, tp);
        fill_if_default(&mut e.token_program_b, tp);
        fill_if_default(&mut e.token_vault_a, get(4));
        fill_if_default(&mut e.token_vault_b, get(6));
        fill_if_default(&mut e.tick_array_0, get(7));
        fill_if_default(&mut e.tick_array_1, get(8));
        fill_if_default(&mut e.tick_array_2, get(9));
        fill_if_default(&mut e.oracle, get(10));
    }
}

/// Orca Whirlpool Liquidity Increased 账户填充
///
/// increaseLiquidity instruction account mapping (based on IDL):
/// 0: whirlpool
/// 1: tokenProgram
/// 2: positionAuthority
/// 3: position
/// 4: positionTokenAccount
/// 5: tokenOwnerAccountA
/// 6: tokenOwnerAccountB
/// 7: tokenVaultA
/// 8: tokenVaultB
/// 9: tickArrayLower
/// 10: tickArrayUpper
pub fn fill_whirlpool_liquidity_increased_accounts(
    e: &mut OrcaWhirlpoolLiquidityIncreasedEvent,
    get: &AccountGetter<'_>,
) {
    if e.position == Pubkey::default() {
        e.position = get(3);
    }
}

/// Orca Whirlpool Liquidity Decreased 账户填充
///
/// decreaseLiquidity instruction account mapping (based on IDL):
/// 0: whirlpool
/// 1: tokenProgram
/// 2: positionAuthority
/// 3: position
/// 4: positionTokenAccount
/// 5: tokenOwnerAccountA
/// 6: tokenOwnerAccountB
/// 7: tokenVaultA
/// 8: tokenVaultB
/// 9: tickArrayLower
/// 10: tickArrayUpper
pub fn fill_whirlpool_liquidity_decreased_accounts(
    e: &mut OrcaWhirlpoolLiquidityDecreasedEvent,
    get: &AccountGetter<'_>,
) {
    if e.position == Pubkey::default() {
        e.position = get(3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_swap_v2_tick_arrays() {
        let memo = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        let mut accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        accounts[2] = memo;
        let mut e = OrcaWhirlpoolSwapEvent::default();
        fill_whirlpool_swap_accounts(&mut e, &|i| accounts.get(i).copied().unwrap_or_default());
        assert_eq!(e.whirlpool, accounts[4]);
        assert_eq!(e.token_program_a, accounts[0]);
        assert_eq!(e.token_mint_a, accounts[5]);
        assert_eq!(e.token_vault_a, accounts[8]);
        assert_eq!(e.tick_array_0, accounts[11]);
        assert_eq!(e.tick_array_2, accounts[13]);
        assert_eq!(e.oracle, accounts[14]);
    }

    #[test]
    fn fill_swap_v1_tick_arrays() {
        let accounts: Vec<_> = (0..11).map(|_| Pubkey::new_unique()).collect();
        let mut e = OrcaWhirlpoolSwapEvent {
            whirlpool: accounts[2],
            ..Default::default()
        };
        fill_whirlpool_swap_accounts(&mut e, &|i| accounts.get(i).copied().unwrap_or_default());
        assert_eq!(e.token_vault_a, accounts[4]);
        assert_eq!(e.tick_array_0, accounts[7]);
        assert_eq!(e.oracle, accounts[10]);
        assert_eq!(e.token_mint_a, Pubkey::default());
    }
}
