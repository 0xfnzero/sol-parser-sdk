use crate::core::events::{DexEvent, EventMetadata};

const EVENT_CPI_PREFIX: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];
const EVENT_CPI_SUFFIX: [u8; 8] = [155, 167, 108, 32, 122, 76, 173, 64];

#[inline(always)]
fn event_discriminator(disc: &[u8; 16]) -> Option<[u8; 8]> {
    if disc[..8] == EVENT_CPI_PREFIX {
        return disc[8..16].try_into().ok();
    }
    if disc[8..16] == EVENT_CPI_SUFFIX {
        return disc[..8].try_into().ok();
    }
    None
}

#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let event_disc = event_discriminator(disc)?;
    match event_disc {
        crate::logs::pump_fees::CREATE_FEE_SHARING_CONFIG_EVENT_DISC => {
            crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(data, metadata)
        }
        crate::logs::pump_fees::INITIALIZE_FEE_CONFIG_EVENT_DISC => {
            crate::logs::pump_fees::parse_initialize_fee_config_from_data(data, metadata)
        }
        crate::logs::pump_fees::RESET_FEE_SHARING_CONFIG_EVENT_DISC => {
            crate::logs::pump_fees::parse_reset_fee_sharing_config_from_data(data, metadata)
        }
        crate::logs::pump_fees::REVOKE_FEE_SHARING_AUTHORITY_EVENT_DISC => {
            crate::logs::pump_fees::parse_revoke_fee_sharing_authority_from_data(data, metadata)
        }
        crate::logs::pump_fees::TRANSFER_FEE_SHARING_AUTHORITY_EVENT_DISC => {
            crate::logs::pump_fees::parse_transfer_fee_sharing_authority_from_data(data, metadata)
        }
        crate::logs::pump_fees::UPDATE_ADMIN_EVENT_DISC => {
            crate::logs::pump_fees::parse_update_admin_from_data(data, metadata)
        }
        crate::logs::pump_fees::UPDATE_FEE_CONFIG_EVENT_DISC => {
            crate::logs::pump_fees::parse_update_fee_config_from_data(data, metadata)
        }
        crate::logs::pump_fees::UPDATE_FEE_SHARES_EVENT_DISC => {
            crate::logs::pump_fees::parse_update_fee_shares_from_data(data, metadata)
        }
        crate::logs::pump_fees::UPSERT_FEE_TIERS_EVENT_DISC => {
            crate::logs::pump_fees::parse_upsert_fee_tiers_from_data(data, metadata)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::DexEvent;
    use solana_sdk::pubkey::Pubkey;

    fn update_admin_data(old_admin: Pubkey, new_admin: Pubkey) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&123i64.to_le_bytes());
        data.extend_from_slice(old_admin.as_ref());
        data.extend_from_slice(new_admin.as_ref());
        data
    }

    #[test]
    fn parses_prefix_and_suffix_event_cpi_layouts() {
        let old_admin = Pubkey::new_unique();
        let new_admin = Pubkey::new_unique();
        let data = update_admin_data(old_admin, new_admin);

        let mut prefix_disc = [0u8; 16];
        prefix_disc[..8].copy_from_slice(&EVENT_CPI_PREFIX);
        prefix_disc[8..].copy_from_slice(&crate::logs::pump_fees::UPDATE_ADMIN_EVENT_DISC);

        let mut suffix_disc = [0u8; 16];
        suffix_disc[..8].copy_from_slice(&crate::logs::pump_fees::UPDATE_ADMIN_EVENT_DISC);
        suffix_disc[8..].copy_from_slice(&EVENT_CPI_SUFFIX);

        for disc in [prefix_disc, suffix_disc] {
            let event = parse(&disc, &data, EventMetadata::default())
                .expect("pump-fees update-admin CPI event should parse");
            match event {
                DexEvent::PumpFeesUpdateAdmin(event) => {
                    assert_eq!(event.timestamp, 123);
                    assert_eq!(event.old_admin, old_admin);
                    assert_eq!(event.new_admin, new_admin);
                }
                other => panic!("expected PumpFeesUpdateAdmin, got {other:?}"),
            }
        }
    }
}
