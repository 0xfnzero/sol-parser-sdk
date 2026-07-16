use crate::core::events::*;
use crate::instr::inner_common::*;
use solana_sdk::pubkey::Pubkey;

// Meteora DAMM V2 Inner Instruction 解析器
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
    pub const SWAP: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 27, 60, 21, 213, 138, 170, 187, 147];
    pub const SWAP2: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 189, 66, 51, 168, 38, 80, 117, 153];
    pub const ADD_LIQUIDITY: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 175, 242, 8, 157, 30, 247, 185, 169];
    pub const REMOVE_LIQUIDITY: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 87, 46, 88, 98, 175, 96, 34, 91];
    pub const INITIALIZE_POOL: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 228, 50, 246, 85, 203, 66, 134, 37];
    pub const CREATE_POSITION: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION: [u8; 16] =
        [228, 69, 165, 46, 81, 203, 154, 29, 20, 145, 144, 68, 143, 142, 214, 178];
}

/// 主入口：根据 discriminator 解析事件
#[inline]
pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    match *disc {
        discriminators::SWAP => parse_swap(data, metadata),
        discriminators::SWAP2 => parse_swap2(data, metadata),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity(data, metadata),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity(data, metadata),
        discriminators::INITIALIZE_POOL => {
            crate::logs::meteora_damm::parse_initialize_pool_from_data(data, metadata)
        }
        discriminators::CREATE_POSITION => parse_create_position(data, metadata),
        discriminators::CLOSE_POSITION => parse_close_position(data, metadata),
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

/// Borsh 解析器
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构：pool(32) + amount_in(8) + output_amount(8) = 48 bytes
    const SWAP_EVENT_SIZE: usize = 32 + 8 + 8;
    if data.len() < SWAP_EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<MeteoraDammV2SwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;
    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent { metadata, ..event }))
}

/// 零拷贝解析器
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 8 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let amount_in = read_u64_unchecked(data, 32);
        let output_amount = read_u64_unchecked(data, 40);
        Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
            metadata,
            pool,
            amount_in,
            output_amount,
            ..Default::default()
        }))
    }
}

// ============================================================================
// Swap2 Event
// ============================================================================

/// 解析 Swap2 事件（统一入口）
#[inline(always)]
fn parse_swap2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_swap2_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_swap2_zero_copy(data, metadata)
    }
}

/// Borsh 解析器 for Swap2
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_swap2_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // Swap2 事件结构：
    // pool(32) + config(32) + trade_direction(1) + has_referral(1) +
    // amount_0(8) + amount_1(8) + swap_mode(1) +
    // included_fee_input_amount(8) + excluded_fee_input_amount(8) + amount_left(8) +
    // output_amount(8) + next_sqrt_price(16) +
    // trading_fee(8) + protocol_fee(8) + referral_fee(8) +
    // quote_reserve_amount(8) + migration_threshold(8) + current_timestamp(8)
    // = 32 + 32 + 1 + 1 + 8 + 8 + 1 + 8 + 8 + 8 + 8 + 16 + 8 + 8 + 8 + 8 + 8 + 8 = 177 bytes
    const SWAP2_EVENT_MIN_SIZE: usize = 177;
    if data.len() < SWAP2_EVENT_MIN_SIZE {
        return None;
    }

    let mut offset = 0;

    // 使用 unsafe 读取以提高性能
    unsafe {
        let pool = read_pubkey_unchecked(data, offset);
        offset += 32;

        let _config = read_pubkey_unchecked(data, offset);
        offset += 32;

        let trade_direction = read_u8_unchecked(data, offset);
        offset += 1;

        let has_referral = read_bool_unchecked(data, offset);
        offset += 1;

        let amount_0 = read_u64_unchecked(data, offset);
        offset += 8;

        let amount_1 = read_u64_unchecked(data, offset);
        offset += 8;

        let swap_mode = read_u8_unchecked(data, offset);
        offset += 1;

        let included_fee_input_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let _excluded_fee_input_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let _amount_left = read_u64_unchecked(data, offset);
        offset += 8;

        let output_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let next_sqrt_price = read_u128_unchecked(data, offset);
        offset += 16;

        let lp_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let protocol_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let referral_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let _quote_reserve_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let _migration_threshold = read_u64_unchecked(data, offset);
        offset += 8;

        let current_timestamp = read_u64_unchecked(data, offset);

        // 根据 swap_mode 确定 amount_in 和 minimum_amount_out
        let (amount_in, minimum_amount_out) =
            if swap_mode == 0 { (amount_0, amount_1) } else { (amount_1, amount_0) };

        Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
            metadata,
            pool,
            trade_direction,
            has_referral,
            amount_in,
            minimum_amount_out,
            output_amount,
            next_sqrt_price,
            lp_fee,
            protocol_fee,
            partner_fee: 0,
            referral_fee,
            actual_amount_in: included_fee_input_amount,
            current_timestamp,
            ..Default::default()
        }))
    }
}

