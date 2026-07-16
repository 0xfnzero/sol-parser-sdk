//! Raydium CPMM account parsing.

use crate::core::events::{
    EventMetadata, RaydiumCpmmAmmConfig, RaydiumCpmmAmmConfigAccountEvent, RaydiumCpmmPoolState,
    RaydiumCpmmPoolStateAccountEvent,
};
use crate::DexEvent;

use super::token::AccountData;
use super::utils::*;

pub mod discriminators {
    pub const AMM_CONFIG: &[u8] = &[218, 244, 33, 104, 203, 203, 43, 111];
    pub const POOL_STATE: &[u8] = &[247, 237, 227, 245, 215, 195, 222, 70];
}

pub const AMM_CONFIG_SIZE: usize = 228;
pub const POOL_STATE_SIZE: usize = 629;

pub fn parse_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if is_amm_config_account(&account.data) {
        return parse_amm_config(account, metadata);
    }
    if is_pool_state_account(&account.data) {
        return parse_pool_state(account, metadata);
    }
    None
}

pub fn parse_amm_config(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + AMM_CONFIG_SIZE
        || !has_discriminator(&account.data, discriminators::AMM_CONFIG)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let amm_config = RaydiumCpmmAmmConfig {
        bump: read_u8_at(data, &mut offset)?,
        disable_create_pool: read_bool_at(data, &mut offset)?,
        index: read_u16_at(data, &mut offset)?,
        trade_fee_rate: read_u64_at(data, &mut offset)?,
        protocol_fee_rate: read_u64_at(data, &mut offset)?,
        fund_fee_rate: read_u64_at(data, &mut offset)?,
        create_pool_fee: read_u64_at(data, &mut offset)?,
        protocol_owner: read_pubkey_at(data, &mut offset)?,
        fund_owner: read_pubkey_at(data, &mut offset)?,
        creator_fee_rate: read_u64_at(data, &mut offset)?,
        padding: read_u64_array(data, &mut offset)?,
    };

    Some(DexEvent::RaydiumCpmmAmmConfigAccount(Box::new(RaydiumCpmmAmmConfigAccountEvent {
        metadata,
        pubkey: account.pubkey,
        amm_config,
    })))
}

pub fn parse_pool_state(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + POOL_STATE_SIZE
        || !has_discriminator(&account.data, discriminators::POOL_STATE)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let pool_state = RaydiumCpmmPoolState {
        amm_config: read_pubkey_at(data, &mut offset)?,
        pool_creator: read_pubkey_at(data, &mut offset)?,
        token_0_vault: read_pubkey_at(data, &mut offset)?,
        token_1_vault: read_pubkey_at(data, &mut offset)?,
        lp_mint: read_pubkey_at(data, &mut offset)?,
        token_0_mint: read_pubkey_at(data, &mut offset)?,
        token_1_mint: read_pubkey_at(data, &mut offset)?,
        token_0_program: read_pubkey_at(data, &mut offset)?,
        token_1_program: read_pubkey_at(data, &mut offset)?,
        observation_key: read_pubkey_at(data, &mut offset)?,
        auth_bump: read_u8_at(data, &mut offset)?,
        status: read_u8_at(data, &mut offset)?,
        lp_mint_decimals: read_u8_at(data, &mut offset)?,
        mint_0_decimals: read_u8_at(data, &mut offset)?,
        mint_1_decimals: read_u8_at(data, &mut offset)?,
        lp_supply: read_u64_at(data, &mut offset)?,
        protocol_fees_token_0: read_u64_at(data, &mut offset)?,
        protocol_fees_token_1: read_u64_at(data, &mut offset)?,
        fund_fees_token_0: read_u64_at(data, &mut offset)?,
        fund_fees_token_1: read_u64_at(data, &mut offset)?,
        open_time: read_u64_at(data, &mut offset)?,
        recent_epoch: read_u64_at(data, &mut offset)?,
        creator_fee_on: read_u8_at(data, &mut offset)?,
        enable_creator_fee: read_bool_at(data, &mut offset)?,
        padding1: read_u8_array(data, &mut offset)?,
        creator_fees_token_0: read_u64_at(data, &mut offset)?,
        creator_fees_token_1: read_u64_at(data, &mut offset)?,
        padding: read_u64_array(data, &mut offset)?,
    };

    Some(DexEvent::RaydiumCpmmPoolStateAccount(Box::new(RaydiumCpmmPoolStateAccountEvent {
        metadata,
        pubkey: account.pubkey,
        pool_state,
    })))
}

