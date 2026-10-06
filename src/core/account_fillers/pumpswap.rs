//! PumpSwap 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 通用的 PumpSwap 交易账户填充宏
///
/// PumpSwap Buy/Sell instruction account mapping (based on IDL):
/// 0: pool
/// 1: user
/// 2: authority
/// 3: baseMint
/// 4: quoteMint
/// 5: userBaseTokenAccount
/// 6: userQuoteTokenAccount
/// 7: poolBaseTokenAccount
/// 8: poolQuoteTokenAccount
/// 9: protocolFeeRecipient
/// 10: protocolFeeRecipientTokenAccount
/// 11: baseTokenProgram
/// 12: quoteTokenProgram
/// ... (13-16 optional system/associated token accounts)
/// 17: coinCreatorVaultAta (optional)
/// 18: coinCreatorVaultAuthority (optional)
/// Upgrade remaining accounts:
/// - buy non-cashback: 23 pool_v2, 24 fee_recipient, 25 fee_recipient_quote_token_account
/// - buy cashback: 24 pool_v2, 25 fee_recipient, 26 fee_recipient_quote_token_account
/// - sell non-cashback: 21 pool_v2, 22 fee_recipient, 23 fee_recipient_quote_token_account
/// - sell cashback: 23 pool_v2, 24 fee_recipient, 25 fee_recipient_quote_token_account
macro_rules! fill_pumpswap_trade_common {
    ($event:expr, $get:expr) => {{
        let e = &mut *$event;
        let get = $get;

        if e.pool == Pubkey::default() {
            e.pool = get(0);
        }
        if e.user == Pubkey::default() {
            e.user = get(1);
        }
        if e.base_mint == Pubkey::default() {
            e.base_mint = get(3);
        }
        if e.quote_mint == Pubkey::default() {
            e.quote_mint = get(4);
        }
        if e.user_base_token_account == Pubkey::default() {
            e.user_base_token_account = get(5);
        }
        if e.user_quote_token_account == Pubkey::default() {
            e.user_quote_token_account = get(6);
        }
        if e.pool_base_token_account == Pubkey::default() {
            e.pool_base_token_account = get(7);
        }
        if e.pool_quote_token_account == Pubkey::default() {
            e.pool_quote_token_account = get(8);
        }
        if e.protocol_fee_recipient == Pubkey::default() {
            e.protocol_fee_recipient = get(9);
        }
        if e.protocol_fee_recipient_token_account == Pubkey::default() {
            e.protocol_fee_recipient_token_account = get(10);
        }
        if e.base_token_program == Pubkey::default() {
            e.base_token_program = get(11);
        }
        if e.quote_token_program == Pubkey::default() {
            e.quote_token_program = get(12);
        }
        if e.coin_creator_vault_ata == Pubkey::default() {
            e.coin_creator_vault_ata = get(17);
        }
        if e.coin_creator_vault_authority == Pubkey::default() {
            e.coin_creator_vault_authority = get(18);
        }
    }};
}

/// Count-aware filling preserves unresolved ALT slots instead of inferring length
/// from the last nonzero pubkey. Only supported account layouts are enriched.
pub fn fill_buy_accounts_with_count(
    e: &mut PumpSwapBuyEvent,
    get: &AccountGetter<'_>,
    count: usize,
) {
    fill_pumpswap_trade_common!(e, get);
    if !(23..=27).contains(&count) {
        return;
    }
    let mut accounts = [Pubkey::default(); 27];
    accounts[3] = e.base_mint;
    for index in 23..count {
        accounts[index] = get(index);
    }
    let (pool_v2, recipient, ata) =
        crate::instr::pump_amm::buy_upgrade_tail(&accounts[..count], e.base_mint);
    if e.pool_v2 == Pubkey::default() {
        e.pool_v2 = pool_v2;
    }
    if e.fee_recipient == Pubkey::default() {
        e.fee_recipient = recipient;
    }
    if e.fee_recipient_quote_token_account == Pubkey::default() {
        e.fee_recipient_quote_token_account = ata;
    }
}

pub fn fill_sell_accounts_with_count(
    e: &mut PumpSwapSellEvent,
    get: &AccountGetter<'_>,
    count: usize,
) {
    fill_pumpswap_trade_common!(e, get);
    if !(21..=26).contains(&count) {
        return;
    }
    let mut accounts = [Pubkey::default(); 26];
    accounts[1] = e.user;
    accounts[3] = e.base_mint;
    for index in 21..count {
        accounts[index] = get(index);
    }
    let (pool_v2, recipient, ata) =
        crate::instr::pump_amm::sell_upgrade_tail(&accounts[..count], e.base_mint);
    if e.pool_v2 == Pubkey::default() {
        e.pool_v2 = pool_v2;
    }
    if e.fee_recipient == Pubkey::default() {
        e.fee_recipient = recipient;
    }
    if e.fee_recipient_quote_token_account == Pubkey::default() {
        e.fee_recipient_quote_token_account = ata;
    }
}

