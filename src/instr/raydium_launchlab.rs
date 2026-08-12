//! Raydium LaunchLab 指令解析器
//!
//! 底层按 `idls/raydium_launchpad.json` 的真实 instruction discriminator
//! 和账户布局解析，对外事件名统一为 `RaydiumLaunchlab*`。

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium LaunchLab instruction discriminators from `idls/raydium_launchpad.json`.
pub mod discriminators {
    pub const BUY_EXACT_IN: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];
    pub const BUY_EXACT_OUT: [u8; 8] = [24, 211, 116, 40, 105, 3, 153, 56];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];
    pub const INITIALIZE_WITH_TOKEN_2022: [u8; 8] = [37, 190, 126, 222, 44, 154, 171, 17];
    pub const MIGRATE_TO_AMM: [u8; 8] = [207, 82, 192, 145, 254, 207, 145, 223];
    pub const MIGRATE_TO_CPSWAP: [u8; 8] = [136, 92, 200, 103, 28, 218, 144, 140];
    pub const SELL_EXACT_IN: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];
    pub const SELL_EXACT_OUT: [u8; 8] = [95, 200, 71, 34, 8, 9, 11, 166];
}

/// Raydium LaunchLab 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID;

/// 主要的 Raydium LaunchLab 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::BUY_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            true,
        ),
        discriminators::BUY_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            false,
        ),
        discriminators::SELL_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            true,
        ),
        discriminators::SELL_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            false,
        ),
        discriminators::INITIALIZE | discriminators::INITIALIZE_V2 => {
            parse_pool_create_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                (11, 12),
            )
        }
        discriminators::INITIALIZE_WITH_TOKEN_2022 => parse_pool_create_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            (10, 11),
        ),
        // The LaunchLab IDL does not expose enough fields to synthesize a
        // migration event with the SDK's migrate layout.
        discriminators::MIGRATE_TO_AMM | discriminators::MIGRATE_TO_CPSWAP => None,
        _ => None,
    }
}

/// Parses a buy or sell instruction.
///
/// Outer instructions contain user limits; log events remain authoritative for executed amounts.
fn parse_trade_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    is_buy: bool,
    exact_in: bool,
) -> Option<DexEvent> {
    let first_amount = read_u64_le(data, 0)?;
    let second_amount = read_u64_le(data, 8)?;

    let (amount_in, amount_out) =
        if exact_in { (first_amount, second_amount) } else { (second_amount, first_amount) };

    let pool_state = get_account(accounts, 4)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
        metadata,
        pool_state,
        user: get_account(accounts, 0).unwrap_or_default(),
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
        global_config: get_account(accounts, 2).unwrap_or_default(),
        platform_config: get_account(accounts, 3).unwrap_or_default(),
        user_base_token: get_account(accounts, 5).unwrap_or_default(),
        user_quote_token: get_account(accounts, 6).unwrap_or_default(),
        base_vault: get_account(accounts, 7).unwrap_or_default(),
        quote_vault: get_account(accounts, 8).unwrap_or_default(),
        base_mint: get_account(accounts, 9).unwrap_or_default(),
        quote_mint: get_account(accounts, 10).unwrap_or_default(),
        base_token_program: get_account(accounts, 11).unwrap_or_default(),
        quote_token_program: get_account(accounts, 12).unwrap_or_default(),
    }))
}

/// Parses an initialize variant using its current IDL token-program indices.
fn parse_pool_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    token_program_indices: (usize, usize),
) -> Option<DexEvent> {
    let base_mint_param = parse_mint_params(data)?;

    let pool_state = get_account(accounts, 5)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent {
        metadata,
        base_mint_param,
        pool_state,
        payer: get_account(accounts, 0).unwrap_or_default(),
        creator: get_account(accounts, 1).unwrap_or_default(),
        global_config: get_account(accounts, 2).unwrap_or_default(),
        platform_config: get_account(accounts, 3).unwrap_or_default(),
        base_mint: get_account(accounts, 6).unwrap_or_default(),
        quote_mint: get_account(accounts, 7).unwrap_or_default(),
        base_vault: get_account(accounts, 8).unwrap_or_default(),
        quote_vault: get_account(accounts, 9).unwrap_or_default(),
        base_token_program: get_account(accounts, token_program_indices.0).unwrap_or_default(),
        quote_token_program: get_account(accounts, token_program_indices.1).unwrap_or_default(),
    }))
}

fn parse_mint_params(data: &[u8]) -> Option<BaseMintParam> {
    let mut offset = 0usize;
    let decimals = *data.get(offset)?;
    offset += 1;
    let name = read_borsh_string(data, &mut offset)?;
    let symbol = read_borsh_string(data, &mut offset)?;
    let uri = read_borsh_string(data, &mut offset)?;
    Some(BaseMintParam { symbol, name, uri, decimals })
}

