//! PumpFun 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 账户获取辅助函数类型
pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

#[inline(always)]
fn account_at_matches_mint(e: &PumpFunTradeEvent, get: &AccountGetter<'_>, idx: usize) -> bool {
    e.mint != Pubkey::default() && get(idx) == e.mint
}

#[inline(always)]
fn pump_trade_uses_v2_layout(e: &PumpFunTradeEvent, get: &AccountGetter<'_>) -> bool {
    matches!(e.ix_name.as_str(), "buy_v2" | "sell_v2" | "buy_exact_quote_in_v2")
        || account_at_matches_mint(e, get, 1)
}

#[inline(always)]
fn fill_create_v2_quote_accounts_if_appended(
    quote_mint_to: &mut Pubkey,
    quote_vault_to: &mut Pubkey,
    quote_token_program_to: &mut Pubkey,
    get: &AccountGetter<'_>,
) {
    let quote_mint = get(16);
    let quote_vault = get(17);
    let quote_token_program = get(18);
    if quote_mint == Pubkey::default()
        || quote_mint == crate::instr::program_ids::PUMPFUN_PROGRAM_ID
        || quote_vault == Pubkey::default()
        || quote_token_program == Pubkey::default()
    {
        return;
    }

    if *quote_mint_to == Pubkey::default() || is_pumpfun_solscan_sol_quote_mint(*quote_mint_to) {
        *quote_mint_to = normalize_pumpfun_quote_mint(quote_mint);
    }
    if *quote_vault_to == Pubkey::default() {
        *quote_vault_to = quote_vault;
    }
    if *quote_token_program_to == Pubkey::default() {
        *quote_token_program_to = quote_token_program;
    }
}

