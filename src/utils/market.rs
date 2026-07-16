use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedTradeSide {
    Buy,
    Sell,
}

impl NormalizedTradeSide {
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
        }
    }
}

/// Convert a Q64.64 sqrt price into a UI price.
///
/// The returned price is quote-token units per one base token, adjusted by
/// token decimals: `(sqrt_price_x64 / 2^64)^2 * 10^(base_decimals - quote_decimals)`.
#[inline]
pub fn sqrt_price_x64_to_price(sqrt_price_x64: u128, base_decimals: u8, quote_decimals: u8) -> f64 {
    let sqrt = sqrt_price_x64 as f64 / 2_f64.powi(64);
    let decimal_adjustment = 10_f64.powi(i32::from(base_decimals) - i32::from(quote_decimals));
    sqrt * sqrt * decimal_adjustment
}

/// Compute quote-token price per one base token from raw vault balances.
#[inline]
pub fn vault_price_from_balances(
    base_raw: u128,
    quote_raw: u128,
    base_decimals: u8,
    quote_decimals: u8,
) -> Option<f64> {
    if base_raw == 0 {
        return None;
    }
    let decimal_adjustment = 10_f64.powi(i32::from(base_decimals) - i32::from(quote_decimals));
    Some((quote_raw as f64 / base_raw as f64) * decimal_adjustment)
}

/// Normalize a watched/base token balance delta into buy/sell direction.
///
/// Positive means the wallet received the watched token (Buy). Negative means
/// the wallet spent the watched token (Sell).
#[inline]
pub fn normalize_buy_sell_from_token_delta(token_delta: i128) -> Option<NormalizedTradeSide> {
    match token_delta.cmp(&0) {
        std::cmp::Ordering::Greater => Some(NormalizedTradeSide::Buy),
        std::cmp::Ordering::Less => Some(NormalizedTradeSide::Sell),
        std::cmp::Ordering::Equal => None,
    }
}

/// Normalize direction from the input mint when base/quote mints are known.
///
/// If the input mint is quote, the user buys base. If the input mint is base,
/// the user sells base.
#[inline]
pub fn normalize_buy_sell_from_input_mint(
    input_mint: &str,
    base_mint: &str,
    quote_mint: &str,
) -> Option<NormalizedTradeSide> {
    if base_mint == quote_mint {
        return None;
    }
    if input_mint == quote_mint {
        Some(NormalizedTradeSide::Buy)
    } else if input_mint == base_mint {
        Some(NormalizedTradeSide::Sell)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_price_x64_identity_price() {
        let q64 = 1_u128 << 64;
        assert_eq!(sqrt_price_x64_to_price(q64, 6, 6), 1.0);
        assert_eq!(sqrt_price_x64_to_price(q64, 9, 6), 1000.0);
    }

    #[test]
    fn vault_price_handles_decimals_and_zero_base() {
        assert_eq!(vault_price_from_balances(1_000_000, 2_000_000, 6, 6), Some(2.0));
        assert_eq!(vault_price_from_balances(1_000_000_000, 2_000_000, 9, 6), Some(2.0));
        assert_eq!(vault_price_from_balances(0, 2_000_000, 6, 6), None);
    }

    #[test]
    fn direction_helpers_are_token_relative() {
        assert_eq!(normalize_buy_sell_from_token_delta(1), Some(NormalizedTradeSide::Buy));
        assert_eq!(normalize_buy_sell_from_token_delta(-1), Some(NormalizedTradeSide::Sell));
        assert_eq!(normalize_buy_sell_from_token_delta(0), None);
        assert_eq!(
            normalize_buy_sell_from_input_mint("USDC", "SOL", "USDC"),
            Some(NormalizedTradeSide::Buy)
        );
        assert_eq!(
            normalize_buy_sell_from_input_mint("SOL", "SOL", "USDC"),
            Some(NormalizedTradeSide::Sell)
        );
    }
}
