use crate::core::events::{DexEvent, EventMetadata};

pub mod discriminators {
    pub const SWAP_BASE_IN: [u8; 16] =
        [143, 190, 90, 218, 196, 30, 51, 222, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const SWAP_BASE_OUT: [u8; 16] =
        [55, 217, 98, 86, 163, 74, 180, 173, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CREATE_POOL: [u8; 16] =
        [233, 146, 209, 142, 207, 104, 64, 188, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const DEPOSIT: [u8; 16] =
        [242, 35, 198, 137, 82, 225, 242, 182, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const WITHDRAW: [u8; 16] =
        [183, 18, 70, 156, 148, 109, 161, 34, 155, 167, 108, 32, 122, 76, 173, 64];
}

#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::SWAP_BASE_IN => {
            crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::SWAP_BASE_OUT => {
            crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
        }
        discriminators::CREATE_POOL => {
            crate::logs::raydium_cpmm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::DEPOSIT => {
            crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
        }
        discriminators::WITHDRAW => {
            crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
        }
        _ => None,
    }
}
