use crate::core::events::{DexEvent, EventMetadata};

pub mod discriminators {
    pub const SWAP: [u8; 16] =
        [81, 108, 227, 190, 205, 208, 10, 196, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const ADD_LIQUIDITY: [u8; 16] =
        [31, 94, 125, 90, 227, 52, 61, 186, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const REMOVE_LIQUIDITY: [u8; 16] =
        [116, 244, 97, 232, 103, 31, 152, 58, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const BOOTSTRAP_LIQUIDITY: [u8; 16] =
        [121, 127, 38, 136, 92, 55, 14, 247, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const POOL_CREATED: [u8; 16] =
        [202, 44, 41, 88, 104, 220, 157, 82, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const SET_POOL_FEES: [u8; 16] =
        [245, 26, 198, 164, 88, 18, 75, 9, 155, 167, 108, 32, 122, 76, 173, 64];
}

#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::SWAP => crate::logs::meteora_amm::parse_swap_from_data(data, metadata),
        discriminators::ADD_LIQUIDITY => {
            crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::REMOVE_LIQUIDITY => {
            crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
        }
        discriminators::BOOTSTRAP_LIQUIDITY => {
            crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
        }
        discriminators::POOL_CREATED => {
            crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
        }
        discriminators::SET_POOL_FEES => {
            crate::logs::meteora_amm::parse_set_pool_fees_from_data(data, metadata)
        }
        _ => None,
    }
}