/// 填充 PumpFun Trade 事件账户
///
/// PumpFun Buy/Sell instruction account mapping (from pumpfun.json IDL):
/// Buy 共 16 个 IDL 账户，升级后追加 `bonding_curve_v2` 与新 fee/buyback recipient:
/// 0 global, 1 fee_recipient, 2 mint, 3 bonding_curve, 4 associated_bonding_curve, 5 associated_user, 6 user,
/// 7 system_program, 8 token_program, 9 creator_vault, 10 event_authority, 11 program,
/// 12 global_volume_accumulator, 13 user_volume_accumulator, 14 fee_config, 15 fee_program,
/// 16 bonding_curve_v2, 17 buyback_fee_recipient。
/// Sell 共 14 个 IDL 账户；升级后非 cashback: 14 bonding_curve_v2, 15 buyback_fee_recipient；
/// cashback: 14 user_volume_accumulator, 15 bonding_curve_v2, 16 buyback_fee_recipient。
///
/// Pump v2 (`buy_v2` / `buy_exact_quote_in_v2` / `sell_v2`) 使用不同账户布局：
/// mint=#1, token_program=#3, fee_recipient=#6, bonding_curve=#10, user=#13, creator_vault=#16。
/// 部分日志/合并路径会把 `buy_exact_quote_in_v2` 暴露成短名 `buy_exact_quote_in`，因此需要用
/// 实际账户形状确认 v2，避免把 Token-2022 mint 当 legacy SPL Token 处理。
pub fn fill_trade_accounts(e: &mut PumpFunTradeEvent, get: &AccountGetter<'_>) {
    let is_v2 = pump_trade_uses_v2_layout(e, get);
    let is_sell = matches!(e.ix_name.as_str(), "sell" | "sell_v2") || !e.is_buy;

    let fill_pk = |to: &mut Pubkey, idx: usize| {
        if *to == Pubkey::default() {
            let from = get(idx);
            if from != Pubkey::default() {
                *to = from;
            }
        }
    };
    let fill_pumpfun_quote_mint = |to: &mut Pubkey, idx: usize| {
        if *to == Pubkey::default() || is_pumpfun_solscan_sol_quote_mint(*to) {
            let from = normalize_pumpfun_quote_mint(get(idx));
            if from != Pubkey::default() {
                *to = from;
            }
        }
    };

    if is_v2 {
        fill_pk(&mut e.global, 0);
        fill_pumpfun_quote_mint(&mut e.quote_mint, 2);
        fill_pk(&mut e.fee_recipient, 6);
        fill_pk(&mut e.bonding_curve, 10);
        // v2 has explicit base/quote bonding curve accounts; no separate legacy bonding_curve_v2 remaining account.
        fill_pk(&mut e.associated_bonding_curve, 11);
        fill_pk(&mut e.associated_quote_bonding_curve, 12);
        fill_pk(&mut e.associated_user, 14);
        fill_pk(&mut e.associated_quote_user, 15);
        fill_pk(&mut e.user, 13);
        fill_pk(&mut e.system_program, if is_sell { 23 } else { 24 });
        fill_pk(&mut e.token_program, 3);
        fill_pk(&mut e.quote_token_program, 4);
        fill_pk(&mut e.associated_token_program, 5);
        fill_pk(&mut e.creator_vault, 16);
        fill_pk(&mut e.associated_quote_fee_recipient, 7);
        fill_pk(&mut e.buyback_fee_recipient, 8);
        fill_pk(&mut e.associated_quote_buyback_fee_recipient, 9);
        fill_pk(&mut e.associated_creator_vault, 17);
        fill_pk(&mut e.sharing_config, 18);
        fill_pk(&mut e.event_authority, if is_sell { 24 } else { 25 });
        fill_pk(&mut e.program, if is_sell { 25 } else { 26 });
        if !is_sell {
            fill_pk(&mut e.global_volume_accumulator, 19);
            fill_pk(&mut e.user_volume_accumulator, 20);
            fill_pk(&mut e.associated_user_volume_accumulator, 21);
            fill_pk(&mut e.fee_config, 22);
            fill_pk(&mut e.fee_program, 23);
        } else {
            fill_pk(&mut e.user_volume_accumulator, 19);
            fill_pk(&mut e.associated_user_volume_accumulator, 20);
            fill_pk(&mut e.fee_config, 21);
            fill_pk(&mut e.fee_program, 22);
        }
        return;
    }

    fill_pk(&mut e.global, 0);
    e.quote_mint = normalize_pumpfun_quote_mint(e.quote_mint);
    // 指令账户 #1 = fee_recipient（IDL）；仅日志路径时常为 default，补全后可与 mayhem/普通池一致，供 sol-trade-sdk 校验。
    fill_pk(&mut e.fee_recipient, 1);
    fill_pk(&mut e.bonding_curve, 3);
    fill_pk(&mut e.associated_bonding_curve, 4);
    fill_pk(&mut e.associated_user, 5);
    fill_pk(&mut e.user, 6);
    fill_pk(&mut e.system_program, 7);
    fill_pk(&mut e.creator_vault, if e.is_buy { 9 } else { 8 });
    fill_pk(&mut e.token_program, if e.is_buy { 8 } else { 9 });
    fill_pk(&mut e.event_authority, 10);
    fill_pk(&mut e.program, 11);
    if e.is_buy {
        fill_pk(&mut e.global_volume_accumulator, 12);
        fill_pk(&mut e.user_volume_accumulator, 13);
        fill_pk(&mut e.fee_config, 14);
        fill_pk(&mut e.fee_program, 15);
        fill_pk(&mut e.bonding_curve_v2, 16);
        fill_pk(&mut e.buyback_fee_recipient, 17);
        let a18 = get(17);
        if e.account.is_none() && a18 != Pubkey::default() {
            e.account = Some(a18);
        }
    } else {
        fill_pk(&mut e.fee_config, 12);
        fill_pk(&mut e.fee_program, 13);
        let a14 = get(14);
        let a15 = get(15);
        let a16 = get(16);
        if a16 != Pubkey::default() {
            fill_pk(&mut e.user_volume_accumulator, 14);
            fill_pk(&mut e.bonding_curve_v2, 15);
            fill_pk(&mut e.buyback_fee_recipient, 16);
            if e.account.is_none() {
                e.account = Some(a16);
            }
        } else if e.is_cashback_coin {
            if a14 != Pubkey::default() {
                fill_pk(&mut e.user_volume_accumulator, 14);
            }
            if a15 != Pubkey::default() {
                fill_pk(&mut e.bonding_curve_v2, 15);
            }
        } else {
            if a14 != Pubkey::default() {
                fill_pk(&mut e.bonding_curve_v2, 14);
            }
            if a15 != Pubkey::default() {
                fill_pk(&mut e.buyback_fee_recipient, 15);
                if e.account.is_none() {
                    e.account = Some(a15);
                }
            }
        }
    }
}

