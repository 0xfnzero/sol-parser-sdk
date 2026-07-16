//! PumpSwap (Pump AMM) Inner Instruction 解析器
//!
//! Inner instructions 使用 16 字节的 discriminator
//!
//! ## 解析器插件系统
//!
//! 本模块提供两种可插拔的解析器实现：
//!
//! ### 1. Borsh 反序列化解析器（默认，推荐）
//! - **启用**: `cargo build --features parse-borsh` （默认）
//! - **优点**: 类型安全、代码简洁、易维护、自动验证
//! - **适用**: 一般场景、需要稳定性和可维护性的项目
//!
//! ### 2. 零拷贝解析器（高性能）
//! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
//! - **优点**: 最快、零拷贝、无验证开销、适合超高频场景
//! - **适用**: 性能关键路径、每秒数万次解析的场景
//!
//! ## 使用示例
//!
//! ```bash
//! # 使用 Borsh 解析器（推荐，默认）
//! cargo build --release
//!
//! # 使用零拷贝解析器（极致性能）
//! cargo build --release --features parse-zero-copy --no-default-features
//! ```

use crate::core::events::*;
use crate::instr::inner_common::*;

/// PumpSwap inner instruction discriminators (16 bytes)
/// Format: [event_magic (8 bytes) | event_discriminator (8 bytes)]
/// The magic prefix is: [228, 69, 165, 46, 81, 203, 154, 29]
/// The event_discriminator matches the 8-byte log discriminator for each event type
pub mod discriminators {
    /// Common magic prefix for all PumpSwap inner instructions
    #[allow(dead_code)]
    const MAGIC_PREFIX: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

    /// BuyEvent
    /// Full discriminator: MAGIC_PREFIX + [103, 244, 82, 31, 44, 245, 119, 119]
    pub const BUY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        103, 244, 82, 31, 44, 245, 119, 119, // BuyEvent hash
    ];

    /// SellEvent
    /// Full discriminator: MAGIC_PREFIX + [62, 47, 55, 10, 165, 3, 220, 42]
    pub const SELL: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        62, 47, 55, 10, 165, 3, 220, 42, // SellEvent hash
    ];

    /// CreatePoolEvent
    /// Full discriminator: MAGIC_PREFIX + [177, 49, 12, 210, 160, 118, 167, 116]
    pub const CREATE_POOL: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        177, 49, 12, 210, 160, 118, 167, 116, // CreatePoolEvent hash
    ];

    /// DepositEvent (Add Liquidity)
    /// Full discriminator: MAGIC_PREFIX + [120, 248, 61, 83, 31, 142, 107, 144]
    pub const ADD_LIQUIDITY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        120, 248, 61, 83, 31, 142, 107, 144, // AddLiquidityEvent hash
    ];

    /// WithdrawEvent (Remove Liquidity)
    /// Full discriminator: MAGIC_PREFIX + [22, 9, 133, 26, 160, 44, 71, 192]
    pub const REMOVE_LIQUIDITY: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, // magic prefix
        22, 9, 133, 26, 160, 44, 71, 192, // RemoveLiquidityEvent hash
    ];
}

/// 解析 PumpSwap inner instruction (统一入口)
#[inline]
pub fn parse_pumpswap_inner_instruction(
    discriminator: &[u8; 16],
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    match *discriminator {
        discriminators::BUY => parse_buy_inner(data, metadata),
        discriminators::SELL => parse_sell_inner(data, metadata),
        discriminators::CREATE_POOL => parse_create_pool_inner(data, metadata),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity_inner(data, metadata),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity_inner(data, metadata),
        _ => None,
    }
}

// ============================================================================
// Buy 事件解析器
// ============================================================================

/// 解析 Buy 事件（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_buy_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_buy_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_buy_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - Buy 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_buy_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // PumpSwap BuyEvent 含可变长度 ix_name 及 cashback 字段，反序列化整段 data
    let event = borsh::from_slice::<PumpSwapBuyEvent>(data).ok()?;

    // 设置 metadata
    Some(DexEvent::PumpSwapBuy(PumpSwapBuyEvent { metadata, ..event }))
}

