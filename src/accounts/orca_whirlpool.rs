//! Orca Whirlpool account parsing.

use crate::core::events::{
    EventMetadata, OrcaFeeTierAccount, OrcaFeeTierAccountEvent, OrcaPositionAccount,
    OrcaPositionAccountEvent, OrcaPositionRewardInfo, OrcaTick, OrcaTickArrayAccount,
    OrcaTickArrayAccountEvent, OrcaWhirlpoolAccount, OrcaWhirlpoolAccountEvent,
    OrcaWhirlpoolRewardInfo, OrcaWhirlpoolsConfigAccount, OrcaWhirlpoolsConfigAccountEvent,
};
use crate::DexEvent;

use super::token::AccountData;
use super::utils::*;

pub mod discriminators {
    pub const WHIRLPOOL: &[u8] = &[63, 149, 209, 12, 225, 128, 99, 9];
    pub const POSITION: &[u8] = &[170, 188, 143, 228, 122, 64, 247, 208];
    pub const TICK_ARRAY: &[u8] = &[69, 97, 189, 190, 110, 7, 66, 187];
    pub const FEE_TIER: &[u8] = &[56, 75, 159, 76, 142, 68, 190, 105];
    pub const WHIRLPOOLS_CONFIG: &[u8] = &[157, 20, 49, 224, 217, 87, 193, 254];
}

pub const WHIRLPOOL_SIZE: usize = 645;
pub const POSITION_SIZE: usize = 208;
pub const TICK_ARRAY_SIZE: usize = 9980;
pub const FEE_TIER_SIZE: usize = 36;
pub const WHIRLPOOLS_CONFIG_SIZE: usize = 98;
const TICK_ARRAY_LEN: usize = 88;

pub fn parse_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if is_whirlpool_account(&account.data) {
        return parse_whirlpool(account, metadata);
    }
    if is_position_account(&account.data) {
        return parse_position(account, metadata);
    }
    if is_tick_array_account(&account.data) {
        return parse_tick_array(account, metadata);
    }
    if is_fee_tier_account(&account.data) {
        return parse_fee_tier(account, metadata);
    }
    if is_whirlpools_config_account(&account.data) {
        return parse_whirlpools_config(account, metadata);
    }
    None
}

pub fn parse_whirlpool(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + WHIRLPOOL_SIZE
        || !has_discriminator(&account.data, discriminators::WHIRLPOOL)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let whirlpool = OrcaWhirlpoolAccount {
        whirlpools_config: read_pubkey_at(data, &mut offset)?,
        whirlpool_bump: read_u8_at(data, &mut offset)?,
        tick_spacing: read_u16_at(data, &mut offset)?,
        tick_spacing_seed: read_u8_array(data, &mut offset)?,
        fee_rate: read_u16_at(data, &mut offset)?,
        protocol_fee_rate: read_u16_at(data, &mut offset)?,
        liquidity: read_u128_at(data, &mut offset)?,
        sqrt_price: read_u128_at(data, &mut offset)?,
        tick_current_index: read_i32_at(data, &mut offset)?,
        protocol_fee_owed_a: read_u64_at(data, &mut offset)?,
        protocol_fee_owed_b: read_u64_at(data, &mut offset)?,
        token_mint_a: read_pubkey_at(data, &mut offset)?,
        token_vault_a: read_pubkey_at(data, &mut offset)?,
        fee_growth_global_a: read_u128_at(data, &mut offset)?,
        token_mint_b: read_pubkey_at(data, &mut offset)?,
        token_vault_b: read_pubkey_at(data, &mut offset)?,
        fee_growth_global_b: read_u128_at(data, &mut offset)?,
        reward_last_updated_timestamp: read_u64_at(data, &mut offset)?,
        reward_infos: [
            parse_reward_info(data, &mut offset)?,
            parse_reward_info(data, &mut offset)?,
            parse_reward_info(data, &mut offset)?,
        ],
    };

    Some(DexEvent::OrcaWhirlpoolAccount(Box::new(OrcaWhirlpoolAccountEvent {
        metadata,
        pubkey: account.pubkey,
        whirlpool,
    })))
}

