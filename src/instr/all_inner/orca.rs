use crate::core::events::{DexEvent, EventMetadata};

pub mod discriminators {
    pub const TRADED: [u8; 16] =
        [225, 202, 73, 175, 147, 43, 160, 150, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const LIQUIDITY_INCREASED: [u8; 16] =
        [30, 7, 144, 181, 102, 254, 155, 161, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const LIQUIDITY_DECREASED: [u8; 16] =
        [166, 1, 36, 71, 112, 202, 181, 171, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const POOL_INITIALIZED: [u8; 16] =
        [100, 118, 173, 87, 12, 198, 254, 229, 155, 167, 108, 32, 122, 76, 173, 64];
}

#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::TRADED => {
            crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
        }
        discriminators::LIQUIDITY_INCREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
        }
        discriminators::LIQUIDITY_DECREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
        }
        discriminators::POOL_INITIALIZED => {
            crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
        }
        _ => None,
    }
}
