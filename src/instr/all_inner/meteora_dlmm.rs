use crate::core::events::*;
use crate::instr::inner_common::*;
use solana_sdk::pubkey::Pubkey;

// Meteora DLMM Inner Instruction 解析器
//
// ## 解析器插件系统
//
// 支持两种可插拔的解析器实现：
//
// ### 1. Borsh 反序列化解析器（默认，推荐）
// - **启用**: `cargo build --features parse-borsh` （默认）
// - 特点：类型安全、代码简洁、易于维护
//
// ### 2. 零拷贝解析器（高性能）
// - **启用**: `cargo build --features parse-zero-copy --no-default-features`
// - 特点：最高性能、零内存分配、直接读取内存
pub mod discriminators {
    pub const SWAP: [u8; 8] = [81, 108, 227, 190, 205, 208, 10, 196];
    pub const SWAP2: [u8; 8] = [46, 116, 82, 215, 148, 27, 84, 77];
    pub const ADD_LIQUIDITY: [u8; 8] = [31, 94, 125, 90, 227, 52, 61, 186];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [116, 244, 97, 232, 103, 31, 152, 58];
    pub const INITIALIZE_POOL: [u8; 8] = [185, 74, 252, 125, 27, 215, 188, 111];
    pub const INITIALIZE_BIN_ARRAY: [u8; 8] = [11, 18, 155, 194, 33, 115, 238, 119];
    pub const CREATE_POSITION: [u8; 8] = [144, 142, 252, 84, 157, 53, 37, 121];
    pub const CLOSE_POSITION: [u8; 8] = [255, 196, 16, 107, 28, 202, 53, 128];
    pub const CLAIM_FEE: [u8; 8] = [75, 122, 154, 48, 140, 74, 123, 163];
    pub const CLAIM_FEE2: [u8; 8] = [232, 171, 242, 97, 58, 77, 35, 45];
}

const EVENT_CPI_PREFIX: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];
const LEGACY_EVENT_CPI_SUFFIX: [u8; 8] = [155, 167, 108, 32, 122, 76, 173, 64];

#[inline(always)]
pub(crate) fn is_event_cpi(data: &[u8]) -> bool {
    data.len() >= 16 && (data[..8] == EVENT_CPI_PREFIX || data[8..16] == LEGACY_EVENT_CPI_SUFFIX)
}

#[inline(always)]
fn event_discriminator(disc: &[u8; 16]) -> Option<[u8; 8]> {
    if disc[..8] == EVENT_CPI_PREFIX {
        return disc[8..].try_into().ok();
    }
    if disc[8..] == LEGACY_EVENT_CPI_SUFFIX {
        return disc[..8].try_into().ok();
    }
    None
}

/// 主入口：根据 discriminator 解析事件
#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match event_discriminator(disc)? {
        discriminators::SWAP => parse_swap(data, metadata),
        discriminators::SWAP2 => parse_swap2(data, metadata),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity(data, metadata),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity(data, metadata),
        discriminators::INITIALIZE_POOL => parse_lb_pair_create(data, metadata),
        discriminators::INITIALIZE_BIN_ARRAY => parse_initialize_bin_array(data, metadata),
        discriminators::CREATE_POSITION => parse_position_create(data, metadata),
        discriminators::CLOSE_POSITION => parse_position_close(data, metadata),
        discriminators::CLAIM_FEE => parse_claim_fee(data, metadata),
        discriminators::CLAIM_FEE2 => parse_claim_fee2(data, metadata),
        _ => None,
    }
}

#[inline(always)]
fn parse_lb_pair_create(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 2 + 32 + 32) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
            metadata,
            pool,
            creator: Pubkey::default(),
            active_bin_id: 0,
            bin_step: read_u16_unchecked(data, 32),
        }))
    }
}

#[inline(always)]
fn parse_position_create(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32) {
            return None;
        }
        Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
            metadata,
            pool: read_pubkey_unchecked(data, 0),
            position: read_pubkey_unchecked(data, 32),
            owner: read_pubkey_unchecked(data, 64),
            lower_bin_id: 0,
            width: 0,
        }))
    }
}