pub fn parse_position(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + POSITION_SIZE
        || !has_discriminator(&account.data, discriminators::POSITION)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let position = OrcaPositionAccount {
        whirlpool: read_pubkey_at(data, &mut offset)?,
        position_mint: read_pubkey_at(data, &mut offset)?,
        liquidity: read_u128_at(data, &mut offset)?,
        tick_lower_index: read_i32_at(data, &mut offset)?,
        tick_upper_index: read_i32_at(data, &mut offset)?,
        fee_growth_checkpoint_a: read_u128_at(data, &mut offset)?,
        fee_owed_a: read_u64_at(data, &mut offset)?,
        fee_growth_checkpoint_b: read_u128_at(data, &mut offset)?,
        fee_owed_b: read_u64_at(data, &mut offset)?,
        reward_infos: [
            parse_position_reward_info(data, &mut offset)?,
            parse_position_reward_info(data, &mut offset)?,
            parse_position_reward_info(data, &mut offset)?,
        ],
    };

    Some(DexEvent::OrcaPositionAccount(Box::new(OrcaPositionAccountEvent {
        metadata,
        pubkey: account.pubkey,
        position,
    })))
}

pub fn parse_tick_array(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + TICK_ARRAY_SIZE
        || !has_discriminator(&account.data, discriminators::TICK_ARRAY)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let start_tick_index = read_i32_at(data, &mut offset)?;
    let mut ticks = Vec::with_capacity(TICK_ARRAY_LEN);
    for _ in 0..TICK_ARRAY_LEN {
        ticks.push(parse_tick(data, &mut offset)?);
    }
    let tick_array = OrcaTickArrayAccount {
        start_tick_index,
        ticks,
        whirlpool: read_pubkey_at(data, &mut offset)?,
    };

    Some(DexEvent::OrcaTickArrayAccount(Box::new(OrcaTickArrayAccountEvent {
        metadata,
        pubkey: account.pubkey,
        tick_array,
    })))
}

pub fn parse_fee_tier(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + FEE_TIER_SIZE
        || !has_discriminator(&account.data, discriminators::FEE_TIER)
    {
        return None;
    }
    let data = &account.data[8..];
    let mut offset = 0;
    let fee_tier = OrcaFeeTierAccount {
        whirlpools_config: read_pubkey_at(data, &mut offset)?,
        tick_spacing: read_u16_at(data, &mut offset)?,
        default_fee_rate: read_u16_at(data, &mut offset)?,
    };
    Some(DexEvent::OrcaFeeTierAccount(Box::new(OrcaFeeTierAccountEvent {
        metadata,
        pubkey: account.pubkey,
        fee_tier,
    })))
}

pub fn parse_whirlpools_config(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + WHIRLPOOLS_CONFIG_SIZE
        || !has_discriminator(&account.data, discriminators::WHIRLPOOLS_CONFIG)
    {
        return None;
    }
    let data = &account.data[8..];
    let mut offset = 0;
    let config = OrcaWhirlpoolsConfigAccount {
        fee_authority: read_pubkey_at(data, &mut offset)?,
        collect_protocol_fees_authority: read_pubkey_at(data, &mut offset)?,
        reward_emissions_super_authority: read_pubkey_at(data, &mut offset)?,
        default_protocol_fee_rate: read_u16_at(data, &mut offset)?,
    };
    Some(DexEvent::OrcaWhirlpoolsConfigAccount(Box::new(OrcaWhirlpoolsConfigAccountEvent {
        metadata,
        pubkey: account.pubkey,
        config,
    })))
}

pub fn is_whirlpool_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::WHIRLPOOL)
}

pub fn is_position_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::POSITION)
}

pub fn is_tick_array_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::TICK_ARRAY)
}

pub fn is_fee_tier_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::FEE_TIER)
}

pub fn is_whirlpools_config_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::WHIRLPOOLS_CONFIG)
}

fn parse_reward_info(data: &[u8], offset: &mut usize) -> Option<OrcaWhirlpoolRewardInfo> {
    Some(OrcaWhirlpoolRewardInfo {
        mint: read_pubkey_at(data, offset)?,
        vault: read_pubkey_at(data, offset)?,
        authority: read_pubkey_at(data, offset)?,
        emissions_per_second_x64: read_u128_at(data, offset)?,
        growth_global_x64: read_u128_at(data, offset)?,
    })
}

