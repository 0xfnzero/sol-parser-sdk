//! `pfeeUx...`（pump-fees）Program log `Program data` → [`DexEvent`](crate::core::events::DexEvent)。
//! 判别子与字段布局对齐 `idls/pump_fees.json` Anchor events / types。

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub const CREATE_FEE_SHARING_CONFIG_EVENT_DISC: [u8; 8] = [133, 105, 170, 200, 184, 116, 251, 88];
pub const INITIALIZE_FEE_CONFIG_EVENT_DISC: [u8; 8] = [89, 138, 244, 230, 10, 56, 226, 126];
pub const RESET_FEE_SHARING_CONFIG_EVENT_DISC: [u8; 8] = [203, 204, 151, 226, 120, 55, 214, 243];
pub const REVOKE_FEE_SHARING_AUTHORITY_EVENT_DISC: [u8; 8] = [114, 23, 101, 60, 14, 190, 153, 62];
pub const TRANSFER_FEE_SHARING_AUTHORITY_EVENT_DISC: [u8; 8] =
    [124, 143, 198, 245, 77, 184, 8, 236];
pub const UPDATE_ADMIN_EVENT_DISC: [u8; 8] = [225, 152, 171, 87, 246, 63, 66, 234];
pub const UPDATE_FEE_CONFIG_EVENT_DISC: [u8; 8] = [90, 23, 65, 35, 62, 244, 188, 208];
pub const UPDATE_FEE_SHARES_EVENT_DISC: [u8; 8] = [21, 186, 196, 184, 91, 228, 225, 203];
pub const UPSERT_FEE_TIERS_EVENT_DISC: [u8; 8] = [171, 89, 169, 187, 122, 186, 33, 204];

#[inline(always)]
pub const fn discriminant_u64(disc: &[u8; 8]) -> u64 {
    u64::from_le_bytes(*disc)
}

const MAX_SHAREHOLDERS: usize = 64;
const MAX_FEE_TIERS: usize = 64;

#[inline(always)]
fn read_i64_at(data: &[u8], o: &mut usize) -> Option<i64> {
    if data.len() < *o + 8 {
        return None;
    }
    let v = i64::from_le_bytes(data[*o..*o + 8].try_into().ok()?);
    *o += 8;
    Some(v)
}

#[inline(always)]
fn read_u8_at(data: &[u8], o: &mut usize) -> Option<u8> {
    let v = *data.get(*o)?;
    *o += 1;
    Some(v)
}

#[inline(always)]
fn read_u16_at(data: &[u8], o: &mut usize) -> Option<u16> {
    if data.len() < *o + 2 {
        return None;
    }
    let v = u16::from_le_bytes(data[*o..*o + 2].try_into().ok()?);
    *o += 2;
    Some(v)
}

#[inline(always)]
fn read_u32_at(data: &[u8], o: &mut usize) -> Option<u32> {
    if data.len() < *o + 4 {
        return None;
    }
    let v = u32::from_le_bytes(data[*o..*o + 4].try_into().ok()?);
    *o += 4;
    Some(v)
}

#[inline(always)]
fn read_u64_at(data: &[u8], o: &mut usize) -> Option<u64> {
    if data.len() < *o + 8 {
        return None;
    }
    let v = u64::from_le_bytes(data[*o..*o + 8].try_into().ok()?);
    *o += 8;
    Some(v)
}

#[inline(always)]
fn read_u128_at(data: &[u8], o: &mut usize) -> Option<u128> {
    if data.len() < *o + 16 {
        return None;
    }
    let v = u128::from_le_bytes(data[*o..*o + 16].try_into().ok()?);
    *o += 16;
    Some(v)
}

#[inline(always)]
fn read_pubkey_at(data: &[u8], o: &mut usize) -> Option<Pubkey> {
    if data.len() < *o + 32 {
        return None;
    }
    let pk = Pubkey::new_from_array(data[*o..*o + 32].try_into().ok()?);
    *o += 32;
    Some(pk)
}

#[inline(always)]
fn read_option_pubkey_at(data: &[u8], o: &mut usize) -> Option<Option<Pubkey>> {
    let tag = *data.get(*o)?;
    *o += 1;
    match tag {
        0 => Some(None),
        1 => Some(Some(read_pubkey_at(data, o)?)),
        _ => None,
    }
}

