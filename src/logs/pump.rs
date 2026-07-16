//! Pump.fun `Program log` → [`DexEvent`](crate::core::events::DexEvent) (SIMD / zero-copy hot path).
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use memchr::memmem;
use once_cell::sync::Lazy;

#[cfg(feature = "perf-stats")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "perf-stats")]
pub static PARSE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "perf-stats")]
pub static PARSE_TIME_NS: AtomicUsize = AtomicUsize::new(0);

// --- discriminators ------------------------------------------------

pub const CREATE_EVENT: u64 = u64::from_le_bytes([27, 114, 169, 77, 222, 235, 99, 118]);
pub const TRADE_EVENT: u64 = u64::from_le_bytes([189, 219, 127, 211, 78, 230, 97, 238]);
pub const MIGRATE_EVENT: u64 = u64::from_le_bytes([189, 233, 93, 185, 92, 148, 234, 148]);
/// `createFeeSharingConfigEvent`（pump-fees IDL）
pub const CREATE_FEE_SHARING_CONFIG_EVENT: u64 = crate::logs::pump_fees::discriminant_u64(
    &crate::logs::pump_fees::CREATE_FEE_SHARING_CONFIG_EVENT_DISC,
);
/// `migrateBondingCurveCreatorEvent`（pump.fun IDL）
pub const MIGRATE_BONDING_CURVE_CREATOR_EVENT: u64 =
    u64::from_le_bytes([155, 167, 104, 220, 213, 108, 243, 3]);

#[inline]
pub fn normalize_pumpfun_ix_name(ix_name: &str) -> &str {
    match ix_name {
        "buy_v2" => "buy",
        "sell_v2" => "sell",
        "buy_exact_quote_in_v2" => "buy_exact_quote_in",
        other => other,
    }
}

