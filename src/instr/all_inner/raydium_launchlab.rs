use crate::core::events::{DexEvent, EventMetadata};

// Raydium LaunchLab CPI event parser.
//
// CPI event data uses a 16-byte prefix: 8-byte event discriminator from
// `idls/raydium_launchpad.json` plus Anchor's event CPI marker.
pub mod discriminators {
    pub const POOL_CREATE: [u8; 16] =
        [151, 215, 226, 9, 118, 161, 115, 174, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const TRADE: [u8; 16] =
        [189, 219, 127, 211, 78, 230, 97, 238, 155, 167, 108, 32, 122, 76, 173, 64];
}

/// Parse Raydium LaunchLab CPI event data.
#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::TRADE => {
            crate::logs::raydium_launchlab::parse_trade_from_data(data, metadata)
        }
        discriminators::POOL_CREATE => {
            crate::logs::raydium_launchlab::parse_pool_create_from_data(data, metadata)
        }
        _ => None,
    }
}
