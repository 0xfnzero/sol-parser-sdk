pub mod nonce;
pub mod orca_whirlpool;
pub mod program_ids;
pub mod pumpswap;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod rpc_wallet;
pub mod token;
pub mod utils;
use crate::core::events::EventMetadata;
use crate::grpc::EventTypeFilter;
use crate::DexEvent;
pub use nonce::parse_nonce_account;
use program_ids::*;
pub use pumpswap::{
    parse_global_config as parse_pumpswap_global_config, parse_pool as parse_pumpswap_pool,
};
pub use rpc_wallet::rpc_resolve_user_wallet_pubkey;
pub use token::parse_token_account;
pub use token::AccountData;
pub use utils::*;

#[inline(always)]
fn filter_parsed_event(
    event: Option<DexEvent>,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    let event = event?;
    if event_type_filter.map(|f| f.should_include_dex_event(&event)).unwrap_or(true) {
        Some(event)
    } else {
        None
    }
}

pub fn parse_account_unified(
    account: &AccountData,
    metadata: EventMetadata,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    if account.data.is_empty() {
        return None;
    }

    // Early filtering based on event type filter
    if let Some(filter) = event_type_filter {
        if let Some(ref include_only) = filter.include_only {
            // Check if any of the account event types are in the include list
            let should_parse = include_only.iter().any(|t| {
                use crate::grpc::EventType;
                matches!(
                    t,
                    EventType::TokenAccount
                        | EventType::TokenInfo
                        | EventType::NonceAccount
                        | EventType::AccountPumpFunGlobal
                        | EventType::AccountPumpFunBondingCurve
                        | EventType::AccountPumpFunFeeConfig
                        | EventType::AccountPumpFunSharingConfig
                        | EventType::AccountPumpFunGlobalVolumeAccumulator
                        | EventType::AccountPumpFunUserVolumeAccumulator
                        | EventType::AccountPumpSwapGlobalConfig
                        | EventType::AccountPumpSwapPool
                        | EventType::AccountRaydiumClmmAmmConfig
                        | EventType::AccountRaydiumClmmPoolState
                        | EventType::AccountRaydiumClmmTickArrayState
                        | EventType::AccountRaydiumCpmmAmmConfig
                        | EventType::AccountRaydiumCpmmPoolState
                        | EventType::AccountOrcaWhirlpool
                        | EventType::AccountOrcaPosition
                        | EventType::AccountOrcaTickArray
                        | EventType::AccountOrcaFeeTier
                        | EventType::AccountOrcaWhirlpoolsConfig
                )
            });
            if !should_parse {
                return None;
            }
        }
    }

    if account.owner == PUMPSWAP_PROGRAM_ID {
        let should_parse = event_type_filter.is_none_or(|filter| {
            filter.should_include(crate::grpc::EventType::AccountPumpSwapGlobalConfig)
                || filter.should_include(crate::grpc::EventType::AccountPumpSwapPool)
        });
        if should_parse {
            let event = filter_parsed_event(
                parse_pumpswap_account(account, metadata.clone()),
                event_type_filter,
            );
            if event.is_some() {
                return event;
            }
        }
        return None;
    }
    if account.owner == crate::instr::program_ids::RAYDIUM_CLMM_PROGRAM_ID {
        let should_parse = event_type_filter.is_none_or(|filter| {
            filter.should_include(crate::grpc::EventType::AccountRaydiumClmmAmmConfig)
                || filter.should_include(crate::grpc::EventType::AccountRaydiumClmmPoolState)
                || filter.should_include(crate::grpc::EventType::AccountRaydiumClmmTickArrayState)
        });
        if should_parse {
            let event = filter_parsed_event(
                raydium_clmm::parse_account(account, metadata.clone()),
                event_type_filter,
            );
            if event.is_some() {
                return event;
            }
        }
        return None;
    }
    if account.owner == crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID {
        let should_parse = event_type_filter.is_none_or(|filter| {
            filter.should_include(crate::grpc::EventType::AccountRaydiumCpmmAmmConfig)
                || filter.should_include(crate::grpc::EventType::AccountRaydiumCpmmPoolState)
        });
        if should_parse {
            let event = filter_parsed_event(
                raydium_cpmm::parse_account(account, metadata.clone()),
                event_type_filter,
            );
            if event.is_some() {
                return event;
            }
        }
        return None;
    }
    if account.owner == crate::instr::program_ids::ORCA_WHIRLPOOL_PROGRAM_ID {
        let should_parse = event_type_filter.is_none_or(|filter| {
            filter.should_include(crate::grpc::EventType::AccountOrcaWhirlpool)
                || filter.should_include(crate::grpc::EventType::AccountOrcaPosition)
                || filter.should_include(crate::grpc::EventType::AccountOrcaTickArray)
                || filter.should_include(crate::grpc::EventType::AccountOrcaFeeTier)
                || filter.should_include(crate::grpc::EventType::AccountOrcaWhirlpoolsConfig)
        });
        if should_parse {
            let event = filter_parsed_event(
                orca_whirlpool::parse_account(account, metadata.clone()),
                event_type_filter,
            );
            if event.is_some() {
                return event;
            }
        }
        return None;
    }
    if account.owner == crate::grpc::program_ids::PUMPFUN_PROGRAM
        || account.owner == crate::instr::program_ids::PUMP_FEES_PROGRAM_ID
    {
        let should_parse = event_type_filter.is_none_or(|filter| {
            filter.should_include(crate::grpc::EventType::AccountPumpFunGlobal)
                || filter.should_include(crate::grpc::EventType::AccountPumpFunBondingCurve)
                || filter.should_include(crate::grpc::EventType::AccountPumpFunFeeConfig)
                || filter.should_include(crate::grpc::EventType::AccountPumpFunSharingConfig)
                || filter
                    .should_include(crate::grpc::EventType::AccountPumpFunGlobalVolumeAccumulator)
                || filter
                    .should_include(crate::grpc::EventType::AccountPumpFunUserVolumeAccumulator)
        });
        if should_parse {
            let event = filter_parsed_event(
                parse_pumpfun_account(account, metadata.clone()),
                event_type_filter,
            );
            if event.is_some() {
                return event;
            }
        }
        return None;
    }
    if nonce::is_nonce_account(&account.data) {
        // Check filter for NonceAccount specifically
        if let Some(filter) = event_type_filter {
            if !filter.should_include(crate::grpc::EventType::NonceAccount) {
                return None;
            }
        }
        return parse_nonce_account(account, metadata);
    }
    // Parse token account (includes both TokenAccount and TokenInfo)
    if let Some(filter) = event_type_filter {
        let includes_token = filter.should_include(crate::grpc::EventType::TokenAccount)
            || filter.should_include(crate::grpc::EventType::TokenInfo);
        if !includes_token {
            return None;
        }
    }
    filter_parsed_event(parse_token_account(account, metadata), event_type_filter)
}