#[inline(always)]
fn read_config_status_at(data: &[u8], o: &mut usize) -> Option<PumpFeesConfigStatus> {
    let b = *data.get(*o)?;
    *o += 1;
    match b {
        0 => Some(PumpFeesConfigStatus::Paused),
        1 => Some(PumpFeesConfigStatus::Active),
        _ => None,
    }
}

#[inline(always)]
pub(crate) fn read_fees_at(data: &[u8], o: &mut usize) -> Option<PumpFeesFees> {
    Some(PumpFeesFees {
        lp_fee_bps: read_u64_at(data, o)?,
        protocol_fee_bps: read_u64_at(data, o)?,
        creator_fee_bps: read_u64_at(data, o)?,
    })
}

#[inline(always)]
pub(crate) fn read_shareholders_vec(
    data: &[u8],
    o: &mut usize,
) -> Option<Vec<PumpFeesShareholder>> {
    let n = read_u32_at(data, o)? as usize;
    if n > MAX_SHAREHOLDERS {
        return None;
    }
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(PumpFeesShareholder {
            address: read_pubkey_at(data, o)?,
            share_bps: read_u16_at(data, o)?,
        });
    }
    Some(v)
}

#[inline(always)]
pub(crate) fn read_fee_tiers_vec(data: &[u8], o: &mut usize) -> Option<Vec<PumpFeesFeeTier>> {
    let n = read_u32_at(data, o)? as usize;
    if n > MAX_FEE_TIERS {
        return None;
    }
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(PumpFeesFeeTier {
            market_cap_lamports_threshold: read_u128_at(data, o)?,
            fees: read_fees_at(data, o)?,
        });
    }
    Some(v)
}

/// `CreateFeeSharingConfigEvent`：数据为去掉 8 字节 discriminator 后的 Borsh 体。
#[inline]
pub fn parse_create_fee_sharing_config_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let mint = read_pubkey_at(data, &mut o)?;
    let bonding_curve = read_pubkey_at(data, &mut o)?;
    let pool = read_option_pubkey_at(data, &mut o)?;
    let sharing_config = read_pubkey_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    let initial_shareholders = read_shareholders_vec(data, &mut o)?;
    let status = read_config_status_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesCreateFeeSharingConfig(PumpFeesCreateFeeSharingConfigEvent {
        metadata,
        timestamp,
        mint,
        bonding_curve,
        pool,
        sharing_config,
        admin,
        initial_shareholders,
        status,
    }))
}

#[inline]
pub fn parse_initialize_fee_config_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    let fee_config = read_pubkey_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesInitializeFeeConfig(PumpFeesInitializeFeeConfigEvent {
        metadata,
        timestamp,
        admin,
        fee_config,
    }))
}

#[inline]
pub fn parse_reset_fee_sharing_config_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let mint = read_pubkey_at(data, &mut o)?;
    let sharing_config = read_pubkey_at(data, &mut o)?;
    let old_admin = read_pubkey_at(data, &mut o)?;
    let old_shareholders = read_shareholders_vec(data, &mut o)?;
    let new_admin = read_pubkey_at(data, &mut o)?;
    let new_shareholders = read_shareholders_vec(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesResetFeeSharingConfig(PumpFeesResetFeeSharingConfigEvent {
        metadata,
        timestamp,
        mint,
        sharing_config,
        old_admin,
        old_shareholders,
        new_admin,
        new_shareholders,
    }))
}

#[inline]
pub fn parse_revoke_fee_sharing_authority_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let mint = read_pubkey_at(data, &mut o)?;
    let sharing_config = read_pubkey_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesRevokeFeeSharingAuthority(PumpFeesRevokeFeeSharingAuthorityEvent {
        metadata,
        timestamp,
        mint,
        sharing_config,
        admin,
    }))
}

#[inline]
pub fn parse_transfer_fee_sharing_authority_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let mint = read_pubkey_at(data, &mut o)?;
    let sharing_config = read_pubkey_at(data, &mut o)?;
    let old_admin = read_pubkey_at(data, &mut o)?;
    let new_admin = read_pubkey_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesTransferFeeSharingAuthority(PumpFeesTransferFeeSharingAuthorityEvent {
        metadata,
        timestamp,
        mint,
        sharing_config,
        old_admin,
        new_admin,
    }))
}

