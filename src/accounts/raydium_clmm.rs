//! Raydium CLMM account parsing.

use crate::core::events::{
    EventMetadata, RaydiumClmmAmmConfig, RaydiumClmmAmmConfigAccountEvent,
    RaydiumClmmDynamicFeeInfo, RaydiumClmmPoolState, RaydiumClmmPoolStateAccountEvent,
    RaydiumClmmRewardInfo, RaydiumClmmTickArrayState, RaydiumClmmTickArrayStateAccountEvent, Tick,
};
use crate::DexEvent;

use super::token::AccountData;
use super::utils::*;

pub mod discriminators {
    pub const AMM_CONFIG: &[u8] = &[218, 244, 33, 104, 203, 203, 43, 111];
    pub const POOL_STATE: &[u8] = &[247, 237, 227, 245, 215, 195, 222, 70];
    pub const TICK_ARRAY_STATE: &[u8] = &[192, 155, 85, 205, 49, 249, 129, 42];
}

pub const AMM_CONFIG_SIZE: usize = 109;
pub const POOL_STATE_SIZE: usize = 1536;
pub const TICK_ARRAY_STATE_SIZE: usize = 10232;
const TICK_ARRAY_LEN: usize = 60;

pub fn parse_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if is_amm_config_account(&account.data) {
        return parse_amm_config(account, metadata);
    }
    if is_pool_state_account(&account.data) {
        return parse_pool_state(account, metadata);
    }
    if is_tick_array_state_account(&account.data) {
        return parse_tick_array_state(account, metadata);
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
    let amm_config = RaydiumClmmAmmConfig {
        bump: read_u8_at(data, &mut offset)?,
        index: read_u16_at(data, &mut offset)?,
        owner: read_pubkey_at(data, &mut offset)?,
        protocol_fee_rate: read_u32_at(data, &mut offset)?,
        trade_fee_rate: read_u32_at(data, &mut offset)?,
        tick_spacing: read_u16_at(data, &mut offset)?,
        fund_fee_rate: read_u32_at(data, &mut offset)?,
        padding_u32: read_u32_at(data, &mut offset)?,
        fund_owner: read_pubkey_at(data, &mut offset)?,
        padding: read_u64_array(data, &mut offset)?,
    };

    Some(DexEvent::RaydiumClmmAmmConfigAccount(Box::new(RaydiumClmmAmmConfigAccountEvent {
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
    let pool_state = RaydiumClmmPoolState {
        bump: [read_u8_at(data, &mut offset)?],
        amm_config: read_pubkey_at(data, &mut offset)?,
        owner: read_pubkey_at(data, &mut offset)?,
        token_mint_0: read_pubkey_at(data, &mut offset)?,
        token_mint_1: read_pubkey_at(data, &mut offset)?,
        token_vault_0: read_pubkey_at(data, &mut offset)?,
        token_vault_1: read_pubkey_at(data, &mut offset)?,
        observation_key: read_pubkey_at(data, &mut offset)?,
        mint_decimals_0: read_u8_at(data, &mut offset)?,
        mint_decimals_1: read_u8_at(data, &mut offset)?,
        tick_spacing: read_u16_at(data, &mut offset)?,
        liquidity: read_u128_at(data, &mut offset)?,
        sqrt_price_x64: read_u128_at(data, &mut offset)?,
        tick_current: read_i32_at(data, &mut offset)?,
        padding3: read_u16_at(data, &mut offset)?,
        padding4: read_u16_at(data, &mut offset)?,
        fee_growth_global_0_x64: read_u128_at(data, &mut offset)?,
        fee_growth_global_1_x64: read_u128_at(data, &mut offset)?,
        protocol_fees_token_0: read_u64_at(data, &mut offset)?,
        protocol_fees_token_1: read_u64_at(data, &mut offset)?,
        padding5: read_u128_array(data, &mut offset)?,
        status: read_u8_at(data, &mut offset)?,
        fee_on: read_u8_at(data, &mut offset)?,
        padding: read_u8_array(data, &mut offset)?,
        reward_infos: [
            parse_reward_info(data, &mut offset)?,
            parse_reward_info(data, &mut offset)?,
            parse_reward_info(data, &mut offset)?,
        ],
        tick_array_bitmap: read_u64_array(data, &mut offset)?,
        padding6: read_u64_array(data, &mut offset)?,
        fund_fees_token_0: read_u64_at(data, &mut offset)?,
        fund_fees_token_1: read_u64_at(data, &mut offset)?,
        open_time: read_u64_at(data, &mut offset)?,
        recent_epoch: read_u64_at(data, &mut offset)?,
        dynamic_fee_info: parse_dynamic_fee_info(data, &mut offset)?,
        padding1: read_u64_array(data, &mut offset)?,
        padding2: read_u64_array(data, &mut offset)?,
    };

    Some(DexEvent::RaydiumClmmPoolStateAccount(Box::new(RaydiumClmmPoolStateAccountEvent {
        metadata,
        pubkey: account.pubkey,
        pool_state,
    })))
}

pub fn parse_tick_array_state(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    if account.data.len() < 8 + TICK_ARRAY_STATE_SIZE
        || !has_discriminator(&account.data, discriminators::TICK_ARRAY_STATE)
    {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0;
    let pool_id = read_pubkey_at(data, &mut offset)?;
    let start_tick_index = read_i32_at(data, &mut offset)?;
    let mut ticks = Vec::with_capacity(TICK_ARRAY_LEN);
    for _ in 0..TICK_ARRAY_LEN {
        ticks.push(parse_tick(data, &mut offset)?);
    }
    let tick_array_state = RaydiumClmmTickArrayState {
        pool_id,
        start_tick_index,
        ticks,
        initialized_tick_count: read_u8_at(data, &mut offset)?,
        recent_epoch: read_u64_at(data, &mut offset)?,
        padding: read_u8_array(data, &mut offset)?,
    };

    Some(DexEvent::RaydiumClmmTickArrayStateAccount(Box::new(
        RaydiumClmmTickArrayStateAccountEvent {
            metadata,
            pubkey: account.pubkey,
            tick_array_state,
        },
    )))
}

pub fn is_amm_config_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::AMM_CONFIG)
}

pub fn is_pool_state_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::POOL_STATE)
}

pub fn is_tick_array_state_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::TICK_ARRAY_STATE)
}

