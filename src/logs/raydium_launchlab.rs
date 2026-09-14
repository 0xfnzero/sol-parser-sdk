//! LaunchLab 日志解析器
//!
//! 底层按 `idls/raydium_launchpad.json` 的真实 event discriminator 和 Borsh
//! 布局解析，对外事件名统一为 `RaydiumLaunchlab*`。

use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// LaunchLab event discriminators from `idls/raydium_launchpad.json`.
pub mod discriminators {
    pub const CLAIM_VESTED: [u8; 8] = [21, 194, 114, 87, 120, 211, 226, 32];
    pub const CREATE_VESTING: [u8; 8] = [150, 152, 11, 179, 52, 210, 191, 125];
    pub const POOL_CREATE: [u8; 8] = [151, 215, 226, 9, 118, 161, 115, 174];
    pub const TRADE: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
}

/// LaunchLab 程序 ID
pub const PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";

/// 检查日志是否来自 LaunchLab 程序
pub fn is_raydium_launchlab_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID))
        || log.contains(&format!("Program {} success", PROGRAM_ID))
}

/// 主要的 LaunchLab 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    };

    match discriminator {
        discriminators::TRADE => parse_trade_from_data(data, metadata),
        discriminators::POOL_CREATE => parse_pool_create_from_data(data, metadata),
        _ => None,
    }
}

/// Parse LaunchLab TradeEvent from pre-decoded event data.
#[inline]
pub fn parse_trade_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const TRADE_EVENT_LEN: usize = 32 + 13 * 8 + 3;
    if data.len() != TRADE_EVENT_LEN {
        return None;
    }

    let pool_state = read_pubkey(data, 0)?;
    let total_base_sell = read_u64_le(data, 32)?;
    let virtual_base = read_u64_le(data, 40)?;
    let virtual_quote = read_u64_le(data, 48)?;
    let real_base_before = read_u64_le(data, 56)?;
    let real_quote_before = read_u64_le(data, 64)?;
    let real_base_after = read_u64_le(data, 72)?;
    let real_quote_after = read_u64_le(data, 80)?;
    let amount_in = read_u64_le(data, 88)?;
    let amount_out = read_u64_le(data, 96)?;
    let protocol_fee = read_u64_le(data, 104)?;
    let platform_fee = read_u64_le(data, 112)?;
    let creator_fee = read_u64_le(data, 120)?;
    let share_fee = read_u64_le(data, 128)?;
    let trade_direction = *data.get(136)?;
    if trade_direction > 1 {
        return None;
    }
    let pool_status = *data.get(137)?;
    if pool_status > 2 {
        return None;
    }
    let exact_in_raw = *data.get(138)?;
    if exact_in_raw > 1 {
        return None;
    }
    let exact_in = exact_in_raw == 1;
    let is_buy = trade_direction == 0;
    let pool_status = match pool_status {
        0 => RaydiumLaunchlabPoolStatus::Fund,
        1 => RaydiumLaunchlabPoolStatus::Migrate,
        2 => RaydiumLaunchlabPoolStatus::Trade,
        _ => return None,
    };

    Some(DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
        metadata,
        pool_state,
        user: Pubkey::default(),
        amount_in,
        amount_out,
        is_buy,
        total_base_sell,
        virtual_base,
        virtual_quote,
        real_base_before,
        real_quote_before,
        real_base_after,
        real_quote_after,
        protocol_fee,
        platform_fee,
        creator_fee,
        share_fee,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        pool_status,
        exact_in,
        global_config: Pubkey::default(),
        platform_config: Pubkey::default(),
        user_base_token: Pubkey::default(),
        user_quote_token: Pubkey::default(),
        base_vault: Pubkey::default(),
        quote_vault: Pubkey::default(),
        base_mint: Pubkey::default(),
        quote_mint: Pubkey::default(),
        base_token_program: Pubkey::default(),
        quote_token_program: Pubkey::default(),
        system_program: Pubkey::default(),
        platform_associated_account: Pubkey::default(),
        creator_associated_account: Pubkey::default(),
    }))
}

/// Parse LaunchLab PoolCreateEvent from pre-decoded event data.
#[inline]
pub fn parse_pool_create_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0usize;
    let pool_state = read_pubkey(data, offset)?;
    offset += 32;
    let creator = read_pubkey(data, offset)?;
    offset += 32;
    let _config = read_pubkey(data, offset)?;
    offset += 32;
    let base_mint_param = parse_mint_params(data, &mut offset)?;

    Some(DexEvent::RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent {
        metadata,
        base_mint_param,
        pool_state,
        payer: Pubkey::default(),
        creator,
        global_config: Pubkey::default(),
        platform_config: Pubkey::default(),
        base_mint: Pubkey::default(),
        quote_mint: Pubkey::default(),
        base_vault: Pubkey::default(),
        quote_vault: Pubkey::default(),
        base_token_program: Pubkey::default(),
        quote_token_program: Pubkey::default(),
    }))
}

fn parse_mint_params(data: &[u8], offset: &mut usize) -> Option<BaseMintParam> {
    let decimals = *data.get(*offset)?;
    *offset += 1;
    let name = read_borsh_string(data, offset)?;
    let symbol = read_borsh_string(data, offset)?;
    let uri = read_borsh_string(data, offset)?;
    Some(BaseMintParam { symbol, name, uri, decimals })
}

fn read_borsh_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    let end = (*offset).checked_add(len)?;
    let bytes = data.get(*offset..end)?;
    *offset = end;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_trade_layout_preserves_reserves_and_all_fee_legs() {
        let mut data = Vec::with_capacity(139);
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        for value in [
            793_100_000_000_000_u64,
            1_073_025_605_597_286,
            21_191_598_554,
            284_268_264_059_232,
            7_637_455_325,
            284_320_641_177_089,
            7_639_369_834,
            1_938_744,
            52_377_117_857,
            4_847,
            19_388,
            0,
            0,
        ] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&[0, 0, 1]);

        let DexEvent::RaydiumLaunchlabTrade(event) =
            parse_trade_from_data(&data, EventMetadata::default()).expect("trade")
        else {
            panic!("LaunchLab trade")
        };

        assert_eq!(event.total_base_sell, 793_100_000_000_000);
        assert_eq!(event.amount_in, 1_938_744);
        assert_eq!(event.amount_out, 52_377_117_857);
        assert_eq!(event.protocol_fee, 4_847);
        assert_eq!(event.platform_fee, 19_388);
        assert_eq!(event.creator_fee, 0);
        assert_eq!(event.share_fee, 0);
        assert_eq!(event.pool_status, RaydiumLaunchlabPoolStatus::Fund);
        assert!(event.is_buy);
        assert!(event.exact_in);
    }
}