// --- binary_read ---------------------------------------------------

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset..offset + 8` is within `data`.
pub unsafe fn read_u64_unchecked(data: &[u8], offset: usize) -> u64 {
    let ptr = data.as_ptr().add(offset) as *const u64;
    u64::from_le(ptr.read_unaligned())
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset..offset + 8` is within `data`.
pub unsafe fn read_i64_unchecked(data: &[u8], offset: usize) -> i64 {
    let ptr = data.as_ptr().add(offset) as *const i64;
    i64::from_le(ptr.read_unaligned())
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset` is within `data`.
pub unsafe fn read_bool_unchecked(data: &[u8], offset: usize) -> bool {
    *data.get_unchecked(offset) == 1
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset..offset + 32` is within `data`.
pub unsafe fn read_pubkey_unchecked(data: &[u8], offset: usize) -> Pubkey {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::_mm_prefetch;
        use std::arch::x86_64::_MM_HINT_T0;
        if offset + 64 < data.len() {
            _mm_prefetch((data.as_ptr().add(offset + 32)) as *const i8, _MM_HINT_T0);
        }
    }

    let ptr = data.as_ptr().add(offset);
    let mut bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), 32);
    Pubkey::new_from_array(bytes)
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure the 4-byte length prefix is readable and, when present,
/// the following bytes are valid UTF-8.
pub unsafe fn read_str_unchecked(data: &[u8], offset: usize) -> Option<(&str, usize)> {
    if data.len() < offset + 4 {
        return None;
    }

    let len = read_u32_unchecked(data, offset) as usize;
    if data.len() < offset + 4 + len {
        return None;
    }

    let string_bytes = &data[offset + 4..offset + 4 + len];
    let s = std::str::from_utf8_unchecked(string_bytes);
    Some((s, 4 + len))
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset..offset + 4` is within `data`.
pub unsafe fn read_u32_unchecked(data: &[u8], offset: usize) -> u32 {
    let ptr = data.as_ptr().add(offset) as *const u32;
    u32::from_le(ptr.read_unaligned())
}

#[inline(always)]
/// # Safety
///
/// Caller must ensure `offset..offset + 2` is within `data`.
pub unsafe fn read_u16_unchecked(data: &[u8], offset: usize) -> u16 {
    let ptr = data.as_ptr().add(offset) as *const u16;
    u16::from_le(ptr.read_unaligned())
}

const MAX_TRADE_SHAREHOLDERS: usize = 64;
type TradeEventExtensions = (u64, u64, Vec<PumpFeesShareholder>, Pubkey, u64, u64, u64);

#[inline(always)]
unsafe fn read_optional_u64(data: &[u8], offset: &mut usize) -> u64 {
    if *offset + 8 <= data.len() {
        let v = read_u64_unchecked(data, *offset);
        *offset += 8;
        v
    } else {
        0
    }
}

#[inline(always)]
unsafe fn read_optional_pubkey(data: &[u8], offset: &mut usize) -> Pubkey {
    if *offset + 32 <= data.len() {
        let v = read_pubkey_unchecked(data, *offset);
        *offset += 32;
        v
    } else {
        Pubkey::default()
    }
}

#[inline(always)]
unsafe fn read_trade_shareholders(
    data: &[u8],
    offset: &mut usize,
) -> Option<Vec<PumpFeesShareholder>> {
    if *offset + 4 > data.len() {
        return Some(Vec::new());
    }
    let n = read_u32_unchecked(data, *offset) as usize;
    if n > MAX_TRADE_SHAREHOLDERS {
        return None;
    }
    let bytes = 4usize.checked_add(n.checked_mul(34)?)?;
    if *offset + bytes > data.len() {
        return None;
    }
    *offset += 4;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let address = read_pubkey_unchecked(data, *offset);
        *offset += 32;
        let share_bps = read_u16_unchecked(data, *offset);
        *offset += 2;
        out.push(PumpFeesShareholder { address, share_bps });
    }
    Some(out)
}

#[inline(always)]
pub(crate) unsafe fn read_trade_event_extensions(
    data: &[u8],
    offset: &mut usize,
) -> Option<TradeEventExtensions> {
    let buyback_fee_basis_points = read_optional_u64(data, offset);
    let buyback_fee = read_optional_u64(data, offset);
    let shareholders = read_trade_shareholders(data, offset)?;
    let quote_mint = normalize_pumpfun_quote_mint(read_optional_pubkey(data, offset));
    let quote_amount = read_optional_u64(data, offset);
    let virtual_quote_reserves = read_optional_u64(data, offset);
    let real_quote_reserves = read_optional_u64(data, offset);
    Some((
        buyback_fee_basis_points,
        buyback_fee,
        shareholders,
        quote_mint,
        quote_amount,
        virtual_quote_reserves,
        real_quote_reserves,
    ))
}

// --- log_decode ----------------------------------------------------

static BASE64_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: "));
/// `b"Program data: "`.len() — base64 payload starts immediately after this tag.
const PROGRAM_DATA_TAG_LEN: usize = 14;

#[inline(always)]
pub fn extract_program_data_zero_copy<'a>(
    log: &'a str,
    buf: &'a mut [u8; 2048],
) -> Option<&'a [u8]> {
    let log_bytes = log.as_bytes();
    let pos = BASE64_FINDER.find(log_bytes)?;

    let data_part = &log[pos + PROGRAM_DATA_TAG_LEN..];
    let trimmed = data_part.trim();

    if trimmed.len() > 2700 {
        return None;
    }

    use base64_simd::AsOut;
    let decoded_slice =
        base64_simd::STANDARD.decode(trimmed.as_bytes(), buf.as_mut().as_out()).ok()?;

    Some(decoded_slice)
}

#[inline(always)]
pub fn extract_discriminator_simd(log: &str) -> Option<u64> {
    let log_bytes = log.as_bytes();
    let pos = BASE64_FINDER.find(log_bytes)?;

    let data_part = &log[pos + PROGRAM_DATA_TAG_LEN..];
    let trimmed = data_part.trim();

    if trimmed.len() < 16 {
        return None;
    }

    use base64_simd::AsOut;
    let mut buf = [0u8; 12];
    let prefix = trimmed.as_bytes().get(..16)?;
    base64_simd::STANDARD.decode(prefix, buf.as_mut().as_out()).ok()?;

    unsafe {
        let ptr = buf.as_ptr() as *const u64;
        Some(ptr.read_unaligned())
    }
}

// --- main parser ---------------------------------------------------
/// 主解析函数 (极限优化版本)
///
/// 性能目标: <100ns
#[inline(always)]
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    is_created_buy: bool,
) -> Option<DexEvent> {
    #[cfg(feature = "perf-stats")]
    let start = std::time::Instant::now();

    // 使用栈分配的缓冲区 (增加到 2KB 以防止 base64-simd 缓冲区溢出)
    let mut buf = [0u8; 2048];
    let program_data = extract_program_data_zero_copy(log, &mut buf)?;

    if program_data.len() < 8 {
        return None;
    }

    // 使用 unsafe 读取 discriminator (SIMD 优化)
    let discriminator = unsafe { read_u64_unchecked(program_data, 0) };
    let data = &program_data[8..];

    let result = match discriminator {
        CREATE_EVENT => parse_create_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        TRADE_EVENT => parse_trade_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            is_created_buy,
        ),
        MIGRATE_EVENT => parse_migrate_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        CREATE_FEE_SHARING_CONFIG_EVENT => parse_create_fee_sharing_config_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        MIGRATE_BONDING_CURVE_CREATOR_EVENT => parse_migrate_bonding_curve_creator_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        _ => None,
    };

    #[cfg(feature = "perf-stats")]
    {
        PARSE_COUNT.fetch_add(1, Ordering::Relaxed);
        PARSE_TIME_NS.fetch_add(start.elapsed().as_nanos() as usize, Ordering::Relaxed);
    }

    result
}

/// 解析 CreateEvent (极限优化)
///
/// 优化:
/// - 使用 unsafe 消除所有边界检查
/// - 零拷贝字符串解析
/// - 内联所有调用
#[inline(always)]
fn parse_create_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    unsafe {
        let mut offset = 0;

        // 读取字符串字段 (零拷贝)
        let (name, name_len) = read_str_unchecked(data, offset)?;
        offset += name_len;

        let (symbol, symbol_len) = read_str_unchecked(data, offset)?;
        offset += symbol_len;

        let (uri, uri_len) = read_str_unchecked(data, offset)?;
        offset += uri_len;

        // 快速边界检查
        if data.len() < offset + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 1 {
            return None;
        }

        // 读取 Pubkey 字段
        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        // 读取数值字段
        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let token_total_supply = read_u64_unchecked(data, offset);
        offset += 8;

        let token_program = if offset + 32 <= data.len() {
            read_pubkey_unchecked(data, offset)
        } else {
            Pubkey::default()
        };
        offset += 32;

        let is_mayhem_mode =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let is_cashback_enabled =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let quote_mint = normalize_pumpfun_quote_mint(if offset + 32 <= data.len() {
            read_pubkey_unchecked(data, offset)
        } else {
            Pubkey::default()
        });
        offset += 32;
        let virtual_quote_reserves =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        // 将 &str 转换为 String (这是唯一的堆分配)
        // 优化: 可以考虑使用 SmallString 或 Cow<'static, str> 进一步优化
        Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            metadata,
            name: name.to_string(),
            symbol: symbol.to_string(),
            uri: uri.to_string(),
            mint,
            bonding_curve,
            user,
            creator,
            timestamp,
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            token_total_supply,
            token_program,
            is_mayhem_mode,
            is_cashback_enabled,
            quote_mint,
            virtual_quote_reserves,
            ix_name: "create".to_string(),
            ..Default::default()
        }))
    }
}

/// 解析 TradeEvent (极限优化)
///
/// 根据 ix_name 返回不同的事件类型:
/// - "buy" -> DexEvent::PumpFunBuy
/// - "sell" -> DexEvent::PumpFunSell
/// - "buy_exact_sol_in" -> DexEvent::PumpFunBuyExactSolIn
/// - "buy_exact_quote_in" -> DexEvent::PumpFunBuy (exact quote args preserved on fields)
/// - 其他/空 -> DexEvent::PumpFunTrade (兼容旧版本)
#[inline(always)]
fn parse_trade_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    is_created_buy: bool,
) -> Option<DexEvent> {
    unsafe {
        // 快速边界检查
        if data.len() < 32 + 8 + 8 + 1 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 8 + 8 + 32 + 8 + 8 {
            return None;
        }

        let mut offset = 0;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let token_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let is_buy = read_bool_unchecked(data, offset);
        offset += 1;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let fee_recipient = read_pubkey_unchecked(data, offset);
        offset += 32;

        let fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let fee = read_u64_unchecked(data, offset);
        offset += 8;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let creator_fee = read_u64_unchecked(data, offset);
        offset += 8;

        // 可选字段
        let track_volume =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;

        let total_unclaimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let total_claimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let current_sol_volume =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let last_update_timestamp =
            if offset + 8 <= data.len() { read_i64_unchecked(data, offset) } else { 0 };
        offset += 8;

        // ix_name: String (4-byte length prefix + content)
        // Values: "buy" | "sell" | "buy_exact_sol_in" | "buy_exact_quote_in"
        let ix_name = if offset + 4 <= data.len() {
            if let Some((s, len)) = read_str_unchecked(data, offset) {
                offset += len;
                s.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let ix_kind = normalize_pumpfun_ix_name(&ix_name);

        // mayhem_mode: bool (1 byte), cashback_fee_basis_points (8), cashback (8) - PUMP_CASHBACK_README
        let mayhem_mode =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let cashback_fee_basis_points =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let cashback = if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let (
            buyback_fee_basis_points,
            buyback_fee,
            shareholders,
            quote_mint,
            quote_amount,
            virtual_quote_reserves,
            real_quote_reserves,
        ) = read_trade_event_extensions(data, &mut offset)?;

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        let trade_event = PumpFunTradeEvent {
            metadata,
            mint,
            sol_amount,
            token_amount,
            is_buy,
            is_created_buy,
            user,
            timestamp,
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            fee_recipient,
            fee_basis_points,
            fee,
            creator,
            creator_fee_basis_points,
            creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            ix_name: ix_name.clone(),
            mayhem_mode,
            cashback_fee_basis_points,
            cashback,
            buyback_fee_basis_points,
            buyback_fee,
            shareholders,
            quote_mint,
            quote_amount,
            virtual_quote_reserves,
            real_quote_reserves,
            is_cashback_coin: cashback_fee_basis_points > 0,
            amount: 0,
            max_sol_cost: 0,
            min_sol_output: 0,
            spendable_sol_in: 0,
            spendable_quote_in: 0,
            min_tokens_out: 0,
            global: Pubkey::default(),
            bonding_curve: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
            associated_user: Pubkey::default(),
            system_program: Pubkey::default(),
            creator_vault: Pubkey::default(),
            event_authority: Pubkey::default(),
            program: Pubkey::default(),
            global_volume_accumulator: Pubkey::default(),
            user_volume_accumulator: Pubkey::default(),
            fee_config: Pubkey::default(),
            fee_program: Pubkey::default(),
            token_program: Pubkey::default(),
            account: None,
            ..Default::default()
        };

        // 根据 ix_name 返回不同的事件类型，支持用户过滤特定交易类型
        match ix_kind {
            "buy" => Some(DexEvent::PumpFunBuy(trade_event)),
            "sell" => Some(DexEvent::PumpFunSell(trade_event)),
            "buy_exact_sol_in" => Some(DexEvent::PumpFunBuyExactSolIn(trade_event)),
            "buy_exact_quote_in" => Some(DexEvent::PumpFunBuy(trade_event)),
            _ => Some(DexEvent::PumpFunTrade(trade_event)), // 兼容旧版本或未知类型
        }
    }
}

/// 解析 MigrateEvent (极限优化)
#[inline(always)]
fn parse_migrate_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    unsafe {
        // 快速边界检查
        if data.len() < 32 + 32 + 8 + 8 + 8 + 32 + 8 + 32 {
            return None;
        }

        let mut offset = 0;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let pool_migration_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let pool = read_pubkey_unchecked(data, offset);

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
            metadata,
            user,
            mint,
            mint_amount,
            sol_amount,
            pool_migration_fee,
            bonding_curve,
            timestamp,
            pool,
        }))
    }
}

#[inline(always)]
fn parse_migrate_bonding_curve_creator_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    };
    parse_migrate_bonding_curve_creator_from_data(data, metadata)
}

#[inline(always)]
fn parse_create_fee_sharing_config_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    };
    crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(data, metadata)
}

// ============================================================================
// 快速过滤 API (用于事件过滤场景)
// ============================================================================

/// 快速判断事件类型 (只解析 discriminator)
///
/// 性能: <50ns
#[inline(always)]
pub fn get_event_type_fast(log: &str) -> Option<u64> {
    extract_discriminator_simd(log)
}

/// 检查是否为特定事件类型 (SIMD 优化)
#[inline(always)]
pub fn is_event_type(log: &str, discriminator: u64) -> bool {
    extract_discriminator_simd(log) == Some(discriminator)
}

// ============================================================================
// Public API for optimized parsing from pre-decoded data
// These functions accept already-decoded data (without discriminator)
// ============================================================================

/// Parse PumpFun Trade event from pre-decoded data
///
/// `data` should be the decoded bytes AFTER the 8-byte discriminator
///
/// Returns different event types based on ix_name:
/// - "buy" -> DexEvent::PumpFunBuy
/// - "sell" -> DexEvent::PumpFunSell
/// - "buy_exact_sol_in" -> DexEvent::PumpFunBuyExactSolIn
/// - "buy_exact_quote_in" -> DexEvent::PumpFunBuy (exact quote args preserved on fields)
/// - other/empty -> DexEvent::PumpFunTrade (backward compatible)
#[inline(always)]
pub fn parse_trade_from_data(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    unsafe {
        // 快速边界检查
        if data.len() < 32 + 8 + 8 + 1 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 8 + 8 + 32 + 8 + 8 {
            return None;
        }

        let mut offset = 0;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let token_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let is_buy = read_bool_unchecked(data, offset);
        offset += 1;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let fee_recipient = read_pubkey_unchecked(data, offset);
        offset += 32;

        let fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let fee = read_u64_unchecked(data, offset);
        offset += 8;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let creator_fee = read_u64_unchecked(data, offset);
        offset += 8;

        // 可选字段
        let track_volume =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;

        let total_unclaimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let total_claimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let current_sol_volume =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let last_update_timestamp =
            if offset + 8 <= data.len() { read_i64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let ix_name = if offset + 4 <= data.len() {
            if let Some((s, len)) = read_str_unchecked(data, offset) {
                offset += len;
                s.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let ix_kind = normalize_pumpfun_ix_name(&ix_name);

        // mayhem_mode (1), cashback_fee_basis_points (8), cashback (8) - PUMP_CASHBACK_README
        let mayhem_mode =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let cashback_fee_basis_points =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let cashback = if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let (
            buyback_fee_basis_points,
            buyback_fee,
            shareholders,
            quote_mint,
            quote_amount,
            virtual_quote_reserves,
            real_quote_reserves,
        ) = read_trade_event_extensions(data, &mut offset)?;

        let trade_event = PumpFunTradeEvent {
            metadata,
            mint,
            sol_amount,
            token_amount,
            is_buy,
            is_created_buy,
            user,
            timestamp,
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            fee_recipient,
            fee_basis_points,
            fee,
            creator,
            creator_fee_basis_points,
            creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            ix_name: ix_name.clone(),
            mayhem_mode,
            cashback_fee_basis_points,
            cashback,
            buyback_fee_basis_points,
            buyback_fee,
            shareholders,
            quote_mint,
            quote_amount,
            virtual_quote_reserves,
            real_quote_reserves,
            is_cashback_coin: cashback_fee_basis_points > 0,
            amount: 0,
            max_sol_cost: 0,
            min_sol_output: 0,
            spendable_sol_in: 0,
            spendable_quote_in: 0,
            min_tokens_out: 0,
            global: Pubkey::default(),
            bonding_curve: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
            associated_user: Pubkey::default(),
            system_program: Pubkey::default(),
            creator_vault: Pubkey::default(),
            event_authority: Pubkey::default(),
            program: Pubkey::default(),
            global_volume_accumulator: Pubkey::default(),
            user_volume_accumulator: Pubkey::default(),
            fee_config: Pubkey::default(),
            fee_program: Pubkey::default(),
            token_program: Pubkey::default(),
            account: None,
            ..Default::default()
        };

        // 根据 ix_name 返回不同的事件类型
        match ix_kind {
            "buy" => Some(DexEvent::PumpFunBuy(trade_event)),
            "sell" => Some(DexEvent::PumpFunSell(trade_event)),
            "buy_exact_sol_in" => Some(DexEvent::PumpFunBuyExactSolIn(trade_event)),
            "buy_exact_quote_in" => Some(DexEvent::PumpFunBuy(trade_event)),
            _ => Some(DexEvent::PumpFunTrade(trade_event)),
        }
    }
}

/// Parse only PumpFun Buy events from pre-decoded data
///
/// Returns None if the event is not a buy event
#[inline(always)]
pub fn parse_buy_from_data(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    let event = parse_trade_from_data(data, metadata, is_created_buy)?;
    match &event {
        DexEvent::PumpFunBuy(_) => Some(event),
        _ => None,
    }
}

/// Parse only PumpFun Sell events from pre-decoded data
///
/// Returns None if the event is not a sell event
#[inline(always)]
pub fn parse_sell_from_data(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    let event = parse_trade_from_data(data, metadata, is_created_buy)?;
    match &event {
        DexEvent::PumpFunSell(_) => Some(event),
        _ => None,
    }
}

/// Parse only PumpFun BuyExactSolIn events from pre-decoded data
///
/// Returns None if the event is not a buy_exact_sol_in event
#[inline(always)]
pub fn parse_buy_exact_sol_in_from_data(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    let event = parse_trade_from_data(data, metadata, is_created_buy)?;
    match &event {
        DexEvent::PumpFunBuyExactSolIn(_) => Some(event),
        _ => None,
    }
}

/// Parse PumpFun Create event from pre-decoded data
#[inline(always)]
pub fn parse_create_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        let mut offset = 0;

        let (name, name_len) = read_str_unchecked(data, offset)?;
        offset += name_len;

        let (symbol, symbol_len) = read_str_unchecked(data, offset)?;
        offset += symbol_len;

        let (uri, uri_len) = read_str_unchecked(data, offset)?;
        offset += uri_len;

        if data.len() < offset + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 1 {
            return None;
        }

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let token_total_supply = read_u64_unchecked(data, offset);
        offset += 8;

        let token_program = if offset + 32 <= data.len() {
            read_pubkey_unchecked(data, offset)
        } else {
            Pubkey::default()
        };
        offset += 32;

        let is_mayhem_mode =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let is_cashback_enabled =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let quote_mint = normalize_pumpfun_quote_mint(if offset + 32 <= data.len() {
            read_pubkey_unchecked(data, offset)
        } else {
            Pubkey::default()
        });
        offset += 32;
        let virtual_quote_reserves =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };

        Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            metadata,
            name: name.to_string(),
            symbol: symbol.to_string(),
            uri: uri.to_string(),
            mint,
            bonding_curve,
            user,
            creator,
            timestamp,
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            token_total_supply,
            token_program,
            is_mayhem_mode,
            is_cashback_enabled,
            quote_mint,
            virtual_quote_reserves,
            ix_name: "create".to_string(),
            ..Default::default()
        }))
    }
}

/// Parse PumpFun Migrate event from pre-decoded data
#[inline(always)]
pub fn parse_migrate_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if data.len() < 32 + 32 + 8 + 8 + 8 + 32 + 8 + 32 {
            return None;
        }

        let mut offset = 0;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let pool_migration_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let pool = read_pubkey_unchecked(data, offset);

        Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
            metadata,
            user,
            mint,
            mint_amount,
            sol_amount,
            pool_migration_fee,
            bonding_curve,
            timestamp,
            pool,
        }))
    }
}

/// `migrateBondingCurveCreatorEvent`：`data` 为去掉 8 字节 discriminator 之后的 Borsh 体。
#[inline(always)]
pub fn parse_migrate_bonding_curve_creator_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    unsafe {
        const NEED: usize = 8 + 32 * 5;
        if data.len() < NEED {
            return None;
        }

        let mut offset = 0usize;
        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;
        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;
        let sharing_config = read_pubkey_unchecked(data, offset);
        offset += 32;
        let old_creator = read_pubkey_unchecked(data, offset);
        offset += 32;
        let new_creator = read_pubkey_unchecked(data, offset);

        Some(DexEvent::PumpFunMigrateBondingCurveCreator(PumpFunMigrateBondingCurveCreatorEvent {
            metadata,
            timestamp,
            mint,
            bonding_curve,
            sharing_config,
            old_creator,
            new_creator,
        }))
    }
}

/// `createFeeSharingConfigEvent`：委托 [`pump_fees::parse_create_fee_sharing_config_from_data`](crate::logs::pump_fees)。
#[inline]
pub fn parse_create_fee_sharing_config_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(data, metadata)
}

#[inline(always)]
fn read_i64_at(data: &[u8], o: &mut usize) -> Option<i64> {
    if data.len() < *o + 8 {
        return None;
    }
    let v = i64::from_le_bytes(data[*o..*o + 8].try_into().ok()?);
    *o += 8;
    Some(v)
}

#[inline(always)]
fn read_u16_at(data: &[u8], o: &mut usize) -> Option<u16> {
    if data.len() < *o + 2 {
        return None;
    }
    let v = u16::from_le_bytes(data[*o..*o + 2].try_into().ok()?);
    *o += 2;
    Some(v)
}

#[inline(always)]
fn read_u32_at(data: &[u8], o: &mut usize) -> Option<u32> {
    if data.len() < *o + 4 {
        return None;
    }
    let v = u32::from_le_bytes(data[*o..*o + 4].try_into().ok()?);
    *o += 4;
    Some(v)
}

#[inline(always)]
fn read_pubkey_at(data: &[u8], o: &mut usize) -> Option<Pubkey> {
    if data.len() < *o + 32 {
        return None;
    }
    let pk = Pubkey::new_from_array(data[*o..*o + 32].try_into().ok()?);
    *o += 32;
    Some(pk)
}

// ============================================================================
// 性能统计 API (可选)
// ============================================================================

#[cfg(feature = "perf-stats")]
pub fn get_perf_stats() -> (usize, usize) {
    let count = PARSE_COUNT.load(Ordering::Relaxed);
    let total_ns = PARSE_TIME_NS.load(Ordering::Relaxed);
    (count, total_ns)
}

#[cfg(feature = "perf-stats")]
pub fn reset_perf_stats() {
    PARSE_COUNT.store(0, Ordering::Relaxed);
    PARSE_TIME_NS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{DexEvent, EventMetadata};

    #[test]
    fn test_discriminator_simd() {
        // 测试 SIMD discriminator 提取
        let log = "Program data: G3Kp5Dfe605nAAAAAAAAAAA=";
        let disc = extract_discriminator_simd(log);
        assert!(disc.is_some());
    }

    #[test]
    fn test_parse_performance() {
        // 性能测试
        let log = "Program data: G3Kp5Dfe605nAAAAAAAAAAA=";
        let sig = Signature::default();

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = parse_log(log, sig, 0, 0, Some(0), 0, false);
        }
        let elapsed = start.elapsed();

        println!("Average parse time: {} ns", elapsed.as_nanos() / 1000);
    }

    #[test]
    fn migrate_bonding_curve_creator_roundtrip_from_data() {
        let ts: i64 = 1_777_920_719;
        let mint = Pubkey::new_unique();
        let bonding_curve = Pubkey::new_unique();
        let sharing_config = Pubkey::new_unique();
        let old_creator = Pubkey::new_unique();
        let new_creator = Pubkey::new_unique();

        let mut buf = Vec::with_capacity(200);
        buf.extend_from_slice(&ts.to_le_bytes());
        buf.extend_from_slice(mint.as_ref());
        buf.extend_from_slice(bonding_curve.as_ref());
        buf.extend_from_slice(sharing_config.as_ref());
        buf.extend_from_slice(old_creator.as_ref());
        buf.extend_from_slice(new_creator.as_ref());

        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 0,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };

        let ev = parse_migrate_bonding_curve_creator_from_data(&buf, metadata).expect("parse");
        match ev {
            DexEvent::PumpFunMigrateBondingCurveCreator(e) => {
                assert_eq!(e.timestamp, ts);
                assert_eq!(e.mint, mint);
                assert_eq!(e.bonding_curve, bonding_curve);
                assert_eq!(e.sharing_config, sharing_config);
                assert_eq!(e.old_creator, old_creator);
                assert_eq!(e.new_creator, new_creator);
            }
            _ => panic!("wrong variant"),
        }
    }
}