fn parse_reward_info(data: &[u8], offset: &mut usize) -> Option<RaydiumClmmRewardInfo> {
    Some(RaydiumClmmRewardInfo {
        reward_state: read_u8_at(data, offset)?,
        open_time: read_u64_at(data, offset)?,
        end_time: read_u64_at(data, offset)?,
        last_update_time: read_u64_at(data, offset)?,
        emissions_per_second_x64: read_u128_at(data, offset)?,
        reward_total_emitted: read_u64_at(data, offset)?,
        reward_claimed: read_u64_at(data, offset)?,
        token_mint: read_pubkey_at(data, offset)?,
        token_vault: read_pubkey_at(data, offset)?,
        authority: read_pubkey_at(data, offset)?,
        reward_growth_global_x64: read_u128_at(data, offset)?,
    })
}

fn parse_dynamic_fee_info(data: &[u8], offset: &mut usize) -> Option<RaydiumClmmDynamicFeeInfo> {
    Some(RaydiumClmmDynamicFeeInfo {
        filter_period: read_u16_at(data, offset)?,
        decay_period: read_u16_at(data, offset)?,
        reduction_factor: read_u16_at(data, offset)?,
        dynamic_fee_control: read_u32_at(data, offset)?,
        max_volatility_accumulator: read_u32_at(data, offset)?,
        tick_spacing_index_reference: read_i32_at(data, offset)?,
        volatility_reference: read_u32_at(data, offset)?,
        volatility_accumulator: read_u32_at(data, offset)?,
        last_update_timestamp: read_u64_at(data, offset)?,
        padding: read_u8_array(data, offset)?,
    })
}

fn parse_tick(data: &[u8], offset: &mut usize) -> Option<Tick> {
    Some(Tick {
        tick: read_i32_at(data, offset)?,
        liquidity_net: read_i128_at(data, offset)?,
        liquidity_gross: read_u128_at(data, offset)?,
        fee_growth_outside_0_x64: read_u128_at(data, offset)?,
        fee_growth_outside_1_x64: read_u128_at(data, offset)?,
        reward_growths_outside_x64: read_u128_array(data, offset)?,
        order_phase: read_u64_at(data, offset)?,
        orders_amount: read_u64_at(data, offset)?,
        part_filled_orders_remaining: read_u64_at(data, offset)?,
        unfilled_ratio_x64: read_u128_at(data, offset)?,
        padding: read_u32_array(data, offset)?,
    })
}

