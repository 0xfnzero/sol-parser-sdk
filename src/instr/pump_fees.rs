//! Pump Fees（`pfeeUx...`）外层指令：`idls/pump_fees.json`。Shred/gRPC 共用账户索引语义。

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

use crate::core::events::*;
use crate::logs::pump_fees::{read_fee_tiers_vec, read_fees_at, read_shareholders_vec};

pub(crate) const CREATE_FEE_SHARING_IX: [u8; 8] = [195, 78, 86, 76, 111, 52, 251, 213];
pub(crate) const INITIALIZE_FEE_CONFIG_IX: [u8; 8] = [62, 162, 20, 133, 121, 65, 145, 27];
pub(crate) const RESET_FEE_SHARING_IX: [u8; 8] = [10, 2, 182, 95, 16, 127, 129, 186];
pub(crate) const RESET_FEE_SHARING_V2_IX: [u8; 8] = [169, 245, 17, 209, 94, 91, 248, 128];
pub(crate) const REVOKE_FEE_SHARING_IX: [u8; 8] = [18, 233, 158, 39, 185, 207, 58, 104];
pub(crate) const TRANSFER_FEE_SHARING_IX: [u8; 8] = [202, 10, 75, 200, 164, 34, 210, 96];
pub(crate) const UPDATE_ADMIN_IX: [u8; 8] = [161, 176, 40, 213, 60, 184, 179, 228];
pub(crate) const UPDATE_FEE_CONFIG_IX: [u8; 8] = [104, 184, 103, 242, 88, 151, 107, 20];
pub(crate) const UPDATE_FEE_SHARES_IX: [u8; 8] = [189, 13, 136, 99, 187, 164, 237, 35];
pub(crate) const UPDATE_FEE_SHARES_V2_IX: [u8; 8] = [111, 251, 49, 6, 78, 78, 106, 18];
pub(crate) const UPSERT_FEE_TIERS_IX: [u8; 8] = [227, 23, 150, 12, 77, 86, 94, 4];

#[inline(always)]
fn disc8(data: &[u8]) -> Option<[u8; 8]> {
    data.get(..8)?.try_into().ok()
}

#[inline(always)]
fn metadata(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> EventMetadata {
    EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    }
}

