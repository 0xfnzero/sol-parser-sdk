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
    // 16-byte discriminators: 8-byte event hash + 8-byte magic
    pub const SWAP: [u8; 16] =
        [143, 190, 90, 218, 196, 30, 51, 222, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const ADD_LIQUIDITY: [u8; 16] =
        [181, 157, 89, 67, 143, 182, 52, 72, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const REMOVE_LIQUIDITY: [u8; 16] =
        [80, 85, 209, 72, 24, 206, 35, 178, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const INITIALIZE_POOL: [u8; 16] =
        [95, 180, 10, 172, 84, 174, 232, 40, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const INITIALIZE_BIN_ARRAY: [u8; 16] =
        [11, 18, 155, 194, 33, 115, 238, 119, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CREATE_POSITION: [u8; 16] =
        [123, 233, 11, 43, 146, 180, 97, 119, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CLOSE_POSITION: [u8; 16] =
        [94, 168, 102, 45, 59, 122, 137, 54, 155, 167, 108, 32, 122, 76, 173, 64];
    pub const CLAIM_FEE: [u8; 16] =
        [152, 70, 208, 111, 104, 91, 44, 1, 155, 167, 108, 32, 122, 76, 173, 64];
}

/// 主入口：根据 discriminator 解析事件
#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::SWAP => parse_swap(data, metadata),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity(data, metadata),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity(data, metadata),
        discriminators::INITIALIZE_POOL => parse_initialize_pool(data, metadata),
        discriminators::INITIALIZE_BIN_ARRAY => parse_initialize_bin_array(data, metadata),
        discriminators::CREATE_POSITION => parse_create_position(data, metadata),
        discriminators::CLOSE_POSITION => parse_close_position(data, metadata),
        discriminators::CLAIM_FEE => parse_claim_fee(data, metadata),
        _ => None,
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
// Initialize Pool Event
// ============================================================================

/// 解析 Initialize Pool 事件（统一入口）
#[inline(always)]
fn parse_initialize_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_initialize_pool_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_initialize_pool_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Initialize Pool
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_initialize_pool_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + creator(32) + active_bin_id(4) + bin_step(2) = 70 bytes
    const INITIALIZE_POOL_EVENT_SIZE: usize = 32 + 32 + 4 + 2;
    if data.len() < INITIALIZE_POOL_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmInitializePoolEvent>(&data[..INITIALIZE_POOL_EVENT_SIZE])
            .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmInitializePool(event))
}

/// 零拷贝解析器 - Initialize Pool
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_initialize_pool_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 4 + 2) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let creator = read_pubkey_unchecked(data, 32);
        let active_bin_id = read_i32_unchecked(data, 64);
        let bin_step = read_u16_unchecked(data, 68);
        Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
            metadata,
            pool,
            creator,
            active_bin_id,
            bin_step,
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
// Create Position Event
// ============================================================================

/// 解析 Create Position 事件（统一入口）
#[inline(always)]
fn parse_create_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_create_position_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_create_position_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Create Position
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_create_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + position(32) + owner(32) + lower_bin_id(4) + width(4) = 104 bytes
    const CREATE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 4 + 4;
    if data.len() < CREATE_POSITION_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmCreatePositionEvent>(&data[..CREATE_POSITION_EVENT_SIZE])
            .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmCreatePosition(event))
}

/// 零拷贝解析器 - Create Position
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_create_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 4 + 4) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let position = read_pubkey_unchecked(data, 32);
        let owner = read_pubkey_unchecked(data, 64);
        let lower_bin_id = read_i32_unchecked(data, 96);
        let width = read_u32_unchecked(data, 100);
        Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
            metadata,
            pool,
            position,
            owner,
            lower_bin_id,
            width,
        }))
    }
}

// ============================================================================
// Close Position Event
// ============================================================================

/// 解析 Close Position 事件（统一入口）
#[inline(always)]
fn parse_close_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_close_position_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_close_position_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 - Close Position
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_close_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // pool(32) + position(32) + owner(32) = 96 bytes
    const CLOSE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32;
    if data.len() < CLOSE_POSITION_EVENT_SIZE {
        return None;
    }

    let mut event =
        borsh::from_slice::<MeteoraDlmmClosePositionEvent>(&data[..CLOSE_POSITION_EVENT_SIZE])
            .ok()?;
    event.metadata = metadata;
    Some(DexEvent::MeteoraDlmmClosePosition(event))
}

/// 零拷贝解析器 - Close Position
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_close_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let position = read_pubkey_unchecked(data, 32);
        let owner = read_pubkey_unchecked(data, 64);
        Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
            metadata,
            pool,
            position,
            owner,
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
