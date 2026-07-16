//! Discriminator Lookup Table (LUT) - Compile-time constant array
//!
//! Zero-latency optimization: Use const array with binary search for O(log n) discriminator -> event type mapping
//! Expected latency reduction: 1-10ns (binary search on sorted array, better cache locality than match)

use crate::core::events::{DexEvent, EventMetadata};
use crate::grpc::types::EventType;

/// Discriminator type alias for clarity
pub type Discriminator = u64;

/// Parser function type - takes decoded data and metadata, returns parsed event
pub type ParserFn = fn(&[u8], EventMetadata) -> Option<DexEvent>;

/// Event metadata for discriminator lookup
#[derive(Debug, Clone, Copy)]
pub struct DiscriminatorInfo {
    pub discriminator: u64,
    pub parser: ParserFn,
    pub protocol: Protocol,
    pub name: &'static str, // Human-readable name for debugging
}

/// Protocol enum for quick protocol identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    PumpFun,
    PumpFees,
    PumpSwap,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumAmm,
    OrcaWhirlpool,
    MeteoraAmm,
    MeteoraDamm,
    MeteoraDlmm,
}

// ============================================================================
// Parser function wrappers - delegate to protocol-specific parsers
// ============================================================================

// PumpFun parsers
#[inline(always)]
fn parse_pumpfun_create(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_create_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfun_trade(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_trade_from_data(data, metadata, false)
}

#[inline(always)]
fn parse_pumpfun_migrate(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_migrate_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfun_migrate_bonding_curve_creator(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::pump::parse_migrate_bonding_curve_creator_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_initialize_fee_config(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_initialize_fee_config_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_reset(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_reset_fee_sharing_config_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_revoke(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_revoke_fee_sharing_authority_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_transfer(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_transfer_fee_sharing_authority_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_update_admin(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_update_admin_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_update_fee_config(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_update_fee_config_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_update_fee_shares(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_update_fee_shares_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfees_upsert_fee_tiers(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_upsert_fee_tiers_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfun_create_fee_sharing_config(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(data, metadata)
}

// PumpSwap parsers
#[inline(always)]
fn parse_pumpswap_buy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_buy_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_sell(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_sell_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_add_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_remove_liquidity_from_data(data, metadata)
}

// Raydium CLMM parsers
#[inline(always)]
fn parse_raydium_clmm_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_swap_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_increase_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_decrease_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_liquidity_change(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_liquidity_change_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_config_change(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_config_change_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_create_personal_position(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_create_personal_position_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_liquidity_calculate(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_liquidity_calculate_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_open_limit_order(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_open_limit_order_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_increase_limit_order(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_increase_limit_order_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_decrease_limit_order(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_decrease_limit_order_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_settle_limit_order(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_settle_limit_order_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_update_reward_infos(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_update_reward_infos_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_collect_personal_fee(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_collect_personal_fee_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_collect_protocol_fee(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_collect_protocol_fee_from_data(data, metadata)
}

// Raydium CPMM parsers
#[inline(always)]
fn parse_raydium_cpmm_swap_base_in(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_swap_base_out(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
}

// Raydium AMM V4 parsers
#[inline(always)]
fn parse_raydium_amm_swap_base_in(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_swap_base_out(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_initialize2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
}

// Orca Whirlpool parsers
#[inline(always)]
fn parse_orca_traded(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_liquidity_increased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_liquidity_decreased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_pool_initialized(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
}

// Meteora AMM parsers
#[inline(always)]
fn parse_meteora_amm_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_swap_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_bootstrap_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_pool_created(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
}

// ============================================================================
// Const lookup table - Sorted by discriminator for binary search
// ============================================================================

macro_rules! disc_entry {
    ($disc:expr, $name:expr, $parser:expr, $protocol:expr) => {
        DiscriminatorInfo {
            discriminator: $disc,
            parser: $parser,
            protocol: $protocol,
            name: $name,
        }
    };
}

/// Compile-time constant array: discriminator -> parser info
/// MUST be kept sorted by discriminator for binary search!
///
/// Expected latency: 3–8 ns (binary search on 33 discriminators ⇒ at most 6 comparisons)
pub const DISCRIMINATOR_LUT: &[DiscriminatorInfo] = &[
    // 按 discriminator 数值升序（binary_search 要求）
    disc_entry!(
        0x0100000000000000,
        "Raydium AMM Initialize2",
        parse_raydium_amm_initialize2,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0300000000000000,
        "Raydium AMM Deposit",
        parse_raydium_amm_deposit,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x03F36CD5DC68A79B_u64,
        "PumpFun Migrate Bonding Curve Creator",
        parse_pumpfun_migrate_bonding_curve_creator,
        Protocol::PumpFun
    ),
    disc_entry!(
        0x0400000000000000,
        "Raydium AMM Withdraw",
        parse_raydium_amm_withdraw,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0900000000000000,
        "Raydium AMM Swap Base In",
        parse_raydium_amm_swap_base_in,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0xABB5CA7047241A6,
        "Orca Whirlpool Liquidity Decreased",
        parse_orca_liquidity_decreased,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0x0B00000000000000,
        "Raydium AMM Swap Base Out",
        parse_raydium_amm_swap_base_out,
        Protocol::RaydiumAmm
    ),
    // Raydium CPMM events
    disc_entry!(
        0x22A16D949C4612B7,
        "Raydium CPMM Withdraw",
        parse_raydium_cpmm_withdraw,
        Protocol::RaydiumCpmm
    ),
    // PumpSwap events
    disc_entry!(0x2ADC03A50A372F3E, "PumpSwap Sell", parse_pumpswap_sell, Protocol::PumpSwap),
    disc_entry!(
        0x385532443A56DE3A,
        "Raydium CLMM Decrease Liquidity",
        parse_raydium_clmm_decrease_liquidity,
        Protocol::RaydiumClmm
    ),
    // Meteora AMM events
    disc_entry!(
        0x3A981F67E861F474,
        "Meteora AMM Remove Liquidity",
        parse_meteora_amm_remove_liquidity,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0x3DD5292D4F1157CE,
        "Raydium CLMM Collect Protocol Fee",
        parse_raydium_clmm_collect_protocol_fee,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x3E99BE0E3C651772_u64,
        "Pump Fees Revoke Fee Sharing Authority",
        parse_pumpfees_revoke,
        Protocol::PumpFees
    ),
    disc_entry!(
        0x3F3563702F4B5E19,
        "Raydium CLMM Create Pool",
        parse_raydium_clmm_create_pool,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x529DDC6858292CCA,
        "Meteora AMM Pool Created",
        parse_meteora_amm_pool_created,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0x541E2220D4694F31,
        "Raydium CLMM Increase Liquidity",
        parse_raydium_clmm_increase_liquidity,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x58FB74B8C8AA6985_u64,
        "Pump Fees Create Fee Sharing Config",
        parse_pumpfun_create_fee_sharing_config,
        Protocol::PumpFees
    ),
    disc_entry!(
        0x6953A151C069AEA6,
        "Raydium CLMM Collect Personal Fee",
        parse_raydium_clmm_collect_personal_fee,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x6B99589ECEAFF07E,
        "Raydium CLMM Liquidity Change",
        parse_raydium_clmm_liquidity_change,
        Protocol::RaydiumClmm
    ),
    // PumpSwap and PumpFun events
    disc_entry!(
        0x74A776A0D20C31B1,
        "PumpSwap Create Pool",
        parse_pumpswap_create_pool,
        Protocol::PumpSwap
    ),
    disc_entry!(0x7663EBDE4DA91B1B, "PumpFun Create", parse_pumpfun_create, Protocol::PumpFun),
    disc_entry!(0x7777F52C1F52F467, "PumpSwap Buy", parse_pumpswap_buy, Protocol::PumpSwap),
    disc_entry!(
        0x7EE2380AE6F48A59_u64,
        "Pump Fees Initialize Fee Config",
        parse_pumpfees_initialize_fee_config,
        Protocol::PumpFees
    ),
    disc_entry!(
        0x906B8E1F533DF878,
        "PumpSwap Add Liquidity",
        parse_pumpswap_add_liquidity,
        Protocol::PumpSwap
    ),
    disc_entry!(0x94EA945DB95DE9BD, "PumpFun Migrate", parse_pumpfun_migrate, Protocol::PumpFun),
    disc_entry!(
        0x96A02B93AF49CAE1,
        "Orca Whirlpool Traded",
        parse_orca_traded,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0x975F706A7707BDF7,
        "Raydium CLMM Config Change",
        parse_raydium_clmm_config_change,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xA19BFE6690071E1E,
        "Orca Whirlpool Liquidity Increased",
        parse_orca_liquidity_increased,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0xA2B45439E69470ED,
        "Raydium CLMM Liquidity Calculate",
        parse_raydium_clmm_liquidity_calculate,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xA3D4EDDBDD283046,
        "Raydium CLMM Decrease Limit Order",
        parse_raydium_clmm_decrease_limit_order,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xADB44AA35662D937,
        "Raydium CPMM Swap Base Out",
        parse_raydium_cpmm_swap_base_out,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xB6F2E15289C623F2,
        "Raydium CPMM Deposit",
        parse_raydium_cpmm_deposit,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xBA3D34E35A7D5E1F,
        "Meteora AMM Add Liquidity",
        parse_meteora_amm_add_liquidity,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0xC0472CA01A850916,
        "PumpSwap Remove Liquidity",
        parse_pumpswap_remove_liquidity,
        Protocol::PumpSwap
    ),
    disc_entry!(
        0xC20A7C7DA44D7758,
        "Raydium CLMM Settle Limit Order",
        parse_raydium_clmm_settle_limit_order,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xC40AD0CDBE6CE351,
        "Meteora AMM Swap",
        parse_meteora_amm_swap,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0xC81357C7CC0D780B,
        "Raydium CLMM Increase Limit Order",
        parse_raydium_clmm_increase_limit_order,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xCBE1E45BB8C4BA15_u64,
        "Pump Fees Update Fee Shares",
        parse_pumpfees_update_fee_shares,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xCC21BA7ABBA959AB_u64,
        "Pump Fees Upsert Fee Tiers",
        parse_pumpfees_upsert_fee_tiers,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xCE9ADFC4F9571E64,
        "Raydium CLMM Create Personal Position",
        parse_raydium_clmm_create_personal_position,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xD0BCF43E2341175A_u64,
        "Pump Fees Update Fee Config",
        parse_pumpfees_update_fee_config,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xD89EA9395547186A,
        "Raydium CLMM Open Limit Order",
        parse_raydium_clmm_open_limit_order,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xDE331EC4DA5ABE8F,
        "Raydium CPMM Swap Base In",
        parse_raydium_cpmm_swap_base_in,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xE2710826E8CDC640,
        "Raydium CLMM Swap",
        parse_raydium_clmm_swap,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xE5FEC60C57AD7664,
        "Orca Whirlpool Initialize",
        parse_orca_pool_initialized,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0xEA423FF657AB98E1_u64,
        "Pump Fees Update Admin",
        parse_pumpfees_update_admin,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xEC08B84DF5C68F7C_u64,
        "Pump Fees Transfer Fee Sharing Authority",
        parse_pumpfees_transfer,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xEC2541724EBA7F6D,
        "Raydium CLMM Update Reward Infos",
        parse_raydium_clmm_update_reward_infos,
        Protocol::RaydiumClmm
    ),
    disc_entry!(0xEE61E64ED37FDBBD, "PumpFun Trade", parse_pumpfun_trade, Protocol::PumpFun),
    disc_entry!(
        0xF3D63778E297CCCB_u64,
        "Pump Fees Reset Fee Sharing Config",
        parse_pumpfees_reset,
        Protocol::PumpFees
    ),
    disc_entry!(
        0xF70E375C88267F79,
        "Meteora AMM Bootstrap Liquidity",
        parse_meteora_amm_bootstrap_liquidity,
        Protocol::MeteoraAmm
    ),
];

/// Fast lookup by discriminator - O(log n) binary search
///
/// With 33 entries, this requires at most 6 comparisons
#[inline(always)]
pub fn lookup_discriminator(discriminator: u64) -> Option<&'static DiscriminatorInfo> {
    DISCRIMINATOR_LUT
        .binary_search_by_key(&discriminator, |info| info.discriminator)
        .ok()
        .map(|idx| &DISCRIMINATOR_LUT[idx])
}

/// Get event name from discriminator
#[inline(always)]
pub fn discriminator_to_name(discriminator: u64) -> Option<&'static str> {
    lookup_discriminator(discriminator).map(|info| info.name)
}

/// Get protocol from discriminator
#[inline(always)]
pub fn discriminator_to_protocol(discriminator: u64) -> Option<Protocol> {
    lookup_discriminator(discriminator).map(|info| info.protocol)
}

/// Parse event using discriminator lookup
#[inline(always)]
pub fn parse_with_discriminator(
    discriminator: u64,
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let info = lookup_discriminator(discriminator)?;
    (info.parser)(data, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_is_sorted() {
        // Verify the LUT is sorted (required for binary search)
        for i in 1..DISCRIMINATOR_LUT.len() {
            assert!(
                DISCRIMINATOR_LUT[i - 1].discriminator < DISCRIMINATOR_LUT[i].discriminator,
                "LUT not sorted at index {}: {:x} >= {:x}",
                i,
                DISCRIMINATOR_LUT[i - 1].discriminator,
                DISCRIMINATOR_LUT[i].discriminator
            );
        }
    }

    #[test]
    fn test_discriminator_lookup() {
        // PumpFun Create
        let disc = 0x7663EBDE4DA91B1B;
        let info = lookup_discriminator(disc).unwrap();
        assert_eq!(info.name, "PumpFun Create");
        assert_eq!(info.protocol, Protocol::PumpFun);

        // Raydium CLMM Swap
        let disc = 0xE2710826E8CDC640;
        let info = lookup_discriminator(disc).unwrap();
        assert_eq!(info.name, "Raydium CLMM Swap");
        assert_eq!(info.protocol, Protocol::RaydiumClmm);

        // Unknown discriminator
        let disc = 0xFFFFFFFFFFFFFFFF;
        assert!(lookup_discriminator(disc).is_none());
    }

    #[test]
    fn test_event_name_lookup() {
        // PumpSwap Buy
        let disc = 0x7777F52C1F52F467;
        assert_eq!(discriminator_to_name(disc), Some("PumpSwap Buy"));

        // Raydium AMM Swap Base In
        let disc = 0x0900000000000000;
        assert_eq!(discriminator_to_name(disc), Some("Raydium AMM Swap Base In"));
    }

    #[test]
    fn test_protocol_lookup() {
        assert_eq!(discriminator_to_protocol(0x7663EBDE4DA91B1B), Some(Protocol::PumpFun));
        assert_eq!(discriminator_to_protocol(0xE2710826E8CDC640), Some(Protocol::RaydiumClmm));
        assert_eq!(discriminator_to_protocol(0x96A02B93AF49CAE1), Some(Protocol::OrcaWhirlpool));
    }
}