fn read_borsh_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    let end = (*offset).checked_add(len)?;
    let bytes = data.get(*offset..end)?;
    *offset = end;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trade_data(discriminator: [u8; 8]) -> Vec<u8> {
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&1_000u64.to_le_bytes());
        data.extend_from_slice(&900u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        data
    }

    fn initialize_data(discriminator: [u8; 8]) -> Vec<u8> {
        let mut data = discriminator.to_vec();
        data.push(6);
        for value in ["USD1 Token", "U1", "https://example.invalid/u1.json"] {
            data.extend_from_slice(&(value.len() as u32).to_le_bytes());
            data.extend_from_slice(value.as_bytes());
        }
        data
    }

    #[test]
    fn trade_instruction_exposes_current_idl_account_context() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();

        for discriminator in [
            discriminators::BUY_EXACT_IN,
            discriminators::BUY_EXACT_OUT,
            discriminators::SELL_EXACT_IN,
            discriminators::SELL_EXACT_OUT,
        ] {
            let event = parse_instruction(
                &trade_data(discriminator),
                &accounts,
                Signature::default(),
                1,
                2,
                Some(3),
            )
            .expect("trade event");
            let DexEvent::RaydiumLaunchlabTrade(event) = event else {
                panic!("expected LaunchLab trade");
            };

            assert_eq!(event.user, accounts[0]);
            assert_eq!(event.global_config, accounts[2]);
            assert_eq!(event.platform_config, accounts[3]);
            assert_eq!(event.pool_state, accounts[4]);
            assert_eq!(event.user_base_token, accounts[5]);
            assert_eq!(event.user_quote_token, accounts[6]);
            assert_eq!(event.base_vault, accounts[7]);
            assert_eq!(event.quote_vault, accounts[8]);
            assert_eq!(event.base_mint, accounts[9]);
            assert_eq!(event.quote_mint, accounts[10]);
            assert_eq!(event.base_token_program, accounts[11]);
            assert_eq!(event.quote_token_program, accounts[12]);
        }
    }

    #[test]
    fn initialize_instructions_expose_current_idl_mint_context() {
        for (discriminator, account_count, token_program_indices) in [
            (discriminators::INITIALIZE, 18, (11, 12)),
            (discriminators::INITIALIZE_V2, 18, (11, 12)),
            (discriminators::INITIALIZE_WITH_TOKEN_2022, 15, (10, 11)),
        ] {
            let accounts: Vec<_> = (0..account_count).map(|_| Pubkey::new_unique()).collect();
            let event = parse_instruction(
                &initialize_data(discriminator),
                &accounts,
                Signature::default(),
                1,
                2,
                Some(3),
            )
            .expect("pool create event");
            let DexEvent::RaydiumLaunchlabPoolCreate(event) = event else {
                panic!("expected LaunchLab pool create");
            };

            assert_eq!(event.payer, accounts[0]);
            assert_eq!(event.creator, accounts[1]);
            assert_eq!(event.global_config, accounts[2]);
            assert_eq!(event.platform_config, accounts[3]);
            assert_eq!(event.pool_state, accounts[5]);
            assert_eq!(event.base_mint, accounts[6]);
            assert_eq!(event.quote_mint, accounts[7]);
            assert_eq!(event.base_vault, accounts[8]);
            assert_eq!(event.quote_vault, accounts[9]);
            assert_eq!(event.base_token_program, accounts[token_program_indices.0]);
            assert_eq!(event.quote_token_program, accounts[token_program_indices.1]);
        }
    }

    #[test]
    fn trade_json_without_new_account_context_remains_readable() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        let event = parse_instruction(
            &trade_data(discriminators::BUY_EXACT_IN),
            &accounts,
            Signature::default(),
            1,
            2,
            Some(3),
        )
        .expect("trade event");
        let DexEvent::RaydiumLaunchlabTrade(event) = event else {
            panic!("expected LaunchLab trade");
        };
        let mut json = serde_json::to_value(event).expect("serialize event");
        let object = json.as_object_mut().expect("event object");
        for field in [
            "global_config",
            "platform_config",
            "user_base_token",
            "user_quote_token",
            "base_vault",
            "quote_vault",
            "base_mint",
            "quote_mint",
            "base_token_program",
            "quote_token_program",
        ] {
            object.remove(field);
        }

        let restored: RaydiumLaunchlabTradeEvent =
            serde_json::from_value(json).expect("deserialize legacy event");
        assert_eq!(restored.quote_mint, Pubkey::default());
        assert_eq!(restored.global_config, Pubkey::default());
    }

    #[test]
    fn pool_create_json_without_new_account_context_remains_readable() {
        let accounts: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
        let event = parse_instruction(
            &initialize_data(discriminators::INITIALIZE),
            &accounts,
            Signature::default(),
            1,
            2,
            Some(3),
        )
        .expect("pool create event");
        let DexEvent::RaydiumLaunchlabPoolCreate(event) = event else {
            panic!("expected LaunchLab pool create");
        };
        let mut json = serde_json::to_value(event).expect("serialize event");
        let object = json.as_object_mut().expect("event object");
        for field in [
            "payer",
            "global_config",
            "platform_config",
            "base_mint",
            "quote_mint",
            "base_vault",
            "quote_vault",
            "base_token_program",
            "quote_token_program",
        ] {
            object.remove(field);
        }

        let restored: RaydiumLaunchlabPoolCreateEvent =
            serde_json::from_value(json).expect("deserialize legacy event");
        assert_eq!(restored.payer, Pubkey::default());
        assert_eq!(restored.quote_mint, Pubkey::default());
    }
}