#[inline(always)]
fn parse_position_close(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32) {
            return None;
        }
        Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
            metadata,
            pool: Pubkey::default(),
            position: read_pubkey_unchecked(data, 0),
            owner: read_pubkey_unchecked(data, 32),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swap2_payload() -> Vec<u8> {
        let mut data = vec![0u8; 147];
        data[72] = 1;
        data[73..89].copy_from_slice(&25u128.to_le_bytes());
        data[89..97].copy_from_slice(&100u64.to_le_bytes());
        data[105..113].copy_from_slice(&90u64.to_le_bytes());
        data[113..121].copy_from_slice(&3u64.to_le_bytes());
        data[121..129].copy_from_slice(&2u64.to_le_bytes());
        data[137..145].copy_from_slice(&1u64.to_le_bytes());
        data
    }

    #[test]
    fn parses_current_anchor_event_cpi_prefix_layout() {
        let mut disc = [0u8; 16];
        disc[..8].copy_from_slice(&EVENT_CPI_PREFIX);
        disc[8..].copy_from_slice(&discriminators::SWAP2);

        let event = parse(&disc, &swap2_payload(), EventMetadata::default()).expect("swap2");
        let DexEvent::MeteoraDlmmSwap(event) = event else {
            panic!("unexpected event");
        };
        assert_eq!(event.amount_in, 100);
        assert_eq!(event.amount_out, 90);
        assert_eq!(event.fee, 3);
    }

    #[test]
    fn keeps_legacy_event_cpi_suffix_layout() {
        let mut disc = [0u8; 16];
        disc[..8].copy_from_slice(&discriminators::SWAP2);
        disc[8..].copy_from_slice(&LEGACY_EVENT_CPI_SUFFIX);
        assert!(parse(&disc, &swap2_payload(), EventMetadata::default()).is_some());
    }

    #[test]
    fn current_position_events_use_current_idl_lengths() {
        let mut create_disc = [0u8; 16];
        create_disc[..8].copy_from_slice(&EVENT_CPI_PREFIX);
        create_disc[8..].copy_from_slice(&discriminators::CREATE_POSITION);
        assert!(parse(&create_disc, &[0u8; 96], EventMetadata::default()).is_some());

        let mut close_disc = [0u8; 16];
        close_disc[..8].copy_from_slice(&EVENT_CPI_PREFIX);
        close_disc[8..].copy_from_slice(&discriminators::CLOSE_POSITION);
        assert!(parse(&close_disc, &[0u8; 64], EventMetadata::default()).is_some());
    }
}

// ============================================================================
// Swap Event
// ============================================================================

/// 解析 Swap 事件（统一入口）
#[inline(always)]
fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_swap_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_swap_zero_copy(data, metadata)
    }
}

#[inline(always)]
fn parse_swap2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 4 + 4 + 1 + 16 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 1) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let from = read_pubkey_unchecked(data, 32);
        let start_bin_id = read_i32_unchecked(data, 64);
        let end_bin_id = read_i32_unchecked(data, 68);
        let swap_for_y = read_bool_unchecked(data, 72);
        let fee_bps = read_u128_unchecked(data, 73);
        let amount_in = read_u64_unchecked(data, 89);
        let amount_out = read_u64_unchecked(data, 105);
        let fee = read_u64_unchecked(data, 113);
        let protocol_fee = read_u64_unchecked(data, 121);
        let host_fee = read_u64_unchecked(data, 137);
        Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
            metadata,
            pool,
            from,
            start_bin_id,
            end_bin_id,
            amount_in,
            amount_out,
            swap_for_y,
            fee,
            protocol_fee,
            fee_bps,
            host_fee,
        }))
    }
}

#[inline(always)]
fn parse_claim_fee2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    if data.len() < 116 {
        return None;
    }
    parse_claim_fee(data, metadata)
}

/// Borsh 解析器 - Swap
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + from(32) + start_bin_id(4) + end_bin_id(4) + amount_in(8) + amount_out(8) + swap_for_y(1) + fee(8) + protocol_fee(8) + fee_bps(16) + host_fee(8) = 129 bytes
    const SWAP_EVENT_SIZE: usize = 32 + 32 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 16 + 8;
    if data.len() < SWAP_EVENT_SIZE {
        return None;
    }

    let mut event = borsh::from_slice::<MeteoraDlmmSwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmSwap(event))
}

/// 零拷贝解析器 - Swap
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 16 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let from = read_pubkey_unchecked(data, 32);
        let start_bin_id = read_i32_unchecked(data, 64);
        let end_bin_id = read_i32_unchecked(data, 68);
        let amount_in = read_u64_unchecked(data, 72);
        let amount_out = read_u64_unchecked(data, 80);
        let swap_for_y = read_bool_unchecked(data, 88);
        let fee = read_u64_unchecked(data, 89);
        let protocol_fee = read_u64_unchecked(data, 97);
        let fee_bps = read_u128_unchecked(data, 105);
        let host_fee = read_u64_unchecked(data, 121);
        Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
            metadata,
            pool,
            from,
            start_bin_id,
            end_bin_id,
            amount_in,
            amount_out,
            swap_for_y,
            fee,
            protocol_fee,
            fee_bps,
            host_fee,
        }))
    }
}

