//! Meteora DBC log parser.

use super::utils::*;
use crate::core::events::*;
use solana_sdk::signature::Signature;

pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const INITIALIZE_POOL_EVENT: [u8; 8] = [228, 50, 246, 85, 203, 66, 134, 37];
    pub const CURVE_COMPLETE_EVENT: [u8; 8] = [229, 231, 86, 84, 156, 134, 75, 24];
}

pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];
    let pool = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);

    match discriminator {
        discriminators::SWAP_EVENT => parse_swap_from_data(data, metadata),
        discriminators::INITIALIZE_POOL_EVENT => parse_initialize_pool_from_data(data, metadata),
        discriminators::CURVE_COMPLETE_EVENT => parse_curve_complete_from_data(data, metadata),
        _ => None,
    }
}

#[inline(always)]
pub fn parse_swap_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;
    let config = read_pubkey(data, offset)?;
    offset += 32;
    let trade_direction = read_u8(data, offset)?;
    offset += 1;
    let has_referral = read_bool(data, offset)?;
    offset += 1;
    let params_amount_in = read_u64_le(data, offset)?;
    offset += 8;
    let minimum_amount_out = read_u64_le(data, offset)?;
    offset += 8;
    let actual_input_amount = read_u64_le(data, offset)?;
    offset += 8;
    let output_amount = read_u64_le(data, offset)?;
    offset += 8;
    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;
    let trading_fee = read_u64_le(data, offset)?;
    offset += 8;
    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;
    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;
    let amount_in = read_u64_le(data, offset).unwrap_or(params_amount_in);
    offset += 8;
    let current_timestamp = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDbcSwap(MeteoraDbcSwapEvent {
        metadata,
        pool,
        config,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        actual_input_amount,
        output_amount,
        next_sqrt_price,
        trading_fee,
        protocol_fee,
        referral_fee,
        current_timestamp,
    }))
}

#[inline(always)]
pub fn parse_initialize_pool_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;
    let config = read_pubkey(data, offset)?;
    offset += 32;
    let creator = read_pubkey(data, offset)?;
    offset += 32;
    let base_mint = read_pubkey(data, offset)?;
    offset += 32;
    let pool_type = read_u8(data, offset)?;
    offset += 1;
    let activation_point = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDbcInitializePool(MeteoraDbcInitializePoolEvent {
        metadata,
        pool,
        config,
        creator,
        base_mint,
        pool_type,
        activation_point,
    }))
}

#[inline(always)]
pub fn parse_curve_complete_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool = read_pubkey(data, offset)?;
    offset += 32;
    let config = read_pubkey(data, offset)?;
    offset += 32;
    let base_reserve = read_u64_le(data, offset)?;
    offset += 8;
    let quote_reserve = read_u64_le(data, offset)?;

    Some(DexEvent::MeteoraDbcCurveComplete(MeteoraDbcCurveCompleteEvent {
        metadata,
        pool,
        config,
        base_reserve,
        quote_reserve,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn push_pubkey(buf: &mut Vec<u8>, byte: u8) -> Pubkey {
        let key = Pubkey::new_from_array([byte; 32]);
        buf.extend_from_slice(key.as_ref());
        key
    }

    #[test]
    fn parses_dbc_swap_layout() {
        let mut data = Vec::new();
        let pool = push_pubkey(&mut data, 1);
        let config = push_pubkey(&mut data, 2);
        data.push(1);
        data.push(1);
        data.extend_from_slice(&10_u64.to_le_bytes());
        data.extend_from_slice(&9_u64.to_le_bytes());
        data.extend_from_slice(&10_u64.to_le_bytes());
        data.extend_from_slice(&8_u64.to_le_bytes());
        data.extend_from_slice(&(1_u128 << 64).to_le_bytes());
        data.extend_from_slice(&1_u64.to_le_bytes());
        data.extend_from_slice(&2_u64.to_le_bytes());
        data.extend_from_slice(&3_u64.to_le_bytes());
        data.extend_from_slice(&10_u64.to_le_bytes());
        data.extend_from_slice(&123_u64.to_le_bytes());

        let event = parse_swap_from_data(&data, EventMetadata::default()).unwrap();
        match event {
            DexEvent::MeteoraDbcSwap(event) => {
                assert_eq!(event.pool, pool);
                assert_eq!(event.config, config);
                assert_eq!(event.trade_direction, 1);
                assert!(event.has_referral);
                assert_eq!(event.minimum_amount_out, 9);
                assert_eq!(event.output_amount, 8);
                assert_eq!(event.protocol_fee, 2);
                assert_eq!(event.current_timestamp, 123);
            }
            other => panic!("expected MeteoraDbcSwap, got {other:?}"),
        }
    }

    #[test]
    fn parses_dbc_initialize_pool_layout() {
        let mut data = Vec::new();
        let pool = push_pubkey(&mut data, 1);
        let config = push_pubkey(&mut data, 2);
        let creator = push_pubkey(&mut data, 3);
        let base_mint = push_pubkey(&mut data, 4);
        data.push(2);
        data.extend_from_slice(&456_u64.to_le_bytes());

        let event = parse_initialize_pool_from_data(&data, EventMetadata::default()).unwrap();
        match event {
            DexEvent::MeteoraDbcInitializePool(event) => {
                assert_eq!(event.pool, pool);
                assert_eq!(event.config, config);
                assert_eq!(event.creator, creator);
                assert_eq!(event.base_mint, base_mint);
                assert_eq!(event.pool_type, 2);
                assert_eq!(event.activation_point, 456);
            }
            other => panic!("expected MeteoraDbcInitializePool, got {other:?}"),
        }
    }
}