/// 填充 PumpFun Create 事件账户
///
/// PumpFun Create instruction account mapping (based on IDL):
/// 0: mint
/// 1: mintAuthority
/// 2: bondingCurve
/// 3: associatedBondingCurve
/// 4: global
/// 5: mplTokenMetadata
/// 6: metadata
/// 7: user
/// 8: systemProgram
/// 9: tokenProgram
/// 10: associatedTokenProgram
/// 11: rent
/// 12: eventAuthority
/// 13: program
pub fn fill_create_accounts(e: &mut PumpFunCreateTokenEvent, get: &AccountGetter<'_>) {
    if e.mint == Pubkey::default() {
        e.mint = get(0);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(2);
    }
    if e.user == Pubkey::default() {
        e.user = get(7);
    }
    if e.mint_authority == Pubkey::default() {
        e.mint_authority = get(1);
    }
    if e.associated_bonding_curve == Pubkey::default() {
        e.associated_bonding_curve = get(3);
    }
    if e.global == Pubkey::default() {
        e.global = get(4);
    }
    if e.system_program == Pubkey::default() {
        e.system_program = get(8);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(9);
    }
    if e.associated_token_program == Pubkey::default() {
        e.associated_token_program = get(10);
    }
    if e.event_authority == Pubkey::default() {
        e.event_authority = get(12);
    }
    if e.program == Pubkey::default() {
        e.program = get(13);
    }
}

/// Fill canonical PumpFun Create from create_v2 instruction accounts.
///
/// CreateV2 instruction (idl create_v2): 0 mint, 1 mint_authority, 2 bonding_curve,
/// 3 associated_bonding_curve, 4 global, 5 user, 6 system_program, 7 token_program,
/// 8 associated_token_program, 9 mayhem_program_id, 10 global_params, 11 sol_vault,
/// 12 mayhem_state, 13 mayhem_token_vault, 14 event_authority, 15 program.
/// Quote-pool variant appends: 16 quote_mint, 17 quote_vault, 18 quote_token_program.
pub fn fill_create_accounts_from_v2(e: &mut PumpFunCreateTokenEvent, get: &AccountGetter<'_>) {
    if e.mint == Pubkey::default() {
        e.mint = get(0);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(2);
    }
    if e.user == Pubkey::default() {
        e.user = get(5);
    }
    if e.mint_authority == Pubkey::default() {
        e.mint_authority = get(1);
    }
    if e.associated_bonding_curve == Pubkey::default() {
        e.associated_bonding_curve = get(3);
    }
    if e.global == Pubkey::default() {
        e.global = get(4);
    }
    if e.system_program == Pubkey::default() {
        e.system_program = get(6);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(7);
    }
    if e.associated_token_program == Pubkey::default() {
        e.associated_token_program = get(8);
    }
    if e.mayhem_program_id == Pubkey::default() {
        e.mayhem_program_id = get(9);
    }
    if e.global_params == Pubkey::default() {
        e.global_params = get(10);
    }
    if e.sol_vault == Pubkey::default() {
        e.sol_vault = get(11);
    }
    if e.mayhem_state == Pubkey::default() {
        e.mayhem_state = get(12);
    }
    if e.mayhem_token_vault == Pubkey::default() {
        e.mayhem_token_vault = get(13);
    }
    if e.event_authority == Pubkey::default() {
        e.event_authority = get(14);
    }
    if e.program == Pubkey::default() {
        e.program = get(15);
    }
    fill_create_v2_quote_accounts_if_appended(
        &mut e.quote_mint,
        &mut e.quote_vault,
        &mut e.quote_token_program,
        get,
    );
}

/// 填充 PumpFun CreateV2 事件账户
///
/// CreateV2 instruction (idl create_v2): 0 mint, 1 mint_authority, 2 bonding_curve,
/// 3 associated_bonding_curve, 4 global, 5 user, 6 system_program, 7 token_program,
/// 8 associated_token_program, 9 mayhem_program_id, 10 global_params, 11 sol_vault,
/// 12 mayhem_state, 13 mayhem_token_vault, 14 event_authority, 15 program.
/// Quote-pool variant appends: 16 quote_mint, 17 quote_vault, 18 quote_token_program.
pub fn fill_create_v2_accounts(e: &mut PumpFunCreateV2TokenEvent, get: &AccountGetter<'_>) {
    if e.mint == Pubkey::default() {
        e.mint = get(0);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(2);
    }
    if e.user == Pubkey::default() {
        e.user = get(5);
    }
    if e.mint_authority == Pubkey::default() {
        e.mint_authority = get(1);
    }
    if e.associated_bonding_curve == Pubkey::default() {
        e.associated_bonding_curve = get(3);
    }
    if e.global == Pubkey::default() {
        e.global = get(4);
    }
    if e.system_program == Pubkey::default() {
        e.system_program = get(6);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(7);
    }
    if e.associated_token_program == Pubkey::default() {
        e.associated_token_program = get(8);
    }
    if e.mayhem_program_id == Pubkey::default() {
        e.mayhem_program_id = get(9);
    }
    if e.global_params == Pubkey::default() {
        e.global_params = get(10);
    }
    if e.sol_vault == Pubkey::default() {
        e.sol_vault = get(11);
    }
    if e.mayhem_state == Pubkey::default() {
        e.mayhem_state = get(12);
    }
    if e.mayhem_token_vault == Pubkey::default() {
        e.mayhem_token_vault = get(13);
    }
    if e.event_authority == Pubkey::default() {
        e.event_authority = get(14);
    }
    if e.program == Pubkey::default() {
        e.program = get(15);
    }
    fill_create_v2_quote_accounts_if_appended(
        &mut e.quote_mint,
        &mut e.quote_vault,
        &mut e.quote_token_program,
        get,
    );
}