// ============================================================================
// Add Liquidity Event
// ============================================================================

/// 解析 Add Liquidity 事件（统一入口）
#[inline(always)]
fn parse_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_add_liquidity_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_add_liquidity_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Add Liquidity
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_add_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + from(32) + position(32) + amounts[2](16) + active_bin_id(4) = 116 bytes
    const ADD_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 16 + 4;
    if data.len() < ADD_LIQUIDITY_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmAddLiquidityEvent>(&data[..ADD_LIQUIDITY_EVENT_SIZE])
            .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmAddLiquidity(event))
}

/// 零拷贝解析器 - Add Liquidity
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_add_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 16 + 4) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let from = read_pubkey_unchecked(data, 32);
        let position = read_pubkey_unchecked(data, 64);
        let amount_0 = read_u64_unchecked(data, 96);
        let amount_1 = read_u64_unchecked(data, 104);
        let active_bin_id = read_i32_unchecked(data, 112);
        Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
            metadata,
            pool,
            from,
            position,
            amounts: [amount_0, amount_1],
            active_bin_id,
        }))
    }
}

// ============================================================================
// Remove Liquidity Event
// ============================================================================

/// 解析 Remove Liquidity 事件（统一入口）
#[inline(always)]
fn parse_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_remove_liquidity_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_remove_liquidity_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Remove Liquidity
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_remove_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + from(32) + position(32) + amounts[2](16) + active_bin_id(4) = 116 bytes
    const REMOVE_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 16 + 4;
    if data.len() < REMOVE_LIQUIDITY_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmRemoveLiquidityEvent>(&data[..REMOVE_LIQUIDITY_EVENT_SIZE])
            .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmRemoveLiquidity(event))
}

/// 零拷贝解析器 - Remove Liquidity
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_remove_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 16 + 4) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let from = read_pubkey_unchecked(data, 32);
        let position = read_pubkey_unchecked(data, 64);
        let amount_0 = read_u64_unchecked(data, 96);
        let amount_1 = read_u64_unchecked(data, 104);
        let active_bin_id = read_i32_unchecked(data, 112);
        Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
            metadata,
            pool,
            from,
            position,
            amounts: [amount_0, amount_1],
            active_bin_id,
        }))
    }
}

// ============================================================================
// Initialize Bin Array Event
// ============================================================================

/// 解析 Initialize Bin Array 事件（统一入口）
#[inline(always)]
fn parse_initialize_bin_array(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_initialize_bin_array_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_initialize_bin_array_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Initialize Bin Array
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_initialize_bin_array_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + bin_array(32) + index(8) = 72 bytes
    const INITIALIZE_BIN_ARRAY_EVENT_SIZE: usize = 32 + 32 + 8;
    if data.len() < INITIALIZE_BIN_ARRAY_EVENT_SIZE {
        return None;
    }

    let mut event = borsh::from_slice::<MeteoraDlmmInitializeBinArrayEvent>(
        &data[..INITIALIZE_BIN_ARRAY_EVENT_SIZE],
    )
    .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmInitializeBinArray(event))
}

/// 零拷贝解析器 - Initialize Bin Array
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_initialize_bin_array_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let bin_array = read_pubkey_unchecked(data, 32);
        let index = read_i64_unchecked(data, 64);
        Some(DexEvent::MeteoraDlmmInitializeBinArray(MeteoraDlmmInitializeBinArrayEvent {
            metadata,
            pool,
            bin_array,
            index,
        }))
    }
}

// ============================================================================
// Claim Fee Event
// ============================================================================

/// 解析 Claim Fee 事件（统一入口）
#[inline(always)]
fn parse_claim_fee(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_claim_fee_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_claim_fee_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Claim Fee
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_claim_fee_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + position(32) + owner(32) + fee_x(8) + fee_y(8) = 112 bytes
    const CLAIM_FEE_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
    if data.len() < CLAIM_FEE_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmClaimFeeEvent>(&data[..CLAIM_FEE_EVENT_SIZE]).ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmClaimFee(event))
}

/// 零拷贝解析器 - Claim Fee
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_claim_fee_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 8 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let position = read_pubkey_unchecked(data, 32);
        let owner = read_pubkey_unchecked(data, 64);
        let fee_x = read_u64_unchecked(data, 96);
        let fee_y = read_u64_unchecked(data, 104);
        Some(DexEvent::MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent {
            metadata,
            pool,
            position,
            owner,
            fee_x,
            fee_y,
        }))
    }
}