#[inline]
pub fn parse_update_admin_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let old_admin = read_pubkey_at(data, &mut o)?;
    let new_admin = read_pubkey_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesUpdateAdmin(PumpFeesUpdateAdminEvent {
        metadata,
        timestamp,
        old_admin,
        new_admin,
    }))
}

#[inline]
pub fn parse_update_fee_config_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    let fee_config = read_pubkey_at(data, &mut o)?;
    let fee_tiers = read_fee_tiers_vec(data, &mut o)?;
    let flat_fees = read_fees_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesUpdateFeeConfig(PumpFeesUpdateFeeConfigEvent {
        metadata,
        timestamp,
        admin,
        fee_config,
        fee_tiers,
        flat_fees,
    }))
}

#[inline]
pub fn parse_update_fee_shares_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let mint = read_pubkey_at(data, &mut o)?;
    let sharing_config = read_pubkey_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    let new_shareholders = read_shareholders_vec(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesUpdateFeeShares(PumpFeesUpdateFeeSharesEvent {
        metadata,
        timestamp,
        mint,
        sharing_config,
        admin,
        bonding_curve: Pubkey::default(),
        pump_creator_vault: Pubkey::default(),
        new_shareholders,
    }))
}

#[inline]
pub fn parse_upsert_fee_tiers_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut o = 0usize;
    let timestamp = read_i64_at(data, &mut o)?;
    let admin = read_pubkey_at(data, &mut o)?;
    let fee_config = read_pubkey_at(data, &mut o)?;
    let fee_tiers = read_fee_tiers_vec(data, &mut o)?;
    let offset = read_u8_at(data, &mut o)?;
    if o != data.len() {
        return None;
    }
    Some(DexEvent::PumpFeesUpsertFeeTiers(PumpFeesUpsertFeeTiersEvent {
        metadata,
        timestamp,
        admin,
        fee_config,
        fee_tiers,
        offset,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Signature;

    #[test]
    fn create_fee_sharing_roundtrip() {
        let ts: i64 = 1_777_920_719;
        let mint = Pubkey::new_unique();
        let bonding_curve = Pubkey::new_unique();
        let sharing_config = Pubkey::new_unique();
        let admin = Pubkey::new_unique();
        let sh_addr = Pubkey::new_unique();
        let mut buf = Vec::new();
        buf.extend_from_slice(&ts.to_le_bytes());
        buf.extend_from_slice(mint.as_ref());
        buf.extend_from_slice(bonding_curve.as_ref());
        buf.push(0u8); // pool None
        buf.extend_from_slice(sharing_config.as_ref());
        buf.extend_from_slice(admin.as_ref());
        buf.extend_from_slice(&(1u32).to_le_bytes());
        buf.extend_from_slice(sh_addr.as_ref());
        buf.extend_from_slice(&(10_000u16).to_le_bytes());
        buf.push(1u8); // Active
        let md = EventMetadata {
            signature: Signature::default(),
            slot: 0,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let ev = parse_create_fee_sharing_config_from_data(&buf, md).unwrap();
        match ev {
            DexEvent::PumpFeesCreateFeeSharingConfig(e) => {
                assert_eq!(e.timestamp, ts);
                assert_eq!(e.initial_shareholders[0].address, sh_addr);
                assert_eq!(e.status, PumpFeesConfigStatus::Active);
            }
            _ => panic!("variant"),
        }
    }

    #[test]
    fn update_fee_shares_roundtrip_program_data() {
        let mint = Pubkey::new_unique();
        let cfg = Pubkey::new_unique();
        let adm = Pubkey::new_unique();
        let mut buf = Vec::new();
        buf.extend_from_slice(&100i64.to_le_bytes());
        buf.extend_from_slice(mint.as_ref());
        buf.extend_from_slice(cfg.as_ref());
        buf.extend_from_slice(adm.as_ref());
        buf.extend_from_slice(&(0u32).to_le_bytes());
        let md = EventMetadata {
            signature: Signature::default(),
            slot: 0,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        assert!(matches!(
            parse_update_fee_shares_from_data(&buf, md),
            Some(DexEvent::PumpFeesUpdateFeeShares(_))
        ));
    }
}
