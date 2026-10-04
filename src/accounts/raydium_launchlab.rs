use crate::core::events::{
    BondingCurveParam, EventMetadata, PlatformCurveParam, RaydiumLaunchlabPlatformConfig,
    RaydiumLaunchlabPlatformConfigAccountEvent,
};
use crate::DexEvent;
use solana_sdk::pubkey::Pubkey;

use super::token::AccountData;

const DISCRIMINATOR_LEN: usize = 8;
const PUBKEY_LEN: usize = 32;
const PLATFORM_CONFIG_FIXED_LEN_WITHOUT_DISCRIMINATOR: usize = 8
    + PUBKEY_LEN
    + PUBKEY_LEN
    + 8
    + 8
    + 8
    + 8
    + 64
    + 256
    + 256
    + PUBKEY_LEN
    + 8
    + PUBKEY_LEN
    + PUBKEY_LEN
    + 8
    + PUBKEY_LEN
    + 108
    + 4;
const MAX_CURVE_PARAMS: u32 = 1024;

pub fn parse_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    parse_platform_config(account, metadata)
}

pub fn parse_platform_config(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = if account.data.len()
        >= DISCRIMINATOR_LEN + PLATFORM_CONFIG_FIXED_LEN_WITHOUT_DISCRIMINATOR
    {
        DISCRIMINATOR_LEN
    } else {
        0
    };

    let platform_config = read_platform_config(&account.data, &mut offset)?;
    Some(DexEvent::RaydiumLaunchlabPlatformConfigAccount(Box::new(
        RaydiumLaunchlabPlatformConfigAccountEvent {
            metadata,
            pubkey: account.pubkey,
            platform_config,
        },
    )))
}

fn read_platform_config(data: &[u8], offset: &mut usize) -> Option<RaydiumLaunchlabPlatformConfig> {
    let epoch = read_u64(data, offset)?;
    let platform_fee_wallet = read_pubkey(data, offset)?;
    let platform_nft_wallet = read_pubkey(data, offset)?;
    let platform_scale = read_u64(data, offset)?;
    let creator_scale = read_u64(data, offset)?;
    let burn_scale = read_u64(data, offset)?;
    let fee_rate = read_u64(data, offset)?;
    let name = take::<64>(data, offset)?;
    let web = take::<256>(data, offset)?;
    let img = take::<256>(data, offset)?;
    let cpswap_config = read_pubkey(data, offset)?;
    let creator_fee_rate = read_u64(data, offset)?;
    let transfer_fee_extension_auth = read_pubkey(data, offset)?;
    let platform_vesting_wallet = read_pubkey(data, offset)?;
    let platform_vesting_scale = read_u64(data, offset)?;
    let platform_cp_creator = read_pubkey(data, offset)?;
    let padding = take::<108>(data, offset)?;
    let curve_params_len = read_u32(data, offset)?;
    if curve_params_len > MAX_CURVE_PARAMS {
        return None;
    }

    let mut curve_params = Vec::with_capacity(curve_params_len as usize);
    for _ in 0..curve_params_len {
        curve_params.push(read_curve_param(data, offset)?);
    }

    Some(RaydiumLaunchlabPlatformConfig {
        epoch,
        platform_fee_wallet,
        platform_nft_wallet,
        platform_scale,
        creator_scale,
        burn_scale,
        fee_rate,
        name,
        web,
        img,
        cpswap_config,
        creator_fee_rate,
        transfer_fee_extension_auth,
        platform_vesting_wallet,
        platform_vesting_scale,
        platform_cp_creator,
        padding,
        curve_params,
    })
}

fn read_curve_param(data: &[u8], offset: &mut usize) -> Option<PlatformCurveParam> {
    let epoch = read_u64(data, offset)?;
    let index = read_u8(data, offset)?;
    let global_config = read_pubkey(data, offset)?;
    let bonding_curve_param = BondingCurveParam {
        migrate_type: read_u8(data, offset)?,
        migrate_cpmm_fee_on: read_u8(data, offset)?,
        supply: read_u64(data, offset)?,
        total_base_sell: read_u64(data, offset)?,
        total_quote_fund_raising: read_u64(data, offset)?,
        total_locked_amount: read_u64(data, offset)?,
        cliff_period: read_u64(data, offset)?,
        unlock_period: read_u64(data, offset)?,
    };
    let mut padding = [0u64; 50];
    for item in &mut padding {
        *item = read_u64(data, offset)?;
    }
    Some(PlatformCurveParam { epoch, index, global_config, bonding_curve_param, padding })
}

