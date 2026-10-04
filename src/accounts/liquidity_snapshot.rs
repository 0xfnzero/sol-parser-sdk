//! Subscription snapshots for liquidity/configuration accounts. Carries validated identity
//! and raw state so consumers can preserve every fee/configuration field.
use super::AccountData;
use crate::{core::events::EventMetadata, DexEvent};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LiquidityAccountKind {
    LaunchLabPool {
        base_mint: Pubkey,
        quote_mint: Pubkey,
        global_config: Pubkey,
        platform_config: Pubkey,
    },
    LaunchLabGlobalConfig,
    LaunchLabPlatformConfig,
    DlmmPool {
        token_x_mint: Pubkey,
        token_y_mint: Pubkey,
        active_id: i32,
        bin_step: u16,
    },
    DlmmBinArray {
        pool: Pubkey,
        index: i64,
    },
    OrcaDynamicTickArray {
        pool: Pubkey,
        start_tick_index: i32,
        tick_bitmap: u128,
    },
    OrcaAdaptiveOracle {
        pool: Pubkey,
    },
    ClmmBitmap {
        pool: Pubkey,
    },
    DlmmBitmap {
        pool: Pubkey,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidityAccountSnapshotEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub kind: LiquidityAccountKind,
    /// Current subscription bytes; no historical trade-derived/default fees.
    pub data: Vec<u8>,
}
/// Opt-in raw gRPC update. Includes unchanged configuration/mint bytes,
/// version ordering and account closures, which normalized trade events lack.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawAccountSnapshotEvent {
    pub metadata: EventMetadata,
    pub account: AccountData,
    pub write_version: u64,
    pub is_startup: bool,
}
fn key(data: &[u8], offset: usize) -> Option<Pubkey> {
    Some(Pubkey::new_from_array(data.get(offset..offset + 32)?.try_into().ok()?))
}
pub fn parse_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    let d = &account.data;
    let disc = d.get(..8)?;
    let kind = if account.owner == pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj") {
        match disc {
            [247, 237, 227, 245, 215, 195, 222, 70] if d.len() >= 429 => {
                LiquidityAccountKind::LaunchLabPool {
                    base_mint: key(d, 205)?,
                    quote_mint: key(d, 237)?,
                    global_config: key(d, 141)?,
                    platform_config: key(d, 173)?,
                }
            }
            [149, 8, 156, 202, 160, 252, 176, 217] if d.len() >= 35 => {
                LiquidityAccountKind::LaunchLabGlobalConfig
            }
            [160, 78, 128, 0, 248, 83, 230, 160] if d.len() >= 728 => {
                LiquidityAccountKind::LaunchLabPlatformConfig
            }
            _ => return None,
        }
    } else if account.owner == pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo") {
        match disc {
            [33, 11, 49, 98, 181, 101, 177, 13] if d.len() >= 904 => {
                LiquidityAccountKind::DlmmPool {
                    token_x_mint: key(d, 88)?,
                    token_y_mint: key(d, 120)?,
                    active_id: i32::from_le_bytes(d[76..80].try_into().ok()?),
                    bin_step: u16::from_le_bytes(d[80..82].try_into().ok()?),
                }
            }
            [92, 142, 92, 220, 5, 148, 70, 181] if d.len() >= 10136 => {
                LiquidityAccountKind::DlmmBinArray {
                    pool: key(d, 24)?,
                    index: i64::from_le_bytes(d[8..16].try_into().ok()?),
                }
            }
            [80, 111, 124, 113, 55, 237, 18, 5] if d.len() >= 1576 => {
                LiquidityAccountKind::DlmmBitmap { pool: key(d, 8)? }
            }
            _ => return None,
        }
    } else if account.owner == pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc") {
        match disc {
            [17, 216, 246, 142, 225, 199, 218, 56] if d.len() >= 148 => {
                let bitmap = u128::from_le_bytes(d.get(44..60)?.try_into().ok()?);
                if bitmap >> 88 != 0 {
                    return None;
                }
                let mut offset = 60;
                for i in 0..88 {
                    let tag = *d.get(offset)?;
                    if tag > 1 || (tag == 1) != ((bitmap >> i) & 1 == 1) {
                        return None;
                    }
                    offset += 1 + if tag == 1 { 112 } else { 0 };
                    d.get(..offset)?;
                }
                LiquidityAccountKind::OrcaDynamicTickArray {
                    pool: key(d, 12)?,
                    start_tick_index: i32::from_le_bytes(d[8..12].try_into().ok()?),
                    tick_bitmap: bitmap,
                }
            }
            [139, 194, 131, 179, 140, 179, 229, 244] if d.len() >= 254 => {
                LiquidityAccountKind::OrcaAdaptiveOracle { pool: key(d, 8)? }
            }
            _ => return None,
        }
    } else if account.owner == pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK")
        && disc == [60, 150, 36, 219, 97, 128, 139, 153]
        && d.len() >= 1832
    {
        LiquidityAccountKind::ClmmBitmap { pool: key(d, 8)? }
    } else {
        return None;
    };
    Some(DexEvent::LiquidityAccountSnapshot(Box::new(LiquidityAccountSnapshotEvent {
        metadata,
        pubkey: account.pubkey,
        owner: account.owner,
        kind,
        data: d.clone(),
    })))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concentrated_auxiliary_snapshots_validate_layout_and_filter() {
        let pool = Pubkey::new_unique();
        for (owner, disc, len, pool_offset) in [
            (
                pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
                [17, 216, 246, 142, 225, 199, 218, 56],
                148,
                12,
            ),
            (
                pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
                [139, 194, 131, 179, 140, 179, 229, 244],
                254,
                8,
            ),
            (
                pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"),
                [60, 150, 36, 219, 97, 128, 139, 153],
                1832,
                8,
            ),
        ] {
            let mut data = vec![0; len];
            data[..8].copy_from_slice(&disc);
            data[pool_offset..pool_offset + 32].copy_from_slice(pool.as_ref());
            let mut account = AccountData {
                pubkey: Pubkey::new_unique(),
                owner,
                data,
                lamports: 1,
                executable: false,
                rent_epoch: 0,
            };
            let event = parse_account(&account, EventMetadata::default()).unwrap();
            let filter = crate::grpc::EventTypeFilter::include_only(vec![
                crate::grpc::EventType::AccountLiquiditySnapshot,
            ]);
            assert!(filter.should_include_dex_event(&event));
            account.data.truncate(len - 1);
            assert!(parse_account(&account, EventMetadata::default()).is_none());
        }
        let mut data = vec![0; 148];
        data[..8].copy_from_slice(&[17, 216, 246, 142, 225, 199, 218, 56]);
        data[44] = 1; // Bitmap claims initialized tick; enum tag disagrees.
        let a = AccountData {
            pubkey: pool,
            owner: pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
            data,
            lamports: 1,
            executable: false,
            rent_epoch: 0,
        };
        assert!(parse_account(&a, EventMetadata::default()).is_none());
    }
    #[test]
    fn owner_discriminator_and_lengths_are_required() {
        let owner = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
        let mut d = vec![0; 904];
        d[..8].copy_from_slice(&[33, 11, 49, 98, 181, 101, 177, 13]);
        let mut a = AccountData {
            pubkey: Pubkey::new_unique(),
            owner,
            data: d,
            lamports: 0,
            executable: false,
            rent_epoch: 0,
        };
        let event =
            crate::accounts::parse_account_unified(&a, EventMetadata::default(), None).unwrap();
        assert!(matches!(event, DexEvent::LiquidityAccountSnapshot(_)));
        a.owner = Pubkey::new_unique();
        assert!(parse_account(&a, EventMetadata::default()).is_none());
        a.owner = owner;
        for size in 0..904 {
            let mut short = a.clone();
            short.data.truncate(size);
            assert!(parse_account(&short, EventMetadata::default()).is_none());
        }
    }
}