/// Compatibility entry point for fully resolved accounts. For ALT or partial
/// account data, use fill_buy_accounts_with_count with the instruction count.
pub fn fill_buy_accounts(e: &mut PumpSwapBuyEvent, get: &AccountGetter<'_>) {
    let count = (23..=27)
        .rev()
        .find(|index| get(*index) != Pubkey::default())
        .map_or(23, |index| index + 1);
    fill_buy_accounts_with_count(e, get, count);
}

/// Compatibility entry point; use the count-aware variant for partial accounts.
pub fn fill_sell_accounts(e: &mut PumpSwapSellEvent, get: &AccountGetter<'_>) {
    let count = (21..=26)
        .rev()
        .find(|index| get(*index) != Pubkey::default())
        .map_or(21, |index| index + 1);
    fill_sell_accounts_with_count(e, get, count);
}

/// 填充由 `boost_buy_and_burn` 发出的 PumpSwap BuyEvent 账户
///
/// boost_buy_and_burn instruction account mapping (based on IDL):
/// 0: pool
/// 1: authority
/// 2: globalConfig
/// 3: baseMint
/// 4: quoteMint
/// 5: poolBaseTokenAccount
/// 6: poolQuoteTokenAccount
/// 7: boostVaultAuthority
/// 8: boostVault
/// 9: baseTokenProgram
/// 10: quoteTokenProgram
///
/// The buy layout must not be applied here: its slots 7/8 would read the boost
/// vault as pool vaults and 11/12 would read event_authority/program as token
/// programs. Missing user token and fee accounts have no counterpart and stay
/// unresolved; values already decoded from the event are preserved.
pub fn fill_boost_buy_and_burn_accounts(e: &mut PumpSwapBuyEvent, get: &AccountGetter<'_>) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(3);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(4);
    }
    if e.pool_base_token_account == Pubkey::default() {
        e.pool_base_token_account = get(5);
    }
    if e.pool_quote_token_account == Pubkey::default() {
        e.pool_quote_token_account = get(6);
    }
    if e.base_token_program == Pubkey::default() {
        e.base_token_program = get(9);
    }
    if e.quote_token_program == Pubkey::default() {
        e.quote_token_program = get(10);
    }
}

pub fn fill_trade_accounts(_e: &mut PumpSwapTradeEvent, _get: &AccountGetter<'_>) {
    // PumpSwapTradeEvent is a different event structure (from IDL TradeEvent)
    // It doesn't have the same account fields as Buy/Sell events
    // All its fields are already parsed from the event data, no need to fill from instruction accounts
}

/// 填充 PumpSwap CreatePool 事件账户
///
/// CreatePool instruction account mapping (based on IDL):
/// 0: pool
/// 1: globalConfig
/// 2: creator
/// 3: baseMint
/// 4: quoteMint
/// 5: lpMint
/// 6: userBaseTokenAccount
/// 7: userQuoteTokenAccount
pub fn fill_create_pool_accounts(e: &mut PumpSwapCreatePoolEvent, get: &AccountGetter<'_>) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.creator == Pubkey::default() {
        e.creator = get(2);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(3);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(4);
    }
    if e.lp_mint == Pubkey::default() {
        e.lp_mint = get(5);
    }
    if e.user_base_token_account == Pubkey::default() {
        e.user_base_token_account = get(6);
    }
    if e.user_quote_token_account == Pubkey::default() {
        e.user_quote_token_account = get(7);
    }
}

/// PumpSwap Liquidity Added 账户填充
///
/// deposit instruction account mapping (based on IDL):
/// 0: pool
/// 1: global_config
/// 2: user
/// 3: base_mint
/// 4: quote_mint
/// 5: lp_mint
/// 6: user_base_token_account
/// 7: user_quote_token_account
/// 8: user_pool_token_account
/// 9: pool_base_token_account
/// 10: pool_quote_token_account
/// 11: token_program
/// 12: token_2022_program
/// 13: event_authority
/// 14: program
pub fn fill_liquidity_added_accounts(_e: &mut PumpSwapLiquidityAdded, _get: &AccountGetter<'_>) {
    // 大部分字段已从事件数据解析
    // PumpSwapLiquidityAdded 事件结构不包含账户字段，只有数值字段
}

/// PumpSwap Liquidity Removed 账户填充
///
/// 注意：PumpSwap IDL 中没有明确的 removeLiquidity 指令
/// 此事件可能通过其他机制触发或暂未实现
pub fn fill_liquidity_removed_accounts(
    _e: &mut PumpSwapLiquidityRemoved,
    _get: &AccountGetter<'_>,
) {
    // 大部分字段已从事件数据解析
    // PumpSwapLiquidityRemoved 事件结构不包含账户字段，只有数值字段
}

#[cfg(test)]
mod upgrade_tests {
    use super::*;