#[inline(always)]
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let md = metadata(signature, slot, tx_index, block_time_us, grpc_recv_us);
    let disc = disc8(instruction_data)?;

    if disc == CREATE_FEE_SHARING_IX {
        let payer = *accounts.get(2)?;
        let mint = *accounts.get(4)?;
        let sharing_config = accounts.get(5).copied().unwrap_or_default();
        let bonding_curve = accounts.get(7).copied().unwrap_or_default();
        let pool = accounts.get(10).copied();
        return Some(DexEvent::PumpFeesCreateFeeSharingConfig(
            PumpFeesCreateFeeSharingConfigEvent {
                metadata: md,
                timestamp: 0,
                mint,
                bonding_curve,
                pool,
                sharing_config,
                admin: payer,
                initial_shareholders: Vec::new(),
                status: PumpFeesConfigStatus::Active,
            },
        ));
    }

    if disc == UPDATE_FEE_SHARES_IX || disc == UPDATE_FEE_SHARES_V2_IX {
        if accounts.len() < 8 || instruction_data.len() < 8 {
            return None;
        }
        let authority = *accounts.get(2)?;
        let mint = *accounts.get(4)?;
        let sharing_config = *accounts.get(5)?;
        let bonding_curve = accounts.get(6).copied().unwrap_or_default();
        let pump_creator_vault = accounts.get(7).copied().unwrap_or_default();
        let mut o = 8usize;
        let new_shareholders = read_shareholders_vec(instruction_data, &mut o)?;
        if o != instruction_data.len() {
            return None;
        }
        return Some(DexEvent::PumpFeesUpdateFeeShares(PumpFeesUpdateFeeSharesEvent {
            metadata: md,
            timestamp: 0,
            mint,
            sharing_config,
            admin: authority,
            bonding_curve,
            pump_creator_vault,
            new_shareholders,
        }));
    }

    if disc == INITIALIZE_FEE_CONFIG_IX {
        let admin = *accounts.first()?;
        let fee_config = *accounts.get(1)?;
        return Some(DexEvent::PumpFeesInitializeFeeConfig(PumpFeesInitializeFeeConfigEvent {
            metadata: md,
            timestamp: 0,
            admin,
            fee_config,
        }));
    }

    if disc == RESET_FEE_SHARING_IX || disc == RESET_FEE_SHARING_V2_IX {
        let new_admin = *accounts.first()?;
        let old_admin = *accounts.get(3)?;
        let mint = *accounts.get(5)?;
        let sharing_config = *accounts.get(6)?;
        return Some(DexEvent::PumpFeesResetFeeSharingConfig(PumpFeesResetFeeSharingConfigEvent {
            metadata: md,
            timestamp: 0,
            mint,
            sharing_config,
            old_admin,
            old_shareholders: Vec::new(),
            new_admin,
            new_shareholders: Vec::new(),
        }));
    }

    if disc == REVOKE_FEE_SHARING_IX {
        let admin = *accounts.first()?;
        let mint = *accounts.get(2)?;
        let sharing_config = *accounts.get(3)?;
        return Some(DexEvent::PumpFeesRevokeFeeSharingAuthority(
            PumpFeesRevokeFeeSharingAuthorityEvent {
                metadata: md,
                timestamp: 0,
                mint,
                sharing_config,
                admin,
            },
        ));
    }

    if disc == TRANSFER_FEE_SHARING_IX {
        let old_admin = *accounts.first()?;
        let mint = *accounts.get(2)?;
        let sharing_config = *accounts.get(3)?;
        let new_admin = *accounts.get(4)?;
        return Some(DexEvent::PumpFeesTransferFeeSharingAuthority(
            PumpFeesTransferFeeSharingAuthorityEvent {
                metadata: md,
                timestamp: 0,
                mint,
                sharing_config,
                old_admin,
                new_admin,
            },
        ));
    }

    if disc == UPDATE_ADMIN_IX {
        let old_admin = *accounts.first()?;
        let new_admin = *accounts.get(2)?;
        return Some(DexEvent::PumpFeesUpdateAdmin(PumpFeesUpdateAdminEvent {
            metadata: md,
            timestamp: 0,
            old_admin,
            new_admin,
        }));
    }

    if disc == UPDATE_FEE_CONFIG_IX {
        let fee_config = *accounts.first()?;
        let admin = *accounts.get(1)?;
        if instruction_data.len() < 8 {
            return None;
        }
        let mut o = 8usize;
        let fee_tiers = read_fee_tiers_vec(instruction_data, &mut o)?;
        let flat_fees = read_fees_at(instruction_data, &mut o)?;
        if o != instruction_data.len() {
            return None;
        }
        return Some(DexEvent::PumpFeesUpdateFeeConfig(PumpFeesUpdateFeeConfigEvent {
            metadata: md,
            timestamp: 0,
            admin,
            fee_config,
            fee_tiers,
            flat_fees,
        }));
    }

    if disc == UPSERT_FEE_TIERS_IX {
        let fee_config = *accounts.first()?;
        let admin = *accounts.get(1)?;
        if instruction_data.len() < 8 {
            return None;
        }
        let mut o = 8usize;
        let fee_tiers = read_fee_tiers_vec(instruction_data, &mut o)?;
        let offset = *instruction_data.get(o)?;
        o += 1;
        if o != instruction_data.len() {
            return None;
        }
        return Some(DexEvent::PumpFeesUpsertFeeTiers(PumpFeesUpsertFeeTiersEvent {
            metadata: md,
            timestamp: 0,
            admin,
            fee_config,
            fee_tiers,
            offset,
        }));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn accounts() -> Vec<Pubkey> {
        vec![
            pk(1),  // event_authority
            pk(2),  // program
            pk(3),  // authority
            pk(4),  // global
            pk(5),  // mint
            pk(6),  // sharing_config
            pk(7),  // bonding_curve
            pk(8),  // pump_creator_vault
            pk(9),  // pump_creator_vault_ata (v2) / system_program (v1)
            pk(10), // system_program (v2) / pump_program (v1)
        ]
    }

    fn reset_accounts() -> Vec<Pubkey> {
        vec![
            pk(10), // new_admin
            pk(11), // event_authority
            pk(12), // program
            pk(13), // authority / old admin
            pk(14), // global
            pk(15), // mint
            pk(16), // sharing_config
            pk(17), // bonding_curve
            pk(18), // pump_creator_vault
            pk(19), // v1 system_program / v2 pump_creator_vault_ata
        ]
    }

    fn update_fee_shares_data(disc: [u8; 8]) -> Vec<u8> {
        let mut data = disc.to_vec();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&pk(42).to_bytes());
        data.extend_from_slice(&2500u16.to_le_bytes());
        data
    }

    fn parse_update_fee_shares(disc: [u8; 8]) -> PumpFeesUpdateFeeSharesEvent {
        let event = parse_instruction(
            &update_fee_shares_data(disc),
            &accounts(),
            Signature::default(),
            1,
            0,
            None,
            99,
        )
        .expect("update_fee_shares event");

        match event {
            DexEvent::PumpFeesUpdateFeeShares(e) => e,
            other => panic!("expected PumpFeesUpdateFeeShares, got {other:?}"),
        }
    }

    #[test]
    fn parses_update_fee_shares_v1_ix_vault() {
        let event = parse_update_fee_shares(UPDATE_FEE_SHARES_IX);
        assert_eq!(event.mint, pk(5));
        assert_eq!(event.sharing_config, pk(6));
        assert_eq!(event.admin, pk(3));
        assert_eq!(event.bonding_curve, pk(7));
        assert_eq!(event.pump_creator_vault, pk(8));
        assert_eq!(event.new_shareholders[0].address, pk(42));
        assert_eq!(event.new_shareholders[0].share_bps, 2500);
    }

    #[test]
    fn parses_update_fee_shares_v2_ix_vault() {
        let event = parse_update_fee_shares(UPDATE_FEE_SHARES_V2_IX);
        assert_eq!(event.mint, pk(5));
        assert_eq!(event.sharing_config, pk(6));
        assert_eq!(event.admin, pk(3));
        assert_eq!(event.bonding_curve, pk(7));
        assert_eq!(event.pump_creator_vault, pk(8));
        assert_eq!(event.new_shareholders[0].address, pk(42));
        assert_eq!(event.new_shareholders[0].share_bps, 2500);
    }

    #[test]
    fn parses_reset_fee_sharing_accounts_from_idl_order() {
        for disc in [RESET_FEE_SHARING_IX, RESET_FEE_SHARING_V2_IX] {
            let event =
                parse_instruction(&disc, &reset_accounts(), Signature::default(), 1, 0, None, 99)
                    .expect("reset fee sharing event");

            match event {
                DexEvent::PumpFeesResetFeeSharingConfig(e) => {
                    assert_eq!(e.new_admin, pk(10));
                    assert_eq!(e.old_admin, pk(13));
                    assert_eq!(e.mint, pk(15));
                    assert_eq!(e.sharing_config, pk(16));
                }
                other => panic!("expected PumpFeesResetFeeSharingConfig, got {other:?}"),
            }
        }
    }
}
