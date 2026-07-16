use crate::core::events::{DexEvent, EventMetadata};

pub mod discriminators {
    pub const SWAP: [u8; 16] =
        [64, 198, 205, 232, 38, 8, 113, 226, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const INCREASE_LIQUIDITY: [u8; 16] =
        [49, 79, 105, 212, 32, 34, 30, 84, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const DECREASE_LIQUIDITY: [u8; 16] =
        [58, 222, 86, 58, 68, 50, 85, 56, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const LIQUIDITY_CHANGE: [u8; 16] =
        [126, 240, 175, 206, 158, 88, 153, 107, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CONFIG_CHANGE: [u8; 16] =
        [247, 189, 7, 119, 106, 112, 95, 151, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CREATE_PERSONAL_POSITION: [u8; 16] =
        [100, 30, 87, 249, 196, 223, 154, 206, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const LIQUIDITY_CALCULATE: [u8; 16] =
        [237, 112, 148, 230, 57, 84, 180, 162, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const OPEN_LIMIT_ORDER: [u8; 16] =
        [106, 24, 71, 85, 57, 169, 158, 216, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const INCREASE_LIMIT_ORDER: [u8; 16] =
        [11, 120, 13, 204, 199, 87, 19, 200, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const DECREASE_LIMIT_ORDER: [u8; 16] =
        [70, 48, 40, 221, 219, 237, 212, 163, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const SETTLE_LIMIT_ORDER: [u8; 16] =
        [88, 119, 77, 164, 125, 124, 10, 194, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const UPDATE_REWARD_INFOS: [u8; 16] =
        [109, 127, 186, 78, 114, 65, 37, 236, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CREATE_POOL: [u8; 16] =
        [25, 94, 75, 47, 112, 99, 53, 63, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const COLLECT_PERSONAL_FEE: [u8; 16] =
        [166, 174, 105, 192, 81, 161, 83, 105, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const COLLECT_PROTOCOL_FEE: [u8; 16] =
        [206, 87, 17, 79, 45, 41, 213, 61, 155, 167, 108, 32, 122, 76, 173, 64];
}

#[inline]
pub fn parse_raydium_clmm_inner_instruction(
    discriminator: &[u8; 16],
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    match *discriminator {
        discriminators::SWAP => crate::logs::raydium_clmm::parse_swap_from_data(data, metadata),
        discriminators::INCREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
        }
        discriminators::DECREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
        }
        discriminators::LIQUIDITY_CHANGE => {
            crate::logs::raydium_clmm::parse_liquidity_change_from_data(data, metadata)
        }
        discriminators::CONFIG_CHANGE => {
            crate::logs::raydium_clmm::parse_config_change_from_data(data, metadata)
        }
        discriminators::CREATE_PERSONAL_POSITION => {
            crate::logs::raydium_clmm::parse_create_personal_position_from_data(data, metadata)
        }
        discriminators::LIQUIDITY_CALCULATE => {
            crate::logs::raydium_clmm::parse_liquidity_calculate_from_data(data, metadata)
        }
        discriminators::OPEN_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_open_limit_order_from_data(data, metadata)
        }
        discriminators::INCREASE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_increase_limit_order_from_data(data, metadata)
        }
        discriminators::DECREASE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_decrease_limit_order_from_data(data, metadata)
        }
        discriminators::SETTLE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_settle_limit_order_from_data(data, metadata)
        }
        discriminators::UPDATE_REWARD_INFOS => {
            crate::logs::raydium_clmm::parse_update_reward_infos_from_data(data, metadata)
        }
        discriminators::CREATE_POOL => {
            crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::COLLECT_PERSONAL_FEE => {
            crate::logs::raydium_clmm::parse_collect_personal_fee_from_data(data, metadata)
        }
        discriminators::COLLECT_PROTOCOL_FEE => {
            crate::logs::raydium_clmm::parse_collect_protocol_fee_from_data(data, metadata)
        }
        _ => None,
    }
}