    #[test]
    fn count_aware_pumpswap_tails_keep_missing_alt_slots_and_optional_pool() {
        for (buy, count, cashback, pool_index) in [
            (true, 25, false, None),
            (true, 26, true, None),
            (true, 26, false, Some(23)),
            (true, 27, true, Some(24)),
            (false, 23, false, None),
            (false, 25, true, None),
            (false, 24, false, Some(21)),
            (false, 26, true, Some(23)),
        ] {
            let mut accounts: Vec<_> = (0..count).map(|_| Pubkey::new_unique()).collect();
            let expected_pool = pool_index
                .map(|index| {
                    let pda = Pubkey::find_program_address(
                        &[b"pool-v2", accounts[3].as_ref()],
                        &crate::instr::pump_amm::PROGRAM_ID_PUBKEY,
                    )
                    .0;
                    accounts[index] = pda;
                    pda
                })
                .unwrap_or_default();
            if cashback && !buy {
                accounts[21] = Pubkey::find_program_address(
                    &[b"user_volume_accumulator", accounts[1].as_ref()],
                    &crate::instr::pump_amm::PROGRAM_ID_PUBKEY,
                )
                .0;
            }
            let recipient = accounts[count - 2];
            for missing_ata in [false, true] {
                let mut keys = accounts.clone();
                if missing_ata {
                    keys[count - 1] = Pubkey::default();
                }
                let get = |index: usize| keys.get(index).copied().unwrap_or_default();
                let (pool, actual_recipient, ata) = if buy {
                    let mut event = PumpSwapBuyEvent::default();
                    fill_buy_accounts_with_count(&mut event, &get, count);
                    (event.pool_v2, event.fee_recipient, event.fee_recipient_quote_token_account)
                } else {
                    let mut event = PumpSwapSellEvent::default();
                    fill_sell_accounts_with_count(&mut event, &get, count);
                    (event.pool_v2, event.fee_recipient, event.fee_recipient_quote_token_account)
                };
                assert_eq!(pool, expected_pool);
                assert_eq!(actual_recipient, recipient);
                assert_eq!(ata, keys[count - 1]);
            }
        }
    }

    #[test]
    fn unresolved_legacy_sell_cashback_prefix_stays_unknown() {
        let mut accounts: Vec<_> = (0..23).map(|_| Pubkey::new_unique()).collect();
        accounts[21] = Pubkey::default();
        let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
        let mut event = PumpSwapSellEvent::default();
        fill_sell_accounts_with_count(&mut event, &get, accounts.len());
        assert_eq!(event.fee_recipient, Pubkey::default());
        assert_eq!(event.fee_recipient_quote_token_account, Pubkey::default());
    }

    #[test]
    fn legacy_buy_pool_tail_and_unknown_extensions_do_not_become_fee_pairs() {
        let mut accounts: Vec<_> = (0..25).map(|_| Pubkey::new_unique()).collect();
        accounts[24] = Pubkey::find_program_address(
            &[b"pool-v2", accounts[3].as_ref()],
            &crate::instr::pump_amm::PROGRAM_ID_PUBKEY,
        )
        .0;
        let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
        let mut event = PumpSwapBuyEvent::default();
        fill_buy_accounts_with_count(&mut event, &get, accounts.len());
        assert_eq!(event.pool_v2, accounts[24]);
        assert_eq!(event.fee_recipient, Pubkey::default());
        assert_eq!(event.fee_recipient_quote_token_account, Pubkey::default());
        accounts.extend((0..3).map(|_| Pubkey::new_unique()));
        let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
        let mut event = PumpSwapBuyEvent::default();
        fill_buy_accounts_with_count(&mut event, &get, accounts.len());
        assert_eq!(event.fee_recipient, Pubkey::default());
    }

    #[test]
    fn fee_tail_is_not_misidentified_as_pool_v2() {
        let mint = Pubkey::new_unique();
        let expected = Pubkey::find_program_address(
            &[b"pool-v2", mint.as_ref()],
            &crate::instr::pump_amm::PROGRAM_ID_PUBKEY,
        )
        .0;
        for (buy, pool_index, tail_len) in
            [(true, 23, 26), (true, 24, 27), (false, 21, 24), (false, 23, 26)]
        {
            let mut accounts: Vec<_> = (0..tail_len).map(|_| Pubkey::new_unique()).collect();
            accounts[3] = mint;
            let parse_pool = |accounts: &[Pubkey]| {
                let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
                if buy {
                    let mut event = PumpSwapBuyEvent::default();
                    fill_buy_accounts(&mut event, &get);
                    event.pool_v2
                } else {
                    let mut event = PumpSwapSellEvent::default();
                    fill_sell_accounts(&mut event, &get);
                    event.pool_v2
                }
            };
            assert_eq!(parse_pool(&accounts), Pubkey::default());
            accounts[pool_index] = expected;
            assert_eq!(parse_pool(&accounts), expected);
        }
    }
}