/// 填充 PumpFun Migrate 事件账户
pub fn fill_migrate_accounts(_e: &mut PumpFunMigrateEvent, _get: &AccountGetter<'_>) {
    // 暂未实现 - 需要 IDL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_trade_accounts_sets_legacy_buy_upgrade_accounts() {
        let bonding_curve_v2 = Pubkey::new_from_array([16u8; 32]);
        let buyback_fee_recipient = Pubkey::new_from_array([17u8; 32]);
        let get = |i: usize| -> Pubkey {
            match i {
                16 => bonding_curve_v2,
                17 => buyback_fee_recipient,
                _ => Pubkey::default(),
            }
        };

        let mut e = PumpFunTradeEvent { is_buy: true, account: None, ..Default::default() };

        fill_trade_accounts(&mut e, &get);
        assert_eq!(e.bonding_curve_v2, bonding_curve_v2);
        assert_eq!(e.buyback_fee_recipient, buyback_fee_recipient);
        assert_eq!(e.account, Some(buyback_fee_recipient));
    }

    #[test]
    fn fill_trade_accounts_sets_cashback_sell_upgrade_accounts() {
        let user_volume = Pubkey::new_from_array([14u8; 32]);
        let bonding_curve_v2 = Pubkey::new_from_array([15u8; 32]);
        let buyback_fee_recipient = Pubkey::new_from_array([16u8; 32]);
        let get = |i: usize| -> Pubkey {
            match i {
                14 => user_volume,
                15 => bonding_curve_v2,
                16 => buyback_fee_recipient,
                _ => Pubkey::default(),
            }
        };

        let mut e = PumpFunTradeEvent { is_buy: false, account: None, ..Default::default() };

        fill_trade_accounts(&mut e, &get);
        assert_eq!(e.user_volume_accumulator, user_volume);
        assert_eq!(e.bonding_curve_v2, bonding_curve_v2);
        assert_eq!(e.buyback_fee_recipient, buyback_fee_recipient);
        assert_eq!(e.account, Some(buyback_fee_recipient));
    }

    #[test]
    fn fill_trade_accounts_sets_fee_recipient_from_ix_account_1() {
        let fee = Pubkey::new_from_array([42u8; 32]);
        let get = |i: usize| -> Pubkey {
            if i == 1 {
                fee
            } else {
                Pubkey::default()
            }
        };
        let mut e = PumpFunTradeEvent { fee_recipient: Pubkey::default(), ..Default::default() };
        fill_trade_accounts(&mut e, &get);
        assert_eq!(e.fee_recipient, fee);
    }

    #[test]
    fn fill_create_accounts_from_v2_sets_appended_quote_accounts_only_when_tail_exists() {
        let quote_mint = PUMPFUN_WSOL_QUOTE_MINT;
        let quote_vault = Pubkey::new_from_array([17u8; 32]);
        let quote_token_program = Pubkey::new_from_array([18u8; 32]);
        let get_with_tail = |i: usize| -> Pubkey {
            match i {
                16 => quote_mint,
                17 => quote_vault,
                18 => quote_token_program,
                _ => Pubkey::default(),
            }
        };
        let mut with_tail = PumpFunCreateTokenEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        };

        fill_create_accounts_from_v2(&mut with_tail, &get_with_tail);
        assert_eq!(with_tail.quote_mint, quote_mint);
        assert_eq!(with_tail.quote_vault, quote_vault);
        assert_eq!(with_tail.quote_token_program, quote_token_program);

        let get_without_tail = |i: usize| -> Pubkey {
            match i {
                16 => quote_mint,
                _ => Pubkey::default(),
            }
        };
        let mut without_tail = PumpFunCreateTokenEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        };

        fill_create_accounts_from_v2(&mut without_tail, &get_without_tail);
        assert_eq!(without_tail.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
        assert_eq!(without_tail.quote_vault, Pubkey::default());
        assert_eq!(without_tail.quote_token_program, Pubkey::default());
    }

    #[test]
    fn fill_create_accounts_from_v2_does_not_fill_partial_quote_tail() {
        let quote_vault = Pubkey::new_from_array([17u8; 32]);
        let quote_token_program = Pubkey::new_from_array([18u8; 32]);
        let get_missing_quote_mint = |i: usize| -> Pubkey {
            match i {
                17 => quote_vault,
                18 => quote_token_program,
                _ => Pubkey::default(),
            }
        };
        let mut e = PumpFunCreateTokenEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        };

        fill_create_accounts_from_v2(&mut e, &get_missing_quote_mint);

        assert_eq!(e.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
        assert_eq!(e.quote_vault, Pubkey::default());
        assert_eq!(e.quote_token_program, Pubkey::default());

        let quote_mint = PUMPFUN_WSOL_QUOTE_MINT;
        let get_missing_quote_vault = |i: usize| -> Pubkey {
            match i {
                16 => quote_mint,
                18 => quote_token_program,
                _ => Pubkey::default(),
            }
        };
        let mut e = PumpFunCreateTokenEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        };

        fill_create_accounts_from_v2(&mut e, &get_missing_quote_vault);

        assert_eq!(e.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
        assert_eq!(e.quote_vault, Pubkey::default());
        assert_eq!(e.quote_token_program, Pubkey::default());
    }

    #[test]
    fn fill_trade_accounts_uses_v2_indices_for_short_buy_exact_quote_in() {
        let mint = Pubkey::new_from_array([1u8; 32]);
        let quote_mint = Pubkey::new_from_array([2u8; 32]);
        let token_program = Pubkey::new_from_array([3u8; 32]);
        let fee_recipient = Pubkey::new_from_array([6u8; 32]);
        let bonding_curve = Pubkey::new_from_array([10u8; 32]);
        let associated_bonding_curve = Pubkey::new_from_array([11u8; 32]);
        let user = Pubkey::new_from_array([13u8; 32]);
        let creator_vault = Pubkey::new_from_array([16u8; 32]);
        let legacy_wrong_creator_vault = Pubkey::new_from_array([9u8; 32]);
        let get = |i: usize| -> Pubkey {
            match i {
                1 => mint,
                2 => quote_mint,
                3 => token_program,
                6 => fee_recipient,
                9 => legacy_wrong_creator_vault,
                10 => bonding_curve,
                11 => associated_bonding_curve,
                13 => user,
                16 => creator_vault,
                _ => Pubkey::default(),
            }
        };
        let mut e = PumpFunTradeEvent {
            mint,
            is_buy: true,
            ix_name: "buy_exact_quote_in".to_string(),
            ..Default::default()
        };

        fill_trade_accounts(&mut e, &get);

        assert_eq!(e.quote_mint, quote_mint);
        assert_eq!(e.token_program, token_program);
        assert_eq!(e.fee_recipient, fee_recipient);
        assert_eq!(e.bonding_curve, bonding_curve);
        assert_eq!(e.associated_bonding_curve, associated_bonding_curve);
        assert_eq!(e.user, user);
        assert_eq!(e.creator_vault, creator_vault);
        assert_ne!(e.creator_vault, legacy_wrong_creator_vault);
    }

    #[test]
    fn fill_trade_accounts_keeps_legacy_layout_for_legacy_shaped_short_buy_exact_quote_in() {
        let mint = Pubkey::new_from_array([2u8; 32]);
        let token_program = Pubkey::new_from_array([8u8; 32]);
        let creator_vault = Pubkey::new_from_array([9u8; 32]);
        let get = |i: usize| -> Pubkey {
            match i {
                2 => mint,
                8 => token_program,
                9 => creator_vault,
                _ => Pubkey::default(),
            }
        };
        let mut e = PumpFunTradeEvent {
            mint,
            is_buy: true,
            ix_name: "buy_exact_quote_in".to_string(),
            ..Default::default()
        };

        fill_trade_accounts(&mut e, &get);

        assert_eq!(e.token_program, token_program);
        assert_eq!(e.creator_vault, creator_vault);
    }
}