/// 零拷贝解析器 - Buy 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_buy_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // PumpSwap Buy 事件固定字段到 last_update_timestamp 共 385 bytes.
    // timestamp: i64 (8)
    // base_amount_out: u64 (8)
    // max_quote_amount_in: u64 (8)
    // user_base_token_reserves: u64 (8)
    // user_quote_token_reserves: u64 (8)
    // pool_base_token_reserves: u64 (8)
    // pool_quote_token_reserves: u64 (8)
    // quote_amount_in: u64 (8)
    // lp_fee_basis_points: u64 (8)
    // lp_fee: u64 (8)
    // protocol_fee_basis_points: u64 (8)
    // protocol_fee: u64 (8)
    // quote_amount_in_with_lp_fee: u64 (8)
    // user_quote_amount_in: u64 (8)
    // pool: Pubkey (32)
    // user: Pubkey (32)
    // user_base_token_account: Pubkey (32)
    // user_quote_token_account: Pubkey (32)
    // protocol_fee_recipient: Pubkey (32)
    // protocol_fee_recipient_token_account: Pubkey (32)
    // coin_creator: Pubkey (32)
    // coin_creator_fee_basis_points: u64 (8)
    // coin_creator_fee: u64 (8)
    // track_volume: bool (1)
    // total_unclaimed_tokens: u64 (8)
    // total_claimed_tokens: u64 (8)
    // current_sol_volume: u64 (8)
    // last_update_timestamp: i64 (8)

    unsafe {
        const MIN_SIZE: usize = 16 * 8 + 32 * 7 + 1 + 4 * 8;
        if !check_length(data, MIN_SIZE) {
            return None;
        }

        let mut offset = 0;

        // 解析数值字段
        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;
        let base_amount_out = read_u64_unchecked(data, offset);
        offset += 8;
        let max_quote_amount_in = read_u64_unchecked(data, offset);
        offset += 8;
        let user_base_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let user_quote_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let pool_base_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let pool_quote_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_in = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let protocol_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let protocol_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_in_with_lp_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let user_quote_amount_in = read_u64_unchecked(data, offset);
        offset += 8;

        // 解析 Pubkey 字段
        let pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user_base_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user_quote_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let protocol_fee_recipient = read_pubkey_unchecked(data, offset);
        offset += 32;
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let coin_creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let coin_creator_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let coin_creator_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let track_volume = data[offset] != 0;
        offset += 1;

        let total_unclaimed_tokens = read_u64_unchecked(data, offset);
        offset += 8;
        let total_claimed_tokens = read_u64_unchecked(data, offset);
        offset += 8;
        let current_sol_volume = read_u64_unchecked(data, offset);
        offset += 8;
        let last_update_timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        // min_base_amount_out, ix_name (variable), cashback_fee_basis_points, cashback
        let min_base_amount_out = if offset + 8 <= data.len() {
            let v = read_u64_unchecked(data, offset);
            offset += 8;
            v
        } else {
            0
        };
        let ix_name = if offset + 4 <= data.len() {
            if let Some((s, consumed)) = read_string_unchecked(data, offset) {
                offset += consumed;
                s
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let cashback_fee_basis_points = if offset + 8 <= data.len() {
            let v = read_u64_unchecked(data, offset);
            offset += 8;
            v
        } else {
            0
        };
        let cashback = if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };

        Some(DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
            metadata,
            timestamp,
            base_amount_out,
            max_quote_amount_in,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            quote_amount_in,
            lp_fee_basis_points,
            lp_fee,
            protocol_fee_basis_points,
            protocol_fee,
            quote_amount_in_with_lp_fee,
            user_quote_amount_in,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            protocol_fee_recipient,
            protocol_fee_recipient_token_account,
            coin_creator,
            coin_creator_fee_basis_points,
            coin_creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            min_base_amount_out,
            ix_name,
            cashback_fee_basis_points,
            cashback,
            ..Default::default()
        }))
    }
}

// ============================================================================
// Sell 事件解析器
// ============================================================================

/// 解析 Sell 事件（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_sell_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
    {
        parse_sell_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_sell_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - Sell 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(all(feature = "parse-borsh", not(feature = "parse-zero-copy")))]
#[inline(always)]
fn parse_sell_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // PumpSwap SellEvent 含 cashback_fee_basis_points, cashback (368 bytes 固定部分)
    const SELL_EVENT_SIZE: usize = 368;

    if data.len() < SELL_EVENT_SIZE {
        return None;
    }

    // 使用 Borsh 反序列化完整的事件数据
    let event = borsh::from_slice::<PumpSwapSellEvent>(&data[..SELL_EVENT_SIZE]).ok()?;

    // 设置 metadata 并设置 is_pump_pool 标志
    Some(DexEvent::PumpSwapSell(PumpSwapSellEvent {
        metadata,
        is_pump_pool: true, // 标记为 PumpSwap pool
        ..event
    }))
}