fn parse_pumpswap_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    // 检查 discriminator 以确定账户类型
    if pumpswap::is_global_config_account(&account.data) {
        return pumpswap::parse_global_config(account, metadata);
    }
    if pumpswap::is_pool_account(&account.data) {
        return pumpswap::parse_pool(account, metadata);
    }
    None
}

fn parse_pumpfun_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    use crate::core::events::{
        PumpFeesConfigStatus, PumpFeesFeeTier, PumpFeesFees, PumpFeesShareholder,
        PumpFunBondingCurve, PumpFunBondingCurveAccountEvent, PumpFunFeeConfig,
        PumpFunFeeConfigAccountEvent, PumpFunGlobal, PumpFunGlobalAccountEvent,
        PumpFunGlobalVolumeAccumulator, PumpFunGlobalVolumeAccumulatorAccountEvent,
        PumpFunSharingConfig, PumpFunSharingConfigAccountEvent, PumpFunUserVolumeAccumulator,
        PumpFunUserVolumeAccumulatorAccountEvent,
    };

    const GLOBAL_DISCRIMINATOR: &[u8; 8] = &[167, 232, 232, 177, 200, 108, 114, 127];
    const BONDING_CURVE_DISCRIMINATOR: &[u8; 8] = &[23, 183, 248, 55, 96, 216, 172, 96];
    const FEE_CONFIG_DISCRIMINATOR: &[u8; 8] = &[143, 52, 146, 187, 219, 123, 76, 155];
    const GLOBAL_VOLUME_ACCUMULATOR_DISCRIMINATOR: &[u8; 8] =
        &[202, 42, 246, 43, 142, 190, 30, 255];
    const SHARING_CONFIG_DISCRIMINATOR: &[u8; 8] = &[216, 74, 9, 0, 56, 140, 93, 75];
    const USER_VOLUME_ACCUMULATOR_DISCRIMINATOR: &[u8; 8] = &[86, 255, 112, 14, 102, 53, 154, 250];
    const MAX_FEE_TIERS: usize = 64;
    const MAX_SHAREHOLDERS: usize = 64;

    fn read_i64(data: &[u8], offset: &mut usize) -> Option<i64> {
        let value = i64::from_le_bytes(data.get(*offset..*offset + 8)?.try_into().ok()?);
        *offset += 8;
        Some(value)
    }
    fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
        let value = u32::from_le_bytes(data.get(*offset..*offset + 4)?.try_into().ok()?);
        *offset += 4;
        Some(value)
    }
    fn read_u64(data: &[u8], offset: &mut usize) -> Option<u64> {
        let value = read_u64_le(data, *offset)?;
        *offset += 8;
        Some(value)
    }
    fn read_u128(data: &[u8], offset: &mut usize) -> Option<u128> {
        let value = u128::from_le_bytes(data.get(*offset..*offset + 16)?.try_into().ok()?);
        *offset += 16;
        Some(value)
    }
    fn read_pk(data: &[u8], offset: &mut usize) -> Option<solana_sdk::pubkey::Pubkey> {
        let value = read_pubkey(data, *offset)?;
        *offset += 32;
        Some(value)
    }
    fn read_fees(data: &[u8], offset: &mut usize) -> Option<PumpFeesFees> {
        Some(PumpFeesFees {
            lp_fee_bps: read_u64(data, offset)?,
            protocol_fee_bps: read_u64(data, offset)?,
            creator_fee_bps: read_u64(data, offset)?,
        })
    }
    fn read_fee_tiers(data: &[u8], offset: &mut usize) -> Option<Vec<PumpFeesFeeTier>> {
        let len = read_u32(data, offset)? as usize;
        if len > MAX_FEE_TIERS {
            return None;
        }
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(PumpFeesFeeTier {
                market_cap_lamports_threshold: read_u128(data, offset)?,
                fees: read_fees(data, offset)?,
            });
        }
        Some(out)
    }
    fn read_shareholders(data: &[u8], offset: &mut usize) -> Option<Vec<PumpFeesShareholder>> {
        let len = read_u32(data, offset)? as usize;
        if len > MAX_SHAREHOLDERS {
            return None;
        }
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            let address = read_pk(data, offset)?;
            let share_bps = u16::from_le_bytes(data.get(*offset..*offset + 2)?.try_into().ok()?);
            *offset += 2;
            out.push(PumpFeesShareholder { address, share_bps });
        }
        Some(out)
    }
    fn read_status(data: &[u8], offset: &mut usize) -> Option<PumpFeesConfigStatus> {
        let value = *data.get(*offset)?;
        *offset += 1;
        match value {
            0 => Some(PumpFeesConfigStatus::Paused),
            1 => Some(PumpFeesConfigStatus::Active),
            _ => None,
        }
    }

    if has_discriminator(&account.data, FEE_CONFIG_DISCRIMINATOR) {
        let data = &account.data[8..];
        let mut offset = 0usize;
        let fee_config = PumpFunFeeConfig {
            bump: *data.get(offset)?,
            admin: {
                offset += 1;
                read_pk(data, &mut offset)?
            },
            flat_fees: read_fees(data, &mut offset)?,
            fee_tiers: read_fee_tiers(data, &mut offset)?,
            stable_fee_tiers: read_fee_tiers(data, &mut offset)?,
        };
        return Some(DexEvent::PumpFunFeeConfigAccount(PumpFunFeeConfigAccountEvent {
            metadata,
            pubkey: account.pubkey,
            fee_config,
        }));
    }

    if has_discriminator(&account.data, SHARING_CONFIG_DISCRIMINATOR) {
        let data = &account.data[8..];
        let mut offset = 0usize;
        let bump = *data.get(offset)?;
        offset += 1;
        let version = *data.get(offset)?;
        offset += 1;
        let status = read_status(data, &mut offset)?;
        let mint = read_pk(data, &mut offset)?;
        let admin = read_pk(data, &mut offset)?;
        let admin_revoked = *data.get(offset)? != 0;
        offset += 1;
        let shareholders = read_shareholders(data, &mut offset)?;
        return Some(DexEvent::PumpFunSharingConfigAccount(PumpFunSharingConfigAccountEvent {
            metadata,
            pubkey: account.pubkey,
            sharing_config: PumpFunSharingConfig {
                bump,
                version,
                status,
                mint,
                admin,
                admin_revoked,
                shareholders,
            },
        }));
    }

    if has_discriminator(&account.data, GLOBAL_VOLUME_ACCUMULATOR_DISCRIMINATOR) {
        let data = &account.data[8..];
        let mut offset = 0usize;
        let start_time = read_i64(data, &mut offset)?;
        let end_time = read_i64(data, &mut offset)?;
        let seconds_in_a_day = read_i64(data, &mut offset)?;
        let mint = read_pk(data, &mut offset)?;
        let mut total_token_supply = [0u64; 30];
        for value in &mut total_token_supply {
            *value = read_u64(data, &mut offset)?;
        }
        let mut sol_volumes = [0u64; 30];
        for value in &mut sol_volumes {
            *value = read_u64(data, &mut offset)?;
        }
        return Some(DexEvent::PumpFunGlobalVolumeAccumulatorAccount(
            PumpFunGlobalVolumeAccumulatorAccountEvent {
                metadata,
                pubkey: account.pubkey,
                global_volume_accumulator: PumpFunGlobalVolumeAccumulator {
                    start_time,
                    end_time,
                    seconds_in_a_day,
                    mint,
                    total_token_supply,
                    sol_volumes,
                },
            },
        ));
    }

    if has_discriminator(&account.data, USER_VOLUME_ACCUMULATOR_DISCRIMINATOR) {
        let data = &account.data[8..];
        let mut offset = 0usize;
        let user = read_pk(data, &mut offset)?;
        let needs_claim = *data.get(offset)? != 0;
        offset += 1;
        let total_unclaimed_tokens = read_u64(data, &mut offset)?;
        let total_claimed_tokens = read_u64(data, &mut offset)?;
        let current_sol_volume = read_u64(data, &mut offset)?;
        let last_update_timestamp = read_i64(data, &mut offset)?;
        let has_total_claimed_tokens = *data.get(offset)? != 0;
        offset += 1;
        let cashback_earned = read_u64(data, &mut offset)?;
        let total_cashback_claimed = read_u64(data, &mut offset)?;
        let stable_cashback_earned = read_u64(data, &mut offset)?;
        let total_stable_cashback_claimed = read_u64(data, &mut offset)?;
        return Some(DexEvent::PumpFunUserVolumeAccumulatorAccount(
            PumpFunUserVolumeAccumulatorAccountEvent {
                metadata,
                pubkey: account.pubkey,
                user_volume_accumulator: PumpFunUserVolumeAccumulator {
                    user,
                    needs_claim,
                    total_unclaimed_tokens,
                    total_claimed_tokens,
                    current_sol_volume,
                    last_update_timestamp,
                    has_total_claimed_tokens,
                    cashback_earned,
                    total_cashback_claimed,
                    stable_cashback_earned,
                    total_stable_cashback_claimed,
                },
            },
        ));
    }
    if has_discriminator(&account.data, BONDING_CURVE_DISCRIMINATOR) {
        let data = &account.data[8..];
        let mut offset = 0usize;
        let virtual_token_reserves = read_u64_le(data, offset)?;
        offset += 8;
        let virtual_quote_reserves = read_u64_le(data, offset)?;
        offset += 8;
        let real_token_reserves = read_u64_le(data, offset)?;
        offset += 8;
        let real_quote_reserves = read_u64_le(data, offset)?;
        offset += 8;
        let token_total_supply = read_u64_le(data, offset)?;
        offset += 8;
        let complete = read_u8(data, offset)? != 0;
        offset += 1;
        let creator = read_pubkey(data, offset)?;
        offset += 32;
        let is_mayhem_mode = read_u8(data, offset)? != 0;
        offset += 1;
        let is_cashback_coin = read_u8(data, offset)? != 0;
        offset += 1;
        let quote_mint = read_pubkey(data, offset)?;

        return Some(DexEvent::PumpFunBondingCurveAccount(PumpFunBondingCurveAccountEvent {
            metadata,
            pubkey: account.pubkey,
            bonding_curve: PumpFunBondingCurve {
                virtual_token_reserves,
                virtual_quote_reserves,
                real_token_reserves,
                real_quote_reserves,
                token_total_supply,
                complete,
                creator,
                is_mayhem_mode,
                is_cashback_coin,
                quote_mint,
            },
        }));
    }
    if !has_discriminator(&account.data, GLOBAL_DISCRIMINATOR) {
        return None;
    }

    let data = &account.data[8..];
    let mut offset = 0usize;
    let initialized = read_u8(data, offset)? != 0;
    offset += 1;
    let authority = read_pubkey(data, offset)?;
    offset += 32;
    let fee_recipient = read_pubkey(data, offset)?;
    offset += 32;
    let initial_virtual_token_reserves = read_u64_le(data, offset)?;
    offset += 8;
    let initial_virtual_sol_reserves = read_u64_le(data, offset)?;
    offset += 8;
    let initial_real_token_reserves = read_u64_le(data, offset)?;
    offset += 8;
    let token_total_supply = read_u64_le(data, offset)?;
    offset += 8;
    let fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;
    let withdraw_authority = read_pubkey(data, offset)?;
    offset += 32;
    let enable_migrate = read_u8(data, offset)? != 0;
    offset += 1;
    let pool_migration_fee = read_u64_le(data, offset)?;
    offset += 8;
    let creator_fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;
    let mut fee_recipients = [solana_sdk::pubkey::Pubkey::default(); 7];
    for fee_recipient in &mut fee_recipients {
        *fee_recipient = read_pubkey(data, offset)?;
        offset += 32;
    }
    let set_creator_authority = read_pubkey(data, offset)?;
    offset += 32;
    let admin_set_creator_authority = read_pubkey(data, offset)?;
    offset += 32;
    let create_v2_enabled = read_u8(data, offset)? != 0;
    offset += 1;
    let whitelist_pda = read_pubkey(data, offset)?;
    offset += 32;
    let reserved_fee_recipient = read_pubkey(data, offset)?;
    offset += 32;
    let mayhem_mode_enabled = read_u8(data, offset)? != 0;
    offset += 1;
    let mut reserved_fee_recipients = [solana_sdk::pubkey::Pubkey::default(); 7];
    for reserved_fee_recipient in &mut reserved_fee_recipients {
        *reserved_fee_recipient = read_pubkey(data, offset)?;
        offset += 32;
    }
    let is_cashback_enabled = read_u8(data, offset)? != 0;
    offset += 1;
    let buyback_fee_recipients = {
        let mut keys = [solana_sdk::pubkey::Pubkey::default(); 8];
        for key in &mut keys {
            *key = read_pubkey(data, offset)?;
            offset += 32;
        }
        keys
    };
    let buyback_basis_points = read_u64_le(data, offset)?;
    offset += 8;
    let initial_virtual_quote_reserves = read_u64_le(data, offset)?;
    offset += 8;
    let whitelisted_quote_mints = {
        let mut keys = [solana_sdk::pubkey::Pubkey::default(); 1];
        keys[0] = read_pubkey(data, offset)?;
        keys
    };

    let global = PumpFunGlobal {
        initialized,
        authority,
        fee_recipient,
        initial_virtual_token_reserves,
        initial_virtual_sol_reserves,
        initial_real_token_reserves,
        token_total_supply,
        fee_basis_points,
        withdraw_authority,
        enable_migrate,
        pool_migration_fee,
        creator_fee_basis_points,
        fee_recipients,
        set_creator_authority,
        admin_set_creator_authority,
        create_v2_enabled,
        whitelist_pda,
        reserved_fee_recipient,
        mayhem_mode_enabled,
        reserved_fee_recipients,
        is_cashback_enabled,
        buyback_fee_recipients,
        buyback_basis_points,
        initial_virtual_quote_reserves,
        whitelisted_quote_mints,
    };

    Some(DexEvent::PumpFunGlobalAccount(PumpFunGlobalAccountEvent {
        metadata,
        pubkey: account.pubkey,
        global,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::{EventType, EventTypeFilter};
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signature;

    fn metadata() -> EventMetadata {
        EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        }
    }

    fn push_pk(out: &mut Vec<u8>, seed: u8) -> Pubkey {
        let key = Pubkey::new_from_array([seed; 32]);
        out.extend_from_slice(key.as_ref());
        key
    }

    #[test]
    fn parse_pumpfun_bonding_curve_reads_quote_fields() {
        let mut data = vec![23, 183, 248, 55, 96, 216, 172, 96];
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&4_292_000_000u64.to_le_bytes());
        data.extend_from_slice(&200u64.to_le_bytes());
        data.extend_from_slice(&3_000_000_000u64.to_le_bytes());
        data.extend_from_slice(&1_000u64.to_le_bytes());
        data.push(1);
        let creator = push_pk(&mut data, 7);
        data.push(1);
        data.push(0);
        let quote_mint = push_pk(&mut data, 8);
        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            executable: false,
            lamports: 0,
            owner: crate::grpc::program_ids::PUMPFUN_PROGRAM,
            rent_epoch: 0,
            data,
        };
        let filter = EventTypeFilter::include_only(vec![EventType::AccountPumpFunBondingCurve]);

        let ev = parse_account_unified(&account, metadata(), Some(&filter)).expect("event");

        match ev {
            DexEvent::PumpFunBondingCurveAccount(e) => {
                assert_eq!(e.bonding_curve.virtual_quote_reserves, 4_292_000_000);
                assert_eq!(e.bonding_curve.real_quote_reserves, 3_000_000_000);
                assert_eq!(e.bonding_curve.creator, creator);
                assert_eq!(e.bonding_curve.quote_mint, quote_mint);
                assert!(e.bonding_curve.complete);
                assert!(e.bonding_curve.is_mayhem_mode);
                assert!(!e.bonding_curve.is_cashback_coin);
            }
            other => panic!("expected bonding curve account, got {other:?}"),
        }
    }

    #[test]
    fn parse_unified_routes_cpmm_and_orca_account_filters() {
        let mut cpmm = Vec::with_capacity(8 + raydium_cpmm::AMM_CONFIG_SIZE);
        cpmm.extend_from_slice(raydium_cpmm::discriminators::AMM_CONFIG);
        cpmm.push(1);
        cpmm.push(0);
        cpmm.extend_from_slice(&42u16.to_le_bytes());
        for value in [10u64, 20, 30, 40] {
            cpmm.extend_from_slice(&value.to_le_bytes());
        }
        push_pk(&mut cpmm, 1);
        push_pk(&mut cpmm, 2);
        cpmm.extend_from_slice(&50u64.to_le_bytes());
        for value in 0u64..15 {
            cpmm.extend_from_slice(&value.to_le_bytes());
        }
        let cpmm_account = AccountData {
            pubkey: Pubkey::new_unique(),
            executable: false,
            lamports: 0,
            owner: crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID,
            rent_epoch: 0,
            data: cpmm,
        };
        let cpmm_filter =
            EventTypeFilter::include_only(vec![EventType::AccountRaydiumCpmmAmmConfig]);
        assert!(matches!(
            parse_account_unified(&cpmm_account, metadata(), Some(&cpmm_filter)),
            Some(DexEvent::RaydiumCpmmAmmConfigAccount(_))
        ));
        let cpmm_mismatch_filter =
            EventTypeFilter::include_only(vec![EventType::AccountRaydiumCpmmPoolState]);
        assert!(
            parse_account_unified(&cpmm_account, metadata(), Some(&cpmm_mismatch_filter)).is_none()
        );

        let mut fee_tier = Vec::with_capacity(8 + orca_whirlpool::FEE_TIER_SIZE);
        fee_tier.extend_from_slice(orca_whirlpool::discriminators::FEE_TIER);
        push_pk(&mut fee_tier, 3);
        fee_tier.extend_from_slice(&64u16.to_le_bytes());
        fee_tier.extend_from_slice(&300u16.to_le_bytes());
        let orca_account = AccountData {
            pubkey: Pubkey::new_unique(),
            executable: false,
            lamports: 0,
            owner: crate::instr::program_ids::ORCA_WHIRLPOOL_PROGRAM_ID,
            rent_epoch: 0,
            data: fee_tier,
        };
        let orca_filter = EventTypeFilter::include_only(vec![EventType::AccountOrcaFeeTier]);
        assert!(matches!(
            parse_account_unified(&orca_account, metadata(), Some(&orca_filter)),
            Some(DexEvent::OrcaFeeTierAccount(_))
        ));
        let orca_exclude_filter =
            EventTypeFilter::exclude_types(vec![EventType::AccountOrcaFeeTier]);
        assert!(
            parse_account_unified(&orca_account, metadata(), Some(&orca_exclude_filter)).is_none()
        );
    }

    #[test]
    fn parse_unified_filters_token_info_and_token_account_separately() {
        let mut data = vec![0u8; 82];
        data[36..44].copy_from_slice(&1_000_000u64.to_le_bytes());
        data[44] = 6;
        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            executable: false,
            lamports: 0,
            owner: Pubkey::new_from_array(spl_token::ID.to_bytes()),
            rent_epoch: 0,
            data,
        };

        let token_info_filter = EventTypeFilter::include_only(vec![EventType::TokenInfo]);
        assert!(matches!(
            parse_account_unified(&account, metadata(), Some(&token_info_filter)),
            Some(DexEvent::TokenInfo(_))
        ));

        let token_account_filter = EventTypeFilter::include_only(vec![EventType::TokenAccount]);
        assert!(parse_account_unified(&account, metadata(), Some(&token_account_filter)).is_none());
    }

    #[test]
    fn parse_unified_known_dex_owner_does_not_fall_through_to_token_parser() {
        let mut data = vec![0u8; 82];
        data[36..44].copy_from_slice(&1_000_000u64.to_le_bytes());
        data[44] = 6;
        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            executable: false,
            lamports: 0,
            owner: crate::grpc::program_ids::PUMPFUN_PROGRAM,
            rent_epoch: 0,
            data,
        };

        assert!(parse_account_unified(&account, metadata(), None).is_none());
    }
}