fn parse_position_reward_info(data: &[u8], offset: &mut usize) -> Option<OrcaPositionRewardInfo> {
    Some(OrcaPositionRewardInfo {
        growth_inside_checkpoint: read_u128_at(data, offset)?,
        amount_owed: read_u64_at(data, offset)?,
    })
}

fn parse_tick(data: &[u8], offset: &mut usize) -> Option<OrcaTick> {
    Some(OrcaTick {
        initialized: read_bool_at(data, offset)?,
        liquidity_net: read_i128_at(data, offset)?,
        liquidity_gross: read_u128_at(data, offset)?,
        fee_growth_outside_a: read_u128_at(data, offset)?,
        fee_growth_outside_b: read_u128_at(data, offset)?,
        reward_growths_outside: read_u128_array(data, offset)?,
    })
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
fn read_i32_at(data: &[u8], offset: &mut usize) -> Option<i32> {
    let value = i32::from_le_bytes(data.get(*offset..*offset + 4)?.try_into().ok()?);
    *offset += 4;
    Some(value)
}

#[inline]
fn read_u64_at(data: &[u8], offset: &mut usize) -> Option<u64> {
    let value = read_u64_le(data, *offset)?;
    *offset += 8;
    Some(value)
}

#[inline]
fn read_u128_at(data: &[u8], offset: &mut usize) -> Option<u128> {
    let value = u128::from_le_bytes(data.get(*offset..*offset + 16)?.try_into().ok()?);
    *offset += 16;
    Some(value)
}

#[inline]
fn read_i128_at(data: &[u8], offset: &mut usize) -> Option<i128> {
    let value = i128::from_le_bytes(data.get(*offset..*offset + 16)?.try_into().ok()?);
    *offset += 16;
    Some(value)
}

#[inline]
fn read_u8_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u8; N]> {
    let value = data.get(*offset..*offset + N)?.try_into().ok()?;
    *offset += N;
    Some(value)
}