/// 零拷贝解析器 - Sell 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_sell_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // PumpSwap Sell 事件数据结构 (352 bytes):
    // timestamp: i64 (8)
    // base_amount_in: u64 (8)
    // min_quote_amount_out: u64 (8)
    // user_base_token_reserves: u64 (8)
    // user_quote_token_reserves: u64 (8)
    // pool_base_token_reserves: u64 (8)
    // pool_quote_token_reserves: u64 (8)
    // quote_amount_out: u64 (8)
    // lp_fee_basis_points: u64 (8)
    // lp_fee: u64 (8)
    // protocol_fee_basis_points: u64 (8)
    // protocol_fee: u64 (8)
    // quote_amount_out_without_lp_fee: u64 (8)
    // user_quote_amount_out: u64 (8)
    // pool: Pubkey (32)
    // user: Pubkey (32)
    // user_base_token_account: Pubkey (32)
    // user_quote_token_account: Pubkey (32)
    // protocol_fee_recipient: Pubkey (32)
    // protocol_fee_recipient_token_account: Pubkey (32)
    // coin_creator: Pubkey (32)
    // coin_creator_fee_basis_points: u64 (8)
    // coin_creator_fee: u64 (8)
    // cashback_fee_basis_points: u64 (8)
    // cashback: u64 (8)

    unsafe {
        const MIN_SIZE: usize = 8 * 16 + 32 * 7 + 8 + 8; // 368
        if !check_length(data, MIN_SIZE) {
            return None;
        }

        let mut offset = 0;

        // 解析数值字段
        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;
        let base_amount_in = read_u64_unchecked(data, offset);
        offset += 8;
        let min_quote_amount_out = read_u64_unchecked(data, offset);
        offset += 8;
        let user_base_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let user_quote_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let pool_base_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let pool_quote_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_out = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let protocol_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let protocol_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_out_without_lp_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let user_quote_amount_out = read_u64_unchecked(data, offset);
        offset += 8;

        // 解析 Pubkey 字段
        let pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user_base_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let user_quote_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let protocol_fee_recipient = read_pubkey_unchecked(data, offset);
        offset += 32;
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, offset);
        offset += 32;
        let coin_creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let coin_creator_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let coin_creator_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let cashback_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;
        let cashback = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapSell(PumpSwapSellEvent {
            metadata,
            timestamp,
            base_amount_in,
            min_quote_amount_out,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            quote_amount_out,
            lp_fee_basis_points,
            lp_fee,
            protocol_fee_basis_points,
            protocol_fee,
            quote_amount_out_without_lp_fee,
            user_quote_amount_out,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            protocol_fee_recipient,
            protocol_fee_recipient_token_account,
            coin_creator,
            coin_creator_fee_basis_points,
            coin_creator_fee,
            cashback_fee_basis_points,
            cashback,
            is_pump_pool: true,
            ..Default::default()
        }))
    }
}

/// 解析 CreatePool 事件
#[inline(always)]
fn parse_create_pool_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 32 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let quote_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
            metadata,
            pool,
            creator,
            base_mint,
            quote_mint,
            base_amount_in: base_amount,
            quote_amount_in: quote_amount,
            ..Default::default()
        }))
    }
}

/// 解析 AddLiquidity 事件
#[inline(always)]
fn parse_add_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let _pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let _user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let base_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let lp_amount = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapLiquidityAdded(PumpSwapLiquidityAdded {
            metadata,
            base_amount_in: base_amount,
            quote_amount_in: quote_amount,
            lp_token_amount_out: lp_amount,
            ..Default::default()
        }))
    }
}

/// 解析 RemoveLiquidity 事件
#[inline(always)]
fn parse_remove_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let _pool = read_pubkey_unchecked(data, offset);
        offset += 32;
        let _user = read_pubkey_unchecked(data, offset);
        offset += 32;
        let lp_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let base_amount_out = read_u64_unchecked(data, offset);
        offset += 8;
        let quote_amount_out = read_u64_unchecked(data, offset);

        Some(DexEvent::PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved {
            metadata,
            lp_token_amount_in: lp_amount,
            base_amount_out,
            quote_amount_out,
            ..Default::default()
        }))
    }
}
