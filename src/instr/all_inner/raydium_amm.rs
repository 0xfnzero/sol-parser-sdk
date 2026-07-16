use crate::core::events::{DexEvent, EventMetadata};

pub mod discriminators {
    pub const SWAP_BASE_IN: [u8; 16] =
        [0, 0, 0, 0, 0, 0, 0, 9, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const SWAP_BASE_OUT: [u8; 16] =
        [0, 0, 0, 0, 0, 0, 0, 11, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const DEPOSIT: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 3, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const WITHDRAW: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 4, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const INITIALIZE2: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 1, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const WITHDRAW_PNL: [u8; 16] =
        [0, 0, 0, 0, 0, 0, 0, 7, 155, 167, 108, 32, 122, 76, 173, 64];
}

#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::SWAP_BASE_IN => {
            crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::SWAP_BASE_OUT => {
            crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
        }
        discriminators::DEPOSIT => {
            crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
        }
        discriminators::WITHDRAW => {
            crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
        }
        discriminators::INITIALIZE2 => {
            crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
        }
        discriminators::WITHDRAW_PNL => {
            crate::logs::raydium_amm::parse_withdraw_pnl_from_data(data, metadata)
        }
        _ => None,
    }
}
