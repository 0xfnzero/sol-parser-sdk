//! Raydium LaunchLab 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// Fills account context for a Raydium LaunchLab trade event.
///
/// Raydium LaunchLab trade instruction account mapping:
/// 0: payer, 2: global_config, 3: platform_config, 4: pool_state,
/// 5-8: user token accounts and vaults, 9-12: mints and token programs.
pub fn fill_trade_accounts(e: &mut RaydiumLaunchlabTradeEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(4);
    }
    if e.global_config == Pubkey::default() {
        e.global_config = get(2);
    }
    if e.platform_config == Pubkey::default() {
        e.platform_config = get(3);
    }
    if e.user_base_token == Pubkey::default() {
        e.user_base_token = get(5);
    }
    if e.user_quote_token == Pubkey::default() {
        e.user_quote_token = get(6);
    }
    if e.base_vault == Pubkey::default() {
        e.base_vault = get(7);
    }
    if e.quote_vault == Pubkey::default() {
        e.quote_vault = get(8);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(9);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(10);
    }
    if e.base_token_program == Pubkey::default() {
        e.base_token_program = get(11);
    }
    if e.quote_token_program == Pubkey::default() {
        e.quote_token_program = get(12);
    }
}

/// Fills account context for a Raydium LaunchLab pool-create event.
///
/// Raydium LaunchLab initialize instruction account mapping:
/// All current initialize variants share indices 0-9. `initialize` and
/// `initialize_v2` use token-program indices 11-12, while
/// `initialize_with_token_2022` uses indices 10-11.
pub fn fill_pool_create_accounts(e: &mut RaydiumLaunchlabPoolCreateEvent, get: &AccountGetter<'_>) {
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(5);
    }
    if e.creator == Pubkey::default() {
        e.creator = get(1);
    }
    if e.payer == Pubkey::default() {
        e.payer = get(0);
    }
    if e.global_config == Pubkey::default() {
        e.global_config = get(2);
    }
    if e.platform_config == Pubkey::default() {
        e.platform_config = get(3);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(6);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(7);
    }
    if e.base_vault == Pubkey::default() {
        e.base_vault = get(8);
    }
    if e.quote_vault == Pubkey::default() {
        e.quote_vault = get(9);
    }
    let token_program_indices = if get(17) == crate::instr::raydium_launchlab::PROGRAM_ID_PUBKEY {
        (11, 12)
    } else if get(14) == crate::instr::raydium_launchlab::PROGRAM_ID_PUBKEY {
        (10, 11)
    } else {
        return;
    };
    if e.base_token_program == Pubkey::default() {
        e.base_token_program = get(token_program_indices.0);
    }
    if e.quote_token_program == Pubkey::default() {
        e.quote_token_program = get(token_program_indices.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> RaydiumLaunchlabPoolCreateEvent {
        RaydiumLaunchlabPoolCreateEvent {
            metadata: EventMetadata::default(),
            base_mint_param: BaseMintParam {
                symbol: String::new(),
                name: String::new(),
                uri: String::new(),
                decimals: 0,
            },
            pool_state: Pubkey::default(),
            payer: Pubkey::default(),
            creator: Pubkey::default(),
            global_config: Pubkey::default(),
            platform_config: Pubkey::default(),
            base_mint: Pubkey::default(),
            quote_mint: Pubkey::default(),
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            base_token_program: Pubkey::default(),
            quote_token_program: Pubkey::default(),
        }
    }

    #[test]
    fn pool_create_filler_handles_standard_initialize_layout() {
        let mut accounts: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
        accounts[17] = crate::instr::raydium_launchlab::PROGRAM_ID_PUBKEY;
        let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
        let mut event = event();

        fill_pool_create_accounts(&mut event, &get);

        assert_eq!(event.base_token_program, accounts[11]);
        assert_eq!(event.quote_token_program, accounts[12]);
    }

    #[test]
    fn pool_create_filler_handles_token_2022_initialize_layout() {
        let mut accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        accounts[14] = crate::instr::raydium_launchlab::PROGRAM_ID_PUBKEY;
        let get = |index: usize| accounts.get(index).copied().unwrap_or_default();
        let mut event = event();

        fill_pool_create_accounts(&mut event, &get);

        assert_eq!(event.base_token_program, accounts[10]);
        assert_eq!(event.quote_token_program, accounts[11]);
    }
}
