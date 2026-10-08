//! Meteora DBC log parser.

use super::utils::*;
use crate::core::events::*;
use solana_sdk::signature::Signature;

pub mod discriminators {
    pub const SWAP2_EVENT: [u8; 8] = [189, 66, 51, 168, 38, 80, 117, 153];
    pub const SWAP2_TRANSFER_HOOK_EVENT: [u8; 8] = [134, 59, 168, 120, 94, 51, 114, 231];
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
        discriminators::SWAP2_EVENT | discriminators::SWAP2_TRANSFER_HOOK_EVENT => {
            parse_swap2_from_data(
                data,
                metadata,
                discriminator == discriminators::SWAP2_TRANSFER_HOOK_EVENT,
            )
        }
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
        ..Default::default()
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

/// Current IDL EvtSwap2 / EvtSwap2WithTransferHook share the same 179-byte body.
pub fn parse_swap2_from_data(
    data: &[u8],
    metadata: EventMetadata,
    has_transfer_hook: bool,
) -> Option<DexEvent> {
    if data.len() < 179 {
        return None;
    }
    let swap_mode = read_u8(data, 82)?;
    let trade_direction = read_u8(data, 64)?;
    if swap_mode > 2 || trade_direction > 1 || data[65] > 1 {
        return None;
    }
    let amount_0 = read_u64_le(data, 66)?;
    let amount_1 = read_u64_le(data, 74)?;
    let included_fee_input_amount = read_u64_le(data, 83)?;
    Some(DexEvent::MeteoraDbcSwap(MeteoraDbcSwapEvent {
        metadata,
        pool: read_pubkey(data, 0)?,
        config: read_pubkey(data, 32)?,
        trade_direction,
        has_referral: read_bool(data, 65)?,
        event_version: 2,
        swap_mode,
        amount_0,
        amount_1,
        has_transfer_hook,
        amount_in: included_fee_input_amount,
        minimum_amount_out: if swap_mode == 2 { 0 } else { amount_1 },
        maximum_amount_in: if swap_mode == 2 { amount_1 } else { 0 },
        included_fee_input_amount,
        actual_input_amount: read_u64_le(data, 91)?,
        amount_left: read_u64_le(data, 99)?,
        output_amount: read_u64_le(data, 107)?,
        next_sqrt_price: read_u128_le(data, 115)?,
        trading_fee: read_u64_le(data, 131)?,
        protocol_fee: read_u64_le(data, 139)?,
        referral_fee: read_u64_le(data, 147)?,
        quote_reserve_amount: read_u64_le(data, 155)?,
        migration_threshold: read_u64_le(data, 163)?,
        current_timestamp: read_u64_le(data, 171)?,
    }))
}

/// Decode Anchor event CPI with explicit DBC program context at the caller.
pub fn parse_event_cpi(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    if data.get(..8)? != [228, 69, 165, 46, 81, 203, 154, 29] {
        return None;
    }
    let disc: [u8; 8] = data.get(8..16)?.try_into().ok()?;
    let body = data.get(16..)?;
    match disc {
        discriminators::SWAP_EVENT => parse_swap_from_data(body, metadata),
        discriminators::SWAP2_EVENT | discriminators::SWAP2_TRANSFER_HOOK_EVENT => {
            parse_swap2_from_data(body, metadata, disc == discriminators::SWAP2_TRANSFER_HOOK_EVENT)
        }
        discriminators::INITIALIZE_POOL_EVENT => parse_initialize_pool_from_data(body, metadata),
        discriminators::CURVE_COMPLETE_EVENT => parse_curve_complete_from_data(body, metadata),
        _ => None,
    }
}

#[cfg(test)]
mod current_dbc_tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    #[test]
    fn current_official_modes_and_transfer_hook_cpi() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/dbc_swap2.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let bytes = STANDARD.decode(case["data"].as_str().unwrap()).unwrap();
            let mut cpi = vec![228, 69, 165, 46, 81, 203, 154, 29];
            cpi.extend(&bytes);
            let e = parse_event_cpi(&cpi, EventMetadata::default()).unwrap();
            let DexEvent::MeteoraDbcSwap(e) = e else { panic!() };
            let mode = case["mode"].as_u64().unwrap() as u8;
            assert_eq!(e.event_version, 2);
            assert_eq!(e.swap_mode, mode);
            assert_eq!(e.amount_in, if mode == 0 { 100000 } else { 90000 });
            assert_eq!(e.minimum_amount_out, if mode == 2 { 0 } else { 79000 });
            assert_eq!(e.maximum_amount_in, if mode == 2 { 100000 } else { 0 });
            assert_eq!(e.actual_input_amount, e.amount_in - 1000);
            assert_eq!(e.amount_left, if mode == 1 { 10000 } else { 0 });
            assert_eq!(e.output_amount, 80000);
            assert_eq!(e.next_sqrt_price, (1u128 << 100) + 7);
            assert_eq!(e.quote_reserve_amount, 9007199254740993);
            assert_eq!(e.migration_threshold, 9007199254740995);
            assert_eq!(e.has_transfer_hook, case["name"] == "EvtSwap2WithTransferHook");
            for len in 0..cpi.len() {
                assert!(parse_event_cpi(&cpi[..len], EventMetadata::default()).is_none());
            }
            let mut invalid = bytes[8..].to_vec();
            invalid[82] = 3;
            assert!(parse_swap2_from_data(&invalid, EventMetadata::default(), false).is_none());
        }
    }
}