fn read_u8(data: &[u8], offset: &mut usize) -> Option<u8> {
    let bytes = take::<1>(data, offset)?;
    Some(bytes[0])
}

fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
    Some(u32::from_le_bytes(take::<4>(data, offset)?))
}

fn read_u64(data: &[u8], offset: &mut usize) -> Option<u64> {
    Some(u64::from_le_bytes(take::<8>(data, offset)?))
}

fn read_pubkey(data: &[u8], offset: &mut usize) -> Option<Pubkey> {
    Some(Pubkey::new_from_array(take::<32>(data, offset)?))
}

fn take<const N: usize>(data: &[u8], offset: &mut usize) -> Option<[u8; N]> {
    if data.len().saturating_sub(*offset) < N {
        return None;
    }
    let out = data[*offset..*offset + N].try_into().ok()?;
    *offset += N;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::EventMetadata;

    fn write_u64(buf: &mut Vec<u8>, n: u64) {
        buf.extend_from_slice(&n.to_le_bytes());
    }

    fn write_pubkey(buf: &mut Vec<u8>, byte: u8) {
        buf.extend_from_slice(&[byte; 32]);
    }

    fn write_fixed<const N: usize>(buf: &mut Vec<u8>, s: &str) {
        let mut out = [0u8; N];
        out[..s.len()].copy_from_slice(s.as_bytes());
        buf.extend_from_slice(&out);
    }

    #[test]
    fn parses_platform_config_account() {
        let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        write_u64(&mut data, 7);
        write_pubkey(&mut data, 9);
        write_pubkey(&mut data, 10);
        write_u64(&mut data, 1);
        write_u64(&mut data, 2);
        write_u64(&mut data, 3);
        write_u64(&mut data, 4);
        write_fixed::<64>(&mut data, "ScreenFI");
        write_fixed::<256>(&mut data, "https://screenfi.fun");
        write_fixed::<256>(&mut data, "https://screenfi.fun/logo.png");
        write_pubkey(&mut data, 11);
        write_u64(&mut data, 5);
        write_pubkey(&mut data, 12);
        write_pubkey(&mut data, 13);
        write_u64(&mut data, 6);
        write_pubkey(&mut data, 14);
        data.extend_from_slice(&[0u8; 108]);
        data.extend_from_slice(&0u32.to_le_bytes());

        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            owner: crate::instr::program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID,
            data,
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        };

        let event = parse_platform_config(&account, EventMetadata::default()).unwrap();
        let DexEvent::RaydiumLaunchlabPlatformConfigAccount(event) = event else {
            panic!("expected LaunchLab platform config account event");
        };
        assert_eq!(event.platform_config.epoch, 7);
        assert_eq!(&event.platform_config.name[..8], b"ScreenFI");
        assert_eq!(event.platform_config.curve_params.len(), 0);
    }

    /// One account update yields one event, and the liquidity snapshot is
    /// parsed first: a platform config is decoded only for a filter that asks
    /// for the decoded event without the snapshots.
    #[test]
    fn platform_config_is_decoded_only_without_liquidity_snapshots() {
        use crate::grpc::{EventType, EventTypeFilter};

        let mut data = vec![160, 78, 128, 0, 248, 83, 230, 160];
        data.resize(DISCRIMINATOR_LEN + PLATFORM_CONFIG_FIXED_LEN_WITHOUT_DISCRIMINATOR, 0);
        let account = AccountData {
            pubkey: Pubkey::new_unique(),
            owner: crate::instr::program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID,
            data,
            executable: false,
            lamports: 1,
            rent_epoch: 0,
        };
        let parse = |filter: Option<&EventTypeFilter>| {
            crate::accounts::parse_account_unified(&account, EventMetadata::default(), filter)
        };

        let decoded =
            EventTypeFilter::include_only(vec![EventType::AccountRaydiumLaunchlabPlatformConfig]);
        assert!(matches!(
            parse(Some(&decoded)),
            Some(DexEvent::RaydiumLaunchlabPlatformConfigAccount(_))
        ));

        let both = EventTypeFilter::include_only(vec![
            EventType::AccountLiquiditySnapshot,
            EventType::AccountRaydiumLaunchlabPlatformConfig,
        ]);
        assert!(matches!(parse(Some(&both)), Some(DexEvent::LiquidityAccountSnapshot(_))));
        assert!(matches!(parse(None), Some(DexEvent::LiquidityAccountSnapshot(_))));
    }
}