pub fn is_amm_config_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::AMM_CONFIG)
}

pub fn is_pool_state_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::POOL_STATE)
}

#[inline]
fn read_pubkey_at(data: &[u8], offset: &mut usize) -> Option<solana_sdk::pubkey::Pubkey> {
    let value = read_pubkey(data, *offset)?;
    *offset += 32;
    Some(value)
}

#[inline]
fn read_bool_at(data: &[u8], offset: &mut usize) -> Option<bool> {
    Some(read_u8_at(data, offset)? != 0)
}

#[inline]
fn read_u8_at(data: &[u8], offset: &mut usize) -> Option<u8> {
    let value = read_u8(data, *offset)?;
    *offset += 1;
    Some(value)
}

#[inline]
fn read_u16_at(data: &[u8], offset: &mut usize) -> Option<u16> {
    let value = read_u16_le(data, *offset)?;
    *offset += 2;
    Some(value)
}

#[inline]
fn read_u64_at(data: &[u8], offset: &mut usize) -> Option<u64> {
    let value = read_u64_le(data, *offset)?;
    *offset += 8;
    Some(value)
}

#[inline]
fn read_u8_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u8; N]> {
    let value = data.get(*offset..*offset + N)?.try_into().ok()?;
    *offset += N;
    Some(value)
}

#[inline]
fn read_u64_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u64; N]> {
    let mut values = [0u64; N];
    for value in &mut values {
        *value = read_u64_at(data, offset)?;
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn account(data: Vec<u8>) -> AccountData {
        AccountData {
            pubkey: Pubkey::new_unique(),
            owner: crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID,
            data,
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        }
    }

    fn push_pubkey(data: &mut Vec<u8>, byte: u8) -> Pubkey {
        let key = Pubkey::new_from_array([byte; 32]);
        data.extend_from_slice(key.as_ref());
        key
    }

    #[test]
    fn parses_cpmm_amm_config_account() {
        let mut data = Vec::with_capacity(8 + AMM_CONFIG_SIZE);
        data.extend_from_slice(discriminators::AMM_CONFIG);
        data.push(3);
        data.push(1);
        data.extend_from_slice(&7u16.to_le_bytes());
        for value in [10u64, 20, 30, 40] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        let protocol_owner = push_pubkey(&mut data, 1);
        let fund_owner = push_pubkey(&mut data, 2);
        data.extend_from_slice(&50u64.to_le_bytes());
        for value in 0u64..15 {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let event = parse_amm_config(&account(data), EventMetadata::default()).expect("event");
        let DexEvent::RaydiumCpmmAmmConfigAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.amm_config.bump, 3);
        assert!(event.amm_config.disable_create_pool);
        assert_eq!(event.amm_config.index, 7);
        assert_eq!(event.amm_config.trade_fee_rate, 10);
        assert_eq!(event.amm_config.protocol_owner, protocol_owner);
        assert_eq!(event.amm_config.fund_owner, fund_owner);
        assert_eq!(event.amm_config.creator_fee_rate, 50);
    }

    #[test]
    fn parses_cpmm_pool_state_account() {
        let mut data = Vec::with_capacity(8 + POOL_STATE_SIZE);
        data.extend_from_slice(discriminators::POOL_STATE);
        let amm_config = push_pubkey(&mut data, 1);
        let pool_creator = push_pubkey(&mut data, 2);
        for byte in 3..=10 {
            push_pubkey(&mut data, byte);
        }
        for byte in [11u8, 1, 9, 6, 6] {
            data.push(byte);
        }
        for value in [100u64, 1, 2, 3, 4, 123456, 99] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.push(2);
        data.push(1);
        data.extend_from_slice(&[0u8; 6]);
        data.extend_from_slice(&5u64.to_le_bytes());
        data.extend_from_slice(&6u64.to_le_bytes());
        for value in 0u64..28 {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let event = parse_pool_state(&account(data), EventMetadata::default()).expect("event");
        let DexEvent::RaydiumCpmmPoolStateAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.pool_state.amm_config, amm_config);
        assert_eq!(event.pool_state.pool_creator, pool_creator);
        assert_eq!(event.pool_state.auth_bump, 11);
        assert_eq!(event.pool_state.lp_supply, 100);
        assert!(event.pool_state.enable_creator_fee);
        assert_eq!(event.pool_state.creator_fees_token_1, 6);
    }
}