#[inline]
fn read_pubkey_at(data: &[u8], offset: &mut usize) -> Option<solana_sdk::pubkey::Pubkey> {
    let value = read_pubkey(data, *offset)?;
    *offset += 32;
    Some(value)
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
fn read_u32_at(data: &[u8], offset: &mut usize) -> Option<u32> {
    let value = u32::from_le_bytes(data.get(*offset..*offset + 4)?.try_into().ok()?);
    *offset += 4;
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
fn read_u32_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u32; N]> {
    let mut values = [0u32; N];
    for value in &mut values {
        *value = read_u32_at(data, offset)?;
    }
    Some(values)
}

#[inline]
fn read_u64_array<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u64; N]> {
    let mut values = [0u64; N];
    for value in &mut values {
        *value = read_u64_at(data, offset)?;
    }
    Some(values)
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
            owner: crate::instr::program_ids::RAYDIUM_CLMM_PROGRAM_ID,
            data,
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        }
    }

    #[test]
    fn parses_amm_config_account() {
        let owner = Pubkey::new_unique();
        let fund_owner = Pubkey::new_unique();
        let mut data = Vec::with_capacity(8 + AMM_CONFIG_SIZE);
        data.extend_from_slice(discriminators::AMM_CONFIG);
        data.push(9);
        data.extend_from_slice(&7u16.to_le_bytes());
        data.extend_from_slice(owner.as_ref());
        data.extend_from_slice(&111u32.to_le_bytes());
        data.extend_from_slice(&222u32.to_le_bytes());
        data.extend_from_slice(&64u16.to_le_bytes());
        data.extend_from_slice(&333u32.to_le_bytes());
        data.extend_from_slice(&444u32.to_le_bytes());
        data.extend_from_slice(fund_owner.as_ref());
        for value in [1u64, 2, 3] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let event = parse_amm_config(&account(data), EventMetadata::default()).expect("event");
        let DexEvent::RaydiumClmmAmmConfigAccount(event) = event else {
            panic!("wrong event type");
        };
        assert_eq!(event.amm_config.bump, 9);
        assert_eq!(event.amm_config.index, 7);
        assert_eq!(event.amm_config.owner, owner);
        assert_eq!(event.amm_config.tick_spacing, 64);
        assert_eq!(event.amm_config.fund_owner, fund_owner);
        assert_eq!(event.amm_config.padding, [1, 2, 3]);
    }

    #[test]
    fn parses_tick_array_tail_limit_order_fields() {
        let pool_id = Pubkey::new_unique();
        let mut data = Vec::with_capacity(8 + TICK_ARRAY_STATE_SIZE);
        data.extend_from_slice(discriminators::TICK_ARRAY_STATE);
        data.extend_from_slice(pool_id.as_ref());
        data.extend_from_slice(&(-128i32).to_le_bytes());
        for i in 0..TICK_ARRAY_LEN {
            data.extend_from_slice(&(i as i32).to_le_bytes());
            data.extend_from_slice(&(i as i128 - 30).to_le_bytes());
            data.extend_from_slice(&(1000u128 + i as u128).to_le_bytes());
            data.extend_from_slice(&(2000u128 + i as u128).to_le_bytes());
            data.extend_from_slice(&(3000u128 + i as u128).to_le_bytes());
            for j in 0..3 {
                data.extend_from_slice(&(4000u128 + i as u128 + j).to_le_bytes());
            }
            data.extend_from_slice(&(5000u64 + i as u64).to_le_bytes());
            data.extend_from_slice(&(6000u64 + i as u64).to_le_bytes());
            data.extend_from_slice(&(7000u64 + i as u64).to_le_bytes());
            data.extend_from_slice(&(8000u128 + i as u128).to_le_bytes());
            for j in 0..3 {
                data.extend_from_slice(&(9000u32 + i as u32 + j).to_le_bytes());
            }
        }
        data.push(12);
        data.extend_from_slice(&88u64.to_le_bytes());
        data.extend_from_slice(&[0u8; 107]);

        let event =
            parse_tick_array_state(&account(data), EventMetadata::default()).expect("event");
        let DexEvent::RaydiumClmmTickArrayStateAccount(event) = event else {
            panic!("wrong event type");
        };
        let tick = &event.tick_array_state.ticks[59];
        assert_eq!(event.tick_array_state.pool_id, pool_id);
        assert_eq!(event.tick_array_state.initialized_tick_count, 12);
        assert_eq!(tick.order_phase, 5059);
        assert_eq!(tick.orders_amount, 6059);
        assert_eq!(tick.part_filled_orders_remaining, 7059);
        assert_eq!(tick.unfilled_ratio_x64, 8059);
        assert_eq!(tick.padding, [9059, 9060, 9061]);
    }
}
