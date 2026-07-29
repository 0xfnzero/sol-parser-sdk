//! PumpSwap (Pump AMM) Inner Instruction 解析器
//!
//! Inner instructions 使用 16 字节的 discriminator
//!
//! Buy and Sell CPI events use the same validated decoder as PumpSwap logs.
//! This keeps the Borsh and zero-copy feature configurations behaviorally
//! identical while retaining unchecked fixed-field reads after one bounds check.

use crate::core::events::*;
use crate::instr::inner_common::*;

/// PumpSwap inner instruction discriminators (16 bytes)
/// Format: [event_magic (8 bytes) | event_discriminator (8 bytes)]
/// The magic prefix is: [228, 69, 165, 46, 81, 203, 154, 29]
/// The event_discriminator matches the 8-byte log discriminator for each event type
pub mod discriminators {
    /// Common magic prefix for all PumpSwap inner instructions
    #[allow(dead_code)]
    const MAGIC_PREFIX: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

    /// BuyEvent
    /// Full discriminator: MAGIC_PREFIX + [103, 244, 82, 31, 44, 245, 119, 119]
    pub const BUY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        103, 244, 82, 31, 44, 245, 119, 119, // BuyEvent hash
    ];

    /// SellEvent
    /// Full discriminator: MAGIC_PREFIX + [62, 47, 55, 10, 165, 3, 220, 42]
    pub const SELL: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        62, 47, 55, 10, 165, 3, 220, 42, // SellEvent hash
    ];

    /// CreatePoolEvent
    /// Full discriminator: MAGIC_PREFIX + [177, 49, 12, 210, 160, 118, 167, 116]
    pub const CREATE_POOL: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        177, 49, 12, 210, 160, 118, 167, 116, // CreatePoolEvent hash
    ];

    /// DepositEvent (Add Liquidity)
    /// Full discriminator: MAGIC_PREFIX + [120, 248, 61, 83, 31, 142, 107, 144]
    pub const ADD_LIQUIDITY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        120, 248, 61, 83, 31, 142, 107, 144, // AddLiquidityEvent hash
    ];

    /// WithdrawEvent (Remove Liquidity)
    /// Full discriminator: MAGIC_PREFIX + [22, 9, 133, 26, 160, 44, 71, 192]
    pub const REMOVE_LIQUIDITY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        22, 9, 133, 26, 160, 44, 71, 192, // RemoveLiquidityEvent hash
    ];
}

/// 解析 PumpSwap inner instruction (统一入口)
#[inline]
pub fn parse_pumpswap_inner_instruction(
    discriminator: &[u8; 16],
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    match *discriminator {
        discriminators::BUY => parse_buy_inner(data, metadata),
        discriminators::SELL => parse_sell_inner(data, metadata),
        discriminators::CREATE_POOL => parse_create_pool_inner(data, metadata),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity_inner(data, metadata),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity_inner(data, metadata),
        _ => None,
    }
}

// ============================================================================
// Buy 事件解析器
// ============================================================================

/// Parse BuyEvent through the shared log/inner decoder.
#[inline(always)]
fn parse_buy_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_buy_from_data(data, metadata)
}

// ============================================================================
// Sell 事件解析器
// ============================================================================

/// Parse SellEvent through the shared log/inner decoder.
#[inline(always)]
fn parse_sell_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut event = crate::logs::pump_amm::parse_sell_from_data(data, metadata)?;
    if let DexEvent::PumpSwapSell(sell) = &mut event {
        sell.is_pump_pool = true;
    }
    Some(event)
}

/// 解析 CreatePool 事件
#[inline(always)]
fn parse_create_pool_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 32 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let quote_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
            metadata,
            pool,
            creator,
            base_mint,
            quote_mint,
            base_amount_in: base_amount,
            quote_amount_in: quote_amount,
            ..Default::default()
        }))
    }
}

/// 解析 AddLiquidity 事件
#[inline(always)]
fn parse_add_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let _pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let _user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_amount = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapLiquidityAdded(PumpSwapLiquidityAdded {
            metadata,
            base_amount_in: base_amount,
            quote_amount_in: quote_amount,
            lp_token_amount_out: lp_amount,
            ..Default::default()
        }))
    }
}

/// 解析 RemoveLiquidity 事件
#[inline(always)]
fn parse_remove_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let _pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let _user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let lp_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let base_amount_out = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_out = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved {
            metadata,
            lp_token_amount_in: lp_amount,
            base_amount_out,
            quote_amount_out,
            ..Default::default()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_trade_decoder_is_used_without_feature_specific_layouts() {
        let buy = parse_pumpswap_inner_instruction(
            &discriminators::BUY,
            &[0u8; 397],
            EventMetadata::default(),
        )
        .expect("legacy buy event should parse");
        let DexEvent::PumpSwapBuy(buy) = buy else {
            panic!("expected PumpSwapBuy event");
        };
        assert_eq!(buy.virtual_quote_reserves, 0);

        let sell = parse_pumpswap_inner_instruction(
            &discriminators::SELL,
            &[0u8; 352],
            EventMetadata::default(),
        )
        .expect("legacy sell event should parse");
        let DexEvent::PumpSwapSell(sell) = sell else {
            panic!("expected PumpSwapSell event");
        };
        assert!(sell.is_pump_pool);
        assert_eq!(sell.virtual_quote_reserves, 0);

        let mut current_sell = vec![0u8; 409];
        current_sell[384..400].copy_from_slice(&(-987_654_321i128).to_le_bytes());
        current_sell[400] = 1;
        current_sell[401..409].copy_from_slice(&42u64.to_le_bytes());
        let sell = parse_pumpswap_inner_instruction(
            &discriminators::SELL,
            &current_sell,
            EventMetadata::default(),
        )
        .expect("current sell event should parse");
        let DexEvent::PumpSwapSell(sell) = sell else {
            panic!("expected PumpSwapSell event");
        };
        assert!(sell.is_pump_pool);
        assert_eq!(sell.virtual_quote_reserves, -987_654_321);
        assert!(sell.can_boost);
        assert_eq!(sell.base_supply, 42);
    }
}