#[inline]
fn read_u128_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u128; N]> {
    let mut values = [0u128; N];
    for value in &mut values {
        *value = read_u128_at(data, offset)?;
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
            owner: crate::instr::program_ids::ORCA_WHIRLPOOL_PROGRAM_ID,
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
    fn parses_whirlpool_account() {
        let mut data = Vec::with_capacity(8 + WHIRLPOOL_SIZE);
        data.extend_from_slice(discriminators::WHIRLPOOL);
        let config = push_pubkey(&mut data, 1);
        data.push(9);
        data.extend_from_slice(&64u16.to_le_bytes());
        data.extend_from_slice(&64u16.to_le_bytes());
        data.extend_from_slice(&300u16.to_le_bytes());
        data.extend_from_slice(&100u16.to_le_bytes());
        data.extend_from_slice(&123u128.to_le_bytes());
        data.extend_from_slice(&(1u128 << 64).to_le_bytes());
        data.extend_from_slice(&(-12i32).to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());
        data.extend_from_slice(&2u64.to_le_bytes());
        let token_mint_a = push_pubkey(&mut data, 2);
        push_pubkey(&mut data, 3);
        data.extend_from_slice(&10u128.to_le_bytes());
        let token_mint_b = push_pubkey(&mut data, 4);
        push_pubkey(&mut data, 5);
        data.extend_from_slice(&20u128.to_le_bytes());
        data.extend_from_slice(&999u64.to_le_bytes());
        for reward in 0..3u8 {
            push_pubkey(&mut data, 10 + reward * 3);
            push_pubkey(&mut data, 11 + reward * 3);
            push_pubkey(&mut data, 12 + reward * 3);
            data.extend_from_slice(&(1000u128 + reward as u128).to_le_bytes());
            data.extend_from_slice(&(2000u128 + reward as u128).to_le_bytes());
        }

        let event = parse_whirlpool(&account(data), EventMetadata::default()).expect("event");
        let DexEvent::OrcaWhirlpoolAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.whirlpool.whirlpools_config, config);
        assert_eq!(event.whirlpool.tick_current_index, -12);
        assert_eq!(event.whirlpool.token_mint_a, token_mint_a);
        assert_eq!(event.whirlpool.token_mint_b, token_mint_b);
        assert_eq!(event.whirlpool.reward_infos[2].growth_global_x64, 2002);
    }

    #[test]
    fn parses_position_and_fee_tier_accounts() {
        let mut position = Vec::with_capacity(8 + POSITION_SIZE);
        position.extend_from_slice(discriminators::POSITION);
        let whirlpool = push_pubkey(&mut position, 1);
        let mint = push_pubkey(&mut position, 2);
        position.extend_from_slice(&777u128.to_le_bytes());
        position.extend_from_slice(&(-20i32).to_le_bytes());
        position.extend_from_slice(&30i32.to_le_bytes());
        position.extend_from_slice(&10u128.to_le_bytes());
        position.extend_from_slice(&11u64.to_le_bytes());
        position.extend_from_slice(&12u128.to_le_bytes());
        position.extend_from_slice(&13u64.to_le_bytes());
        for i in 0..3u64 {
            position.extend_from_slice(&(100u128 + i as u128).to_le_bytes());
            position.extend_from_slice(&(200u64 + i).to_le_bytes());
        }

        let event = parse_position(&account(position), EventMetadata::default()).expect("event");
        let DexEvent::OrcaPositionAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.position.whirlpool, whirlpool);
        assert_eq!(event.position.position_mint, mint);
        assert_eq!(event.position.liquidity, 777);
        assert_eq!(event.position.reward_infos[1].amount_owed, 201);

        let mut fee_tier = Vec::with_capacity(8 + FEE_TIER_SIZE);
        fee_tier.extend_from_slice(discriminators::FEE_TIER);
        let cfg = push_pubkey(&mut fee_tier, 9);
        fee_tier.extend_from_slice(&128u16.to_le_bytes());
        fee_tier.extend_from_slice(&500u16.to_le_bytes());
        let event = parse_fee_tier(&account(fee_tier), EventMetadata::default()).expect("event");
        let DexEvent::OrcaFeeTierAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.fee_tier.whirlpools_config, cfg);
        assert_eq!(event.fee_tier.tick_spacing, 128);
        assert_eq!(event.fee_tier.default_fee_rate, 500);
    }

    #[test]
    fn parses_tick_array_and_config_accounts() {
        let mut tick_array = Vec::with_capacity(8 + TICK_ARRAY_SIZE);
        tick_array.extend_from_slice(discriminators::TICK_ARRAY);
        tick_array.extend_from_slice(&(-704i32).to_le_bytes());
        for i in 0..TICK_ARRAY_LEN {
            tick_array.push((i % 2) as u8);
            tick_array.extend_from_slice(&(i as i128 - 44).to_le_bytes());
            tick_array.extend_from_slice(&(1000u128 + i as u128).to_le_bytes());
            tick_array.extend_from_slice(&(2000u128 + i as u128).to_le_bytes());
            tick_array.extend_from_slice(&(3000u128 + i as u128).to_le_bytes());
            for j in 0..3 {
                tick_array.extend_from_slice(&(4000u128 + i as u128 + j).to_le_bytes());
            }
        }
        let whirlpool = push_pubkey(&mut tick_array, 8);
        let event =
            parse_tick_array(&account(tick_array), EventMetadata::default()).expect("event");
        let DexEvent::OrcaTickArrayAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.tick_array.whirlpool, whirlpool);
        assert_eq!(event.tick_array.ticks[87].liquidity_gross, 1087);

        let mut config = Vec::with_capacity(8 + WHIRLPOOLS_CONFIG_SIZE);
        config.extend_from_slice(discriminators::WHIRLPOOLS_CONFIG);
        let fee_authority = push_pubkey(&mut config, 1);
        push_pubkey(&mut config, 2);
        push_pubkey(&mut config, 3);
        config.extend_from_slice(&250u16.to_le_bytes());
        let event =
            parse_whirlpools_config(&account(config), EventMetadata::default()).expect("event");
        let DexEvent::OrcaWhirlpoolsConfigAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.config.fee_authority, fee_authority);
        assert_eq!(event.config.default_protocol_fee_rate, 250);
    }
}
