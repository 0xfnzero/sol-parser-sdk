//! Meteora DBC inner-instruction (event-CPI) parser.
//!
//! The program emits its events with `emit_cpi!` only, never as `Program
//! data:` logs, so this is where DBC swaps come from. The instruction accounts
//! are filled afterwards from the swap instruction of the same pool.

use crate::core::events::*;

pub mod discriminators {
    /// `EvtSwap`: the legacy twin a `swap` or `swap2` emits next to `EvtSwap2`.
    pub const SWAP: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 27, 60, 21, 213, 138, 170, 187, 147];
    pub const SWAP2: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 189, 66, 51, 168, 38, 80, 117, 153];
    pub const SWAP2_WITH_TRANSFER_HOOK: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 134, 59, 168, 120, 94, 51, 114, 231];
    pub const CURVE_COMPLETE: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 229, 231, 86, 84, 156, 134, 75, 24];
    pub const CURVE_COMPLETE_WITH_TRANSFER_HOOK: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 59, 47, 109, 205, 13, 31, 44, 159];
    pub const INITIALIZE_POOL: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 228, 50, 246, 85, 203, 66, 134, 37];
    pub const INITIALIZE_POOL_WITH_TRANSFER_HOOK: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 213, 137, 164, 53, 193, 74, 15, 110];
}

/// The swap instructions whose accounts fill a swap event.
pub mod instruction_discriminators {
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const SWAP2: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];
    pub const SWAP2_WITH_TRANSFER_HOOK: [u8; 8] = [183, 93, 153, 40, 24, 230, 194, 151];
}

/// Parses a DBC event-CPI by its 16-byte discriminator. The transfer-hook
/// variants share their layouts with the plain events.
#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    use crate::logs::meteora_dbc::{
        parse_curve_complete_from_data, parse_initialize_pool_from_data, parse_swap2_from_data,
    };
    match *disc {
        discriminators::SWAP2 => parse_swap2_from_data(data, metadata, false),
        discriminators::SWAP2_WITH_TRANSFER_HOOK => parse_swap2_from_data(data, metadata, true),
        // The same trade as the `EvtSwap2` that follows it.
        discriminators::SWAP => None,
        discriminators::CURVE_COMPLETE | discriminators::CURVE_COMPLETE_WITH_TRANSFER_HOOK => {
            parse_curve_complete_from_data(data, metadata)
        }
        discriminators::INITIALIZE_POOL | discriminators::INITIALIZE_POOL_WITH_TRANSFER_HOOK => {
            parse_initialize_pool_from_data(data, metadata)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn swap2_body(pool: Pubkey, config: Pubkey, swap_mode: u8) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(pool.as_ref());
        data.extend_from_slice(config.as_ref());
        data.push(1); // quote to base
        data.push(1); // has referral
        data.extend_from_slice(&100_u64.to_le_bytes()); // amount_0
        data.extend_from_slice(&7_u64.to_le_bytes()); // amount_1
        data.push(swap_mode);
        data.extend_from_slice(&90_u64.to_le_bytes()); // included fee input
        data.extend_from_slice(&88_u64.to_le_bytes()); // excluded fee input
        data.extend_from_slice(&10_u64.to_le_bytes()); // amount left
        data.extend_from_slice(&55_u64.to_le_bytes()); // output
        data.extend_from_slice(&(3_u128 << 64).to_le_bytes()); // next sqrt price
        data.extend_from_slice(&1_u64.to_le_bytes()); // trading fee
        data.extend_from_slice(&2_u64.to_le_bytes()); // protocol fee
        data.extend_from_slice(&3_u64.to_le_bytes()); // referral fee
        data.extend_from_slice(&1_000_u64.to_le_bytes()); // quote reserve
        data.extend_from_slice(&1_000_u64.to_le_bytes()); // migration threshold
        data.extend_from_slice(&123_u64.to_le_bytes()); // timestamp
        data
    }

    #[test]
    fn swap2_and_its_transfer_hook_twin_share_a_layout() {
        let pool = Pubkey::new_from_array([1; 32]);
        let config = Pubkey::new_from_array([2; 32]);
        let body = swap2_body(pool, config, 1);
        for (disc, transfer_hook) in
            [(discriminators::SWAP2, false), (discriminators::SWAP2_WITH_TRANSFER_HOOK, true)]
        {
            let Some(DexEvent::MeteoraDbcSwap(event)) =
                parse(&disc, &body, EventMetadata::default())
            else {
                panic!("expected a DBC swap");
            };
            assert_eq!(event.pool, pool);
            assert_eq!(event.config, config);
            assert!(event.is_buy());
            assert!(event.has_referral);
            assert_eq!(event.transfer_hook, transfer_hook);
            assert_eq!(event.swap_mode, 1);
            assert_eq!((event.amount_0, event.amount_1), (100, 7));
            assert_eq!(event.amount_in, 90);
            assert_eq!(event.actual_input_amount, 88);
            assert_eq!(event.amount_left, 10);
            assert_eq!(event.minimum_amount_out, 7);
            assert_eq!(event.output_amount, 55);
            assert_eq!(event.next_sqrt_price, 3_u128 << 64);
            assert_eq!((event.trading_fee, event.protocol_fee, event.referral_fee), (1, 2, 3));
            assert_eq!(event.quote_reserve_amount, 1_000);
            assert!(event.completed_curve());
            assert_eq!(event.current_timestamp, 123);
        }
    }

    #[test]
    fn exact_out_limit_is_its_output() {
        let body = swap2_body(Pubkey::new_unique(), Pubkey::new_unique(), 2);
        let Some(DexEvent::MeteoraDbcSwap(event)) =
            parse(&discriminators::SWAP2, &body, EventMetadata::default())
        else {
            panic!("expected a DBC swap");
        };
        assert_eq!(event.minimum_amount_out, 100);
    }

    #[test]
    fn legacy_twin_and_truncated_bodies_yield_nothing() {
        let body = swap2_body(Pubkey::new_unique(), Pubkey::new_unique(), 0);
        assert!(parse(&discriminators::SWAP, &body, EventMetadata::default()).is_none());
        assert!(parse(&discriminators::SWAP2, &body[..body.len() - 1], EventMetadata::default())
            .is_none());
    }

    #[test]
    fn curve_complete_variants_parse_alike() {
        let pool = Pubkey::new_from_array([5; 32]);
        let mut body = Vec::new();
        body.extend_from_slice(pool.as_ref());
        body.extend_from_slice(Pubkey::new_from_array([6; 32]).as_ref());
        body.extend_from_slice(&11_u64.to_le_bytes());
        body.extend_from_slice(&22_u64.to_le_bytes());
        for disc in
            [discriminators::CURVE_COMPLETE, discriminators::CURVE_COMPLETE_WITH_TRANSFER_HOOK]
        {
            let Some(DexEvent::MeteoraDbcCurveComplete(event)) =
                parse(&disc, &body, EventMetadata::default())
            else {
                panic!("expected a DBC curve completion");
            };
            assert_eq!(event.pool, pool);
            assert_eq!((event.base_reserve, event.quote_reserve), (11, 22));
        }
    }
}