/// 零拷贝解析器 for Swap2
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_swap2_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // Swap2 事件结构：
    // pool(32) + config(32) + trade_direction(1) + has_referral(1) +
    // amount_0(8) + amount_1(8) + swap_mode(1) +
    // included_fee_input_amount(8) + excluded_fee_input_amount(8) + amount_left(8) +
    // output_amount(8) + next_sqrt_price(16) +
    // trading_fee(8) + protocol_fee(8) + referral_fee(8) +
    // quote_reserve_amount(8) + migration_threshold(8) + current_timestamp(8)
    const SWAP2_EVENT_MIN_SIZE: usize = 177;

    unsafe {
        if !check_length(data, SWAP2_EVENT_MIN_SIZE) {
            return None;
        }

        let pool = read_pubkey_unchecked(data, 0);
        let trade_direction = read_u8_unchecked(data, 64);
        let has_referral = read_bool_unchecked(data, 65);
        let amount_0 = read_u64_unchecked(data, 66);
        let amount_1 = read_u64_unchecked(data, 74);
        let swap_mode = read_u8_unchecked(data, 82);
        let included_fee_input_amount = read_u64_unchecked(data, 83);
        let output_amount = read_u64_unchecked(data, 107);
        let next_sqrt_price = read_u128_unchecked(data, 115);
        let lp_fee = read_u64_unchecked(data, 131);
        let protocol_fee = read_u64_unchecked(data, 139);
        let referral_fee = read_u64_unchecked(data, 147);
        let current_timestamp = read_u64_unchecked(data, 169);

        // 根据 swap_mode 确定 amount_in 和 minimum_amount_out
        let (amount_in, minimum_amount_out) =
            if swap_mode == 0 { (amount_0, amount_1) } else { (amount_1, amount_0) };

        Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
            metadata,
            pool,
            trade_direction,
            has_referral,
            amount_in,
            minimum_amount_out,
            output_amount,
            next_sqrt_price,
            lp_fee,
            protocol_fee,
            partner_fee: 0,
            referral_fee,
            actual_amount_in: included_fee_input_amount,
            current_timestamp,
            ..Default::default()
        }))
    }
}

// ============================================================================
// AddLiquidity Event
// ============================================================================

/// 解析 AddLiquidity 事件（统一入口）
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

/// Borsh 解析器
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_add_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构：pool(32) + position(32) + owner(32) + token_a_amount(8) + token_b_amount(8) = 112 bytes
    const ADD_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
    if data.len() < ADD_LIQUIDITY_EVENT_SIZE {
        return None;
    }

    let event =
        borsh::from_slice::<MeteoraDammV2AddLiquidityEvent>(&data[..ADD_LIQUIDITY_EVENT_SIZE])
            .ok()?;
    Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent { metadata, ..event }))
}

/// 零拷贝解析器
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_add_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 8 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let position = read_pubkey_unchecked(data, 32);
        let owner = read_pubkey_unchecked(data, 64);
        let token_a_amount = read_u64_unchecked(data, 96);
        let token_b_amount = read_u64_unchecked(data, 104);
        Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
            metadata,
            pool,
            position,
            owner,
            token_a_amount,
            token_b_amount,
            liquidity_delta: 0,
            token_a_amount_threshold: 0,
            token_b_amount_threshold: 0,
            total_amount_a: 0,
            total_amount_b: 0,
        }))
    }
}

// ============================================================================
// RemoveLiquidity Event
// ============================================================================

/// 解析 RemoveLiquidity 事件（统一入口）
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

/// Borsh 解析器
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_remove_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构：pool(32) + position(32) + owner(32) + token_a_amount(8) + token_b_amount(8) = 112 bytes
    const REMOVE_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
    if data.len() < REMOVE_LIQUIDITY_EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<MeteoraDammV2RemoveLiquidityEvent>(
        &data[..REMOVE_LIQUIDITY_EVENT_SIZE],
    )
    .ok()?;
    Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_remove_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 8 + 8) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let position = read_pubkey_unchecked(data, 32);
        let owner = read_pubkey_unchecked(data, 64);
        let token_a_amount = read_u64_unchecked(data, 96);
        let token_b_amount = read_u64_unchecked(data, 104);
        Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
            metadata,
            pool,
            position,
            owner,
            token_a_amount,
            token_b_amount,
            liquidity_delta: 0,
            token_a_amount_threshold: 0,
            token_b_amount_threshold: 0,
        }))
    }
}

// ============================================================================
// CreatePosition Event
// ============================================================================

/// 解析 CreatePosition 事件（统一入口）
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

/// Borsh 解析器
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_create_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构：pool(32) + owner(32) + position(32) + position_nft_mint(32) = 128 bytes
    const CREATE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 32;
    if data.len() < CREATE_POSITION_EVENT_SIZE {
        return None;
    }

    let event =
        borsh::from_slice::<MeteoraDammV2CreatePositionEvent>(&data[..CREATE_POSITION_EVENT_SIZE])
            .ok()?;
    Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_create_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 32) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let owner = read_pubkey_unchecked(data, 32);
        let position = read_pubkey_unchecked(data, 64);
        let position_nft_mint = read_pubkey_unchecked(data, 96);
        Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
            metadata,
            pool,
            owner,
            position,
            position_nft_mint,
        }))
    }
}

// ============================================================================
// ClosePosition Event
// ============================================================================

/// 解析 ClosePosition 事件（统一入口）
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

/// Borsh 解析器
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_close_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构：pool(32) + owner(32) + position(32) + position_nft_mint(32) = 128 bytes
    const CLOSE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 32;
    if data.len() < CLOSE_POSITION_EVENT_SIZE {
        return None;
    }

    let event =
        borsh::from_slice::<MeteoraDammV2ClosePositionEvent>(&data[..CLOSE_POSITION_EVENT_SIZE])
            .ok()?;
    Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_close_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 32) {
            return None;
        }
        let pool = read_pubkey_unchecked(data, 0);
        let owner = read_pubkey_unchecked(data, 32);
        let position = read_pubkey_unchecked(data, 64);
        let position_nft_mint = read_pubkey_unchecked(data, 96);
        Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
            metadata,
            pool,
            owner,
            position,
            position_nft_mint,
        }))
    }
}
