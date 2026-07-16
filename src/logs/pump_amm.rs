//! PumpSwap (Pump AMM) 极限优化解析器 - 纳秒/微秒级性能
//!
//! 优化策略:
//! - 零拷贝解析 (zero-copy)
//! - 栈分配替代堆分配
//! - unsafe 消除边界检查
//! - 编译器自动向量化 (target-cpu=native)
//! - 内联所有热路径
//! - 编译时计算
//! - 预计算查找表
//! - L1 cache 优化 (1KB 栈缓冲区)

use crate::core::events::*;
use memchr::memmem;
use once_cell::sync::Lazy;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[cfg(feature = "perf-stats")]
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// 性能计数器 (可选，用于性能分析)
// ============================================================================

#[cfg(feature = "perf-stats")]
pub static PARSE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "perf-stats")]
pub static PARSE_TIME_NS: AtomicUsize = AtomicUsize::new(0);

// ============================================================================
// 编译时常量和查找表
// ============================================================================

/// PumpSwap discriminator constants (compile-time computed)
pub mod discriminators {
    // Use u64 direct comparison to avoid array comparison
    // Event discriminators from pump_amm.json
    pub const BUY: u64 = u64::from_le_bytes([103, 244, 82, 31, 44, 245, 119, 119]); // BuyEvent
    pub const SELL: u64 = u64::from_le_bytes([62, 47, 55, 10, 165, 3, 220, 42]); // SellEvent
    pub const CREATE_POOL: u64 = u64::from_le_bytes([177, 49, 12, 210, 160, 118, 167, 116]); // CreatePoolEvent
    pub const ADD_LIQUIDITY: u64 = u64::from_le_bytes([120, 248, 61, 83, 31, 142, 107, 144]); // DepositEvent
    pub const REMOVE_LIQUIDITY: u64 = u64::from_le_bytes([22, 9, 133, 26, 160, 44, 71, 192]);
    // WithdrawEvent
}

/// Base64 查找器预计算 (用于快速定位)
static BASE64_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: "));

/// 跳过 ASCII 空白后拷贝 base64 前缀（Explorer / 部分日志会在 base64 中插空格）
#[inline(always)]
fn copy_b64_skip_ws_prefix(src: &[u8], out: &mut [u8], max_copy: usize) -> Option<usize> {
    let cap = max_copy.min(out.len());
    let mut j = 0usize;
    for &b in src {
        if b.is_ascii_whitespace() {
            continue;
        }
        if j >= cap {
            break;
        }
        out[j] = b;
        j += 1;
    }
    if j < 12 {
        return None;
    }
    Some(j)
}

// ============================================================================
// 零拷贝解析核心 - 使用栈分配
// ============================================================================

/// 零拷贝提取 program data (栈分配，无堆分配)
///
/// 优化: 使用固定大小栈缓冲区，避免 Vec 分配
/// 缓冲区大小增加到 2KB 以防止 base64-simd 缓冲区溢出panic
#[inline(always)]
fn extract_program_data_zero_copy<'a>(log: &'a str, buf: &'a mut [u8; 2048]) -> Option<&'a [u8]> {
    let log_bytes = log.as_bytes();
    let pos = BASE64_FINDER.find(log_bytes)?;

    let data_part = &log[pos + 14..];
    let trimmed = data_part.trim();
    let body = trimmed.as_bytes();

    if body.len() > 2700 {
        return None;
    }

    use base64_simd::AsOut;
    const COMPACT_CAP: usize = 2730;
    let decoded_slice = if body.iter().any(|&b| b.is_ascii_whitespace()) {
        let mut compact = [0u8; COMPACT_CAP];
        let n = copy_b64_skip_ws_prefix(body, &mut compact, COMPACT_CAP)?;
        base64_simd::STANDARD.decode(&compact[..n], buf.as_mut().as_out()).ok()?
    } else {
        base64_simd::STANDARD.decode(body, buf.as_mut().as_out()).ok()?
    };

    Some(decoded_slice)
}

/// 快速 discriminator 提取 (SIMD 优化)
#[inline(always)]
fn extract_discriminator_simd(log: &str) -> Option<u64> {
    let log_bytes = log.as_bytes();
    let pos = BASE64_FINDER.find(log_bytes)?;

    let data_part = &log[pos + 14..];
    let body = data_part.trim().as_bytes();

    use base64_simd::AsOut;
    let mut compact = [0u8; 24];
    let n = copy_b64_skip_ws_prefix(body, &mut compact, 16)?;
    let mut dec = [0u8; 12];
    base64_simd::STANDARD.decode(&compact[..n], dec.as_mut().as_out()).ok()?;

    unsafe {
        let ptr = dec.as_ptr() as *const u64;
        Some(ptr.read_unaligned())
    }
}

// ============================================================================
// Unsafe 读取函数 - 消除边界检查
// ============================================================================

/// 读取 u64 (unsafe, 无边界检查)
#[inline(always)]
unsafe fn read_u64_unchecked(data: &[u8], offset: usize) -> u64 {
    let ptr = data.as_ptr().add(offset) as *const u64;
    u64::from_le(ptr.read_unaligned())
}

/// 读取 i64 (unsafe, 无边界检查)
#[inline(always)]
unsafe fn read_i64_unchecked(data: &[u8], offset: usize) -> i64 {
    let ptr = data.as_ptr().add(offset) as *const i64;
    i64::from_le(ptr.read_unaligned())
}

/// Read u16 (unsafe, no bounds check)
#[inline(always)]
unsafe fn read_u16_unchecked(data: &[u8], offset: usize) -> u16 {
    let ptr = data.as_ptr().add(offset) as *const u16;
    u16::from_le(ptr.read_unaligned())
}

/// Read u32 (unsafe, no bounds check)
#[allow(dead_code)]
#[inline(always)]
unsafe fn read_u32_unchecked(data: &[u8], offset: usize) -> u32 {
    let ptr = data.as_ptr().add(offset) as *const u32;
    u32::from_le(ptr.read_unaligned())
}

/// Read u8 (unsafe, no bounds check)
#[inline(always)]
unsafe fn read_u8_unchecked(data: &[u8], offset: usize) -> u8 {
    *data.get_unchecked(offset)
}

/// 读取 bool (unsafe, 无边界检查)
#[inline(always)]
unsafe fn read_bool_unchecked(data: &[u8], offset: usize) -> bool {
    *data.get_unchecked(offset) == 1
}

/// 读取 Pubkey (unsafe, 无边界检查)
///
/// 优化: 添加内存预取，假设连续读取多个 Pubkey
#[inline(always)]
unsafe fn read_pubkey_unchecked(data: &[u8], offset: usize) -> Pubkey {
    // 预取下一个可能的 Pubkey 位置 (假设连续读取)
    // 使用 T0 提示 (最高优先级) 将数据预取到 L1 cache
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

// ============================================================================
// Optimized event parsing functions
// ============================================================================

/// Main parse function (optimized)
///
/// Performance target: <100ns
#[inline(always)]
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    #[cfg(feature = "perf-stats")]
    let start = std::time::Instant::now();

    // Stack-allocated buffer (增加到 2KB 以防止 base64-simd 缓冲区溢出)
    let mut buf = [0u8; 2048];
    let program_data = extract_program_data_zero_copy(log, &mut buf)?;

    if program_data.len() < 8 {
        return None;
    }

    // Read discriminator using unsafe (SIMD optimized)
    let discriminator = unsafe { read_u64_unchecked(program_data, 0) };
    let data = &program_data[8..];

    let result = match discriminator {
        discriminators::BUY => {
            parse_buy_event_optimized(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::SELL => {
            parse_sell_event_optimized(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CREATE_POOL => parse_create_pool_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::ADD_LIQUIDITY => parse_add_liquidity_event_optimized(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity_event_optimized(
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

/// Parse buy event (optimized) - BuyEvent from pump_amm.json
///
/// Optimizations:
/// - Use unsafe to eliminate all bounds checks
/// - Batch bounds check instead of per-field check
/// - Inline all calls
#[inline(always)]
fn parse_buy_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // Minimum size through min_base_amount_out plus an empty ix_name string prefix.
    const MIN_REQUIRED_LEN: usize = 16 * 8 + 7 * 32 + 1 + 5 * 8 + 4;
    if data.len() < MIN_REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let base_amount_out = read_u64_unchecked(data, 8);
        let max_quote_amount_in = read_u64_unchecked(data, 16);
        let user_base_token_reserves = read_u64_unchecked(data, 24);
        let user_quote_token_reserves = read_u64_unchecked(data, 32);
        let pool_base_token_reserves = read_u64_unchecked(data, 40);
        let pool_quote_token_reserves = read_u64_unchecked(data, 48);
        let quote_amount_in = read_u64_unchecked(data, 56);
        let lp_fee_basis_points = read_u64_unchecked(data, 64);
        let lp_fee = read_u64_unchecked(data, 72);
        let protocol_fee_basis_points = read_u64_unchecked(data, 80);
        let protocol_fee = read_u64_unchecked(data, 88);
        let quote_amount_in_with_lp_fee = read_u64_unchecked(data, 96);
        let user_quote_amount_in = read_u64_unchecked(data, 104);

        let pool = read_pubkey_unchecked(data, 112);
        let user = read_pubkey_unchecked(data, 144);
        let user_base_token_account = read_pubkey_unchecked(data, 176);
        let user_quote_token_account = read_pubkey_unchecked(data, 208);
        let protocol_fee_recipient = read_pubkey_unchecked(data, 240);
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, 272);
        let coin_creator = read_pubkey_unchecked(data, 304);

        let coin_creator_fee_basis_points = read_u64_unchecked(data, 336);
        let coin_creator_fee = read_u64_unchecked(data, 344);
        let track_volume = read_bool_unchecked(data, 352);
        let total_unclaimed_tokens = read_u64_unchecked(data, 353);
        let total_claimed_tokens = read_u64_unchecked(data, 361);
        let current_sol_volume = read_u64_unchecked(data, 369);
        let last_update_timestamp = read_i64_unchecked(data, 377);

        // New fields from IDL update
        let mut offset = 385;
        let min_base_amount_out = read_u64_unchecked(data, offset);
        offset += 8;

        // ix_name: String (4-byte length prefix + content)
        let ix_name = if offset + 4 <= data.len() {
            let len = read_u32_unchecked(data, offset) as usize;
            offset += 4;
            if offset + len <= data.len() {
                let string_bytes = &data[offset..offset + len];
                let s = std::str::from_utf8_unchecked(string_bytes);
                offset += len;
                s.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // BuyEvent 新增字段 (PUMP_CASHBACK_README): cashback_fee_basis_points, cashback
        let cashback_fee_basis_points =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let cashback = if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

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

/// 解析卖出事件 (极限优化)
#[inline(always)]
fn parse_sell_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // 一次性边界检查 (13个u64 + 1个i64 + 7个Pubkey + 2个u64 cashback 字段)
    const REQUIRED_LEN: usize = 13 * 8 + 8 + 7 * 32 + 8 + 8;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let base_amount_in = read_u64_unchecked(data, 8);
        let min_quote_amount_out = read_u64_unchecked(data, 16);
        let user_base_token_reserves = read_u64_unchecked(data, 24);
        let user_quote_token_reserves = read_u64_unchecked(data, 32);
        let pool_base_token_reserves = read_u64_unchecked(data, 40);
        let pool_quote_token_reserves = read_u64_unchecked(data, 48);
        let quote_amount_out = read_u64_unchecked(data, 56);
        let lp_fee_basis_points = read_u64_unchecked(data, 64);
        let lp_fee = read_u64_unchecked(data, 72);
        let protocol_fee_basis_points = read_u64_unchecked(data, 80);
        let protocol_fee = read_u64_unchecked(data, 88);
        let quote_amount_out_without_lp_fee = read_u64_unchecked(data, 96);
        let user_quote_amount_out = read_u64_unchecked(data, 104);

        let pool = read_pubkey_unchecked(data, 112);
        let user = read_pubkey_unchecked(data, 144);
        let user_base_token_account = read_pubkey_unchecked(data, 176);
        let user_quote_token_account = read_pubkey_unchecked(data, 208);
        let protocol_fee_recipient = read_pubkey_unchecked(data, 240);
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, 272);
        let coin_creator = read_pubkey_unchecked(data, 304);

        let coin_creator_fee_basis_points = read_u64_unchecked(data, 336);
        let coin_creator_fee = read_u64_unchecked(data, 344);
        // SellEvent 新增字段 (PUMP_CASHBACK_README): cashback_fee_basis_points, cashback
        let cashback_fee_basis_points = read_u64_unchecked(data, 352);
        let cashback = read_u64_unchecked(data, 360);

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

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
            ..Default::default()
        }))
    }
}

/// 解析池创建事件 (极限优化)
#[inline(always)]
fn parse_create_pool_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // 一次性边界检查 (含 IDL 最后一列 is_mayhem_mode: bool)
    const CREATE_POOL_EVENT_LEN: usize = 326;
    const REQUIRED_LEN: usize = CREATE_POOL_EVENT_LEN;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let index = read_u16_unchecked(data, 8);

        let creator = read_pubkey_unchecked(data, 10);
        let base_mint = read_pubkey_unchecked(data, 42);
        let quote_mint = read_pubkey_unchecked(data, 74);

        let base_mint_decimals = read_u8_unchecked(data, 106);
        let quote_mint_decimals = read_u8_unchecked(data, 107);

        let base_amount_in = read_u64_unchecked(data, 108);
        let quote_amount_in = read_u64_unchecked(data, 116);
        let pool_base_amount = read_u64_unchecked(data, 124);
        let pool_quote_amount = read_u64_unchecked(data, 132);
        let minimum_liquidity = read_u64_unchecked(data, 140);
        let initial_liquidity = read_u64_unchecked(data, 148);
        let lp_token_amount_out = read_u64_unchecked(data, 156);

        let pool_bump = read_u8_unchecked(data, 164);

        let pool = read_pubkey_unchecked(data, 165);
        let lp_mint = read_pubkey_unchecked(data, 197);
        let user_base_token_account = read_pubkey_unchecked(data, 229);
        let user_quote_token_account = read_pubkey_unchecked(data, 261);
        let coin_creator = read_pubkey_unchecked(data, 293);
        let is_mayhem_mode = read_bool_unchecked(data, 325);

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
            metadata,
            timestamp,
            index,
            creator,
            base_mint,
            quote_mint,
            base_mint_decimals,
            quote_mint_decimals,
            base_amount_in,
            quote_amount_in,
            pool_base_amount,
            pool_quote_amount,
            minimum_liquidity,
            initial_liquidity,
            lp_token_amount_out,
            pool_bump,
            pool,
            lp_mint,
            user_base_token_account,
            user_quote_token_account,
            coin_creator,
            is_mayhem_mode,
            is_cashback_coin: false,
        }))
    }
}

/// 解析添加流动性事件 (极限优化)
#[inline(always)]
fn parse_add_liquidity_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    const REQUIRED_LEN: usize = 10 * 8 + 5 * 32;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let lp_token_amount_out = read_u64_unchecked(data, 8);
        let max_base_amount_in = read_u64_unchecked(data, 16);
        let max_quote_amount_in = read_u64_unchecked(data, 24);
        let user_base_token_reserves = read_u64_unchecked(data, 32);
        let user_quote_token_reserves = read_u64_unchecked(data, 40);
        let pool_base_token_reserves = read_u64_unchecked(data, 48);
        let pool_quote_token_reserves = read_u64_unchecked(data, 56);
        let base_amount_in = read_u64_unchecked(data, 64);
        let quote_amount_in = read_u64_unchecked(data, 72);
        let lp_mint_supply = read_u64_unchecked(data, 80);

        let pool = read_pubkey_unchecked(data, 88);
        let user = read_pubkey_unchecked(data, 120);
        let user_base_token_account = read_pubkey_unchecked(data, 152);
        let user_quote_token_account = read_pubkey_unchecked(data, 184);
        let user_pool_token_account = read_pubkey_unchecked(data, 216);

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        Some(DexEvent::PumpSwapLiquidityAdded(PumpSwapLiquidityAdded {
            metadata,
            timestamp,
            lp_token_amount_out,
            max_base_amount_in,
            max_quote_amount_in,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            base_amount_in,
            quote_amount_in,
            lp_mint_supply,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            user_pool_token_account,
        }))
    }
}

/// 解析移除流动性事件 (极限优化)
#[inline(always)]
fn parse_remove_liquidity_event_optimized(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    const REQUIRED_LEN: usize = 10 * 8 + 5 * 32;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let lp_token_amount_in = read_u64_unchecked(data, 8);
        let min_base_amount_out = read_u64_unchecked(data, 16);
        let min_quote_amount_out = read_u64_unchecked(data, 24);
        let user_base_token_reserves = read_u64_unchecked(data, 32);
        let user_quote_token_reserves = read_u64_unchecked(data, 40);
        let pool_base_token_reserves = read_u64_unchecked(data, 48);
        let pool_quote_token_reserves = read_u64_unchecked(data, 56);
        let base_amount_out = read_u64_unchecked(data, 64);
        let quote_amount_out = read_u64_unchecked(data, 72);
        let lp_mint_supply = read_u64_unchecked(data, 80);

        let pool = read_pubkey_unchecked(data, 88);
        let user = read_pubkey_unchecked(data, 120);
        let user_base_token_account = read_pubkey_unchecked(data, 152);
        let user_quote_token_account = read_pubkey_unchecked(data, 184);
        let user_pool_token_account = read_pubkey_unchecked(data, 216);

        let metadata = EventMetadata {
            signature,
            slot,
            tx_index,
            block_time_us: block_time_us.unwrap_or(0),
            grpc_recv_us,
            recent_blockhash: None,
        };

        Some(DexEvent::PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved {
            metadata,
            timestamp,
            lp_token_amount_in,
            min_base_amount_out,
            min_quote_amount_out,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            base_amount_out,
            quote_amount_out,
            lp_mint_supply,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            user_pool_token_account,
        }))
    }
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

/// Parse PumpSwap Buy event from pre-decoded data
#[inline(always)]
pub fn parse_buy_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // Minimum size through min_base_amount_out plus an empty ix_name string prefix.
    const MIN_REQUIRED_LEN: usize = 16 * 8 + 7 * 32 + 1 + 5 * 8 + 4;
    if data.len() < MIN_REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let base_amount_out = read_u64_unchecked(data, 8);
        let max_quote_amount_in = read_u64_unchecked(data, 16);
        let user_base_token_reserves = read_u64_unchecked(data, 24);
        let user_quote_token_reserves = read_u64_unchecked(data, 32);
        let pool_base_token_reserves = read_u64_unchecked(data, 40);
        let pool_quote_token_reserves = read_u64_unchecked(data, 48);
        let quote_amount_in = read_u64_unchecked(data, 56);
        let lp_fee_basis_points = read_u64_unchecked(data, 64);
        let lp_fee = read_u64_unchecked(data, 72);
        let protocol_fee_basis_points = read_u64_unchecked(data, 80);
        let protocol_fee = read_u64_unchecked(data, 88);
        let quote_amount_in_with_lp_fee = read_u64_unchecked(data, 96);
        let user_quote_amount_in = read_u64_unchecked(data, 104);

        let pool = read_pubkey_unchecked(data, 112);
        let user = read_pubkey_unchecked(data, 144);
        let user_base_token_account = read_pubkey_unchecked(data, 176);
        let user_quote_token_account = read_pubkey_unchecked(data, 208);
        let protocol_fee_recipient = read_pubkey_unchecked(data, 240);
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, 272);
        let coin_creator = read_pubkey_unchecked(data, 304);

        let coin_creator_fee_basis_points = read_u64_unchecked(data, 336);
        let coin_creator_fee = read_u64_unchecked(data, 344);
        let track_volume = read_bool_unchecked(data, 352);
        let total_unclaimed_tokens = read_u64_unchecked(data, 353);
        let total_claimed_tokens = read_u64_unchecked(data, 361);
        let current_sol_volume = read_u64_unchecked(data, 369);
        let last_update_timestamp = read_i64_unchecked(data, 377);

        // New fields from IDL update
        let mut offset = 385;
        let min_base_amount_out = read_u64_unchecked(data, offset);
        offset += 8;

        // ix_name: String (4-byte length prefix + content)
        let ix_name = if offset + 4 <= data.len() {
            let len = read_u32_unchecked(data, offset) as usize;
            offset += 4;
            if offset + len <= data.len() {
                let string_bytes = &data[offset..offset + len];
                let s = std::str::from_utf8_unchecked(string_bytes);
                s.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

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
            ..Default::default()
        }))
    }
}

/// Parse PumpSwap Sell event from pre-decoded data
#[inline(always)]
pub fn parse_sell_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const REQUIRED_LEN: usize = 13 * 8 + 8 + 7 * 32;
    const CASHBACK_FEE_BASIS_POINTS_OFFSET: usize = 352;
    const CASHBACK_OFFSET: usize = 360;
    const CASHBACK_FIELDS_LEN: usize = 16;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let base_amount_in = read_u64_unchecked(data, 8);
        let min_quote_amount_out = read_u64_unchecked(data, 16);
        let user_base_token_reserves = read_u64_unchecked(data, 24);
        let user_quote_token_reserves = read_u64_unchecked(data, 32);
        let pool_base_token_reserves = read_u64_unchecked(data, 40);
        let pool_quote_token_reserves = read_u64_unchecked(data, 48);
        let quote_amount_out = read_u64_unchecked(data, 56);
        let lp_fee_basis_points = read_u64_unchecked(data, 64);
        let lp_fee = read_u64_unchecked(data, 72);
        let protocol_fee_basis_points = read_u64_unchecked(data, 80);
        let protocol_fee = read_u64_unchecked(data, 88);
        let quote_amount_out_without_lp_fee = read_u64_unchecked(data, 96);
        let user_quote_amount_out = read_u64_unchecked(data, 104);

        let pool = read_pubkey_unchecked(data, 112);
        let user = read_pubkey_unchecked(data, 144);
        let user_base_token_account = read_pubkey_unchecked(data, 176);
        let user_quote_token_account = read_pubkey_unchecked(data, 208);
        let protocol_fee_recipient = read_pubkey_unchecked(data, 240);
        let protocol_fee_recipient_token_account = read_pubkey_unchecked(data, 272);
        let coin_creator = read_pubkey_unchecked(data, 304);

        let coin_creator_fee_basis_points = read_u64_unchecked(data, 336);
        let coin_creator_fee = read_u64_unchecked(data, 344);
        let (cashback_fee_basis_points, cashback) =
            if data.len() >= CASHBACK_FEE_BASIS_POINTS_OFFSET + CASHBACK_FIELDS_LEN {
                (
                    read_u64_unchecked(data, CASHBACK_FEE_BASIS_POINTS_OFFSET),
                    read_u64_unchecked(data, CASHBACK_OFFSET),
                )
            } else {
                (0, 0)
            };

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
            ..Default::default()
        }))
    }
}

/// Parse PumpSwap CreatePool event from pre-decoded data
#[inline(always)]
pub fn parse_create_pool_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const CREATE_POOL_EVENT_LEN: usize = 326;
    const REQUIRED_LEN: usize = CREATE_POOL_EVENT_LEN;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let index = read_u16_unchecked(data, 8);

        let creator = read_pubkey_unchecked(data, 10);
        let base_mint = read_pubkey_unchecked(data, 42);
        let quote_mint = read_pubkey_unchecked(data, 74);

        let base_mint_decimals = read_u8_unchecked(data, 106);
        let quote_mint_decimals = read_u8_unchecked(data, 107);

        let base_amount_in = read_u64_unchecked(data, 108);
        let quote_amount_in = read_u64_unchecked(data, 116);
        let pool_base_amount = read_u64_unchecked(data, 124);
        let pool_quote_amount = read_u64_unchecked(data, 132);
        let minimum_liquidity = read_u64_unchecked(data, 140);
        let initial_liquidity = read_u64_unchecked(data, 148);
        let lp_token_amount_out = read_u64_unchecked(data, 156);

        let pool_bump = read_u8_unchecked(data, 164);

        let pool = read_pubkey_unchecked(data, 165);
        let lp_mint = read_pubkey_unchecked(data, 197);
        let user_base_token_account = read_pubkey_unchecked(data, 229);
        let user_quote_token_account = read_pubkey_unchecked(data, 261);
        let coin_creator = read_pubkey_unchecked(data, 293);
        let is_mayhem_mode = data.len() > 325 && read_bool_unchecked(data, 325);

        Some(DexEvent::PumpSwapCreatePool(PumpSwapCreatePoolEvent {
            metadata,
            timestamp,
            index,
            creator,
            base_mint,
            quote_mint,
            base_mint_decimals,
            quote_mint_decimals,
            base_amount_in,
            quote_amount_in,
            pool_base_amount,
            pool_quote_amount,
            minimum_liquidity,
            initial_liquidity,
            lp_token_amount_out,
            pool_bump,
            pool,
            lp_mint,
            user_base_token_account,
            user_quote_token_account,
            coin_creator,
            is_mayhem_mode,
            is_cashback_coin: false,
        }))
    }
}

/// Parse PumpSwap AddLiquidity event from pre-decoded data
#[inline(always)]
pub fn parse_add_liquidity_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const REQUIRED_LEN: usize = 10 * 8 + 5 * 32;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let lp_token_amount_out = read_u64_unchecked(data, 8);
        let max_base_amount_in = read_u64_unchecked(data, 16);
        let max_quote_amount_in = read_u64_unchecked(data, 24);
        let user_base_token_reserves = read_u64_unchecked(data, 32);
        let user_quote_token_reserves = read_u64_unchecked(data, 40);
        let pool_base_token_reserves = read_u64_unchecked(data, 48);
        let pool_quote_token_reserves = read_u64_unchecked(data, 56);
        let base_amount_in = read_u64_unchecked(data, 64);
        let quote_amount_in = read_u64_unchecked(data, 72);
        let lp_mint_supply = read_u64_unchecked(data, 80);

        let pool = read_pubkey_unchecked(data, 88);
        let user = read_pubkey_unchecked(data, 120);
        let user_base_token_account = read_pubkey_unchecked(data, 152);
        let user_quote_token_account = read_pubkey_unchecked(data, 184);
        let user_pool_token_account = read_pubkey_unchecked(data, 216);

        Some(DexEvent::PumpSwapLiquidityAdded(PumpSwapLiquidityAdded {
            metadata,
            timestamp,
            lp_token_amount_out,
            max_base_amount_in,
            max_quote_amount_in,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            base_amount_in,
            quote_amount_in,
            lp_mint_supply,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            user_pool_token_account,
        }))
    }
}

/// Parse PumpSwap RemoveLiquidity event from pre-decoded data
#[inline(always)]
pub fn parse_remove_liquidity_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    const REQUIRED_LEN: usize = 10 * 8 + 5 * 32;
    if data.len() < REQUIRED_LEN {
        return None;
    }

    unsafe {
        let timestamp = read_i64_unchecked(data, 0);
        let lp_token_amount_in = read_u64_unchecked(data, 8);
        let min_base_amount_out = read_u64_unchecked(data, 16);
        let min_quote_amount_out = read_u64_unchecked(data, 24);
        let user_base_token_reserves = read_u64_unchecked(data, 32);
        let user_quote_token_reserves = read_u64_unchecked(data, 40);
        let pool_base_token_reserves = read_u64_unchecked(data, 48);
        let pool_quote_token_reserves = read_u64_unchecked(data, 56);
        let base_amount_out = read_u64_unchecked(data, 64);
        let quote_amount_out = read_u64_unchecked(data, 72);
        let lp_mint_supply = read_u64_unchecked(data, 80);

        let pool = read_pubkey_unchecked(data, 88);
        let user = read_pubkey_unchecked(data, 120);
        let user_base_token_account = read_pubkey_unchecked(data, 152);
        let user_quote_token_account = read_pubkey_unchecked(data, 184);
        let user_pool_token_account = read_pubkey_unchecked(data, 216);

        Some(DexEvent::PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved {
            metadata,
            timestamp,
            lp_token_amount_in,
            min_base_amount_out,
            min_quote_amount_out,
            user_base_token_reserves,
            user_quote_token_reserves,
            pool_base_token_reserves,
            pool_quote_token_reserves,
            base_amount_out,
            quote_amount_out,
            lp_mint_supply,
            pool,
            user,
            user_base_token_account,
            user_quote_token_account,
            user_pool_token_account,
        }))
    }
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
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    fn metadata() -> EventMetadata {
        EventMetadata {
            signature: Signature::default(),
            slot: 0,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        }
    }

    fn write_u64(buf: &mut [u8], offset: usize, value: u64) {
        buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i64(buf: &mut [u8], offset: usize, value: i64) {
        buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_pubkey(buf: &mut [u8], offset: usize, value: Pubkey) {
        buf[offset..offset + 32].copy_from_slice(value.as_ref());
    }

    fn build_sell_payload(include_cashback: bool) -> Vec<u8> {
        let len = if include_cashback { 368 } else { 352 };
        let mut data = vec![0u8; len];

        write_i64(&mut data, 0, 1_713_498_953);
        write_u64(&mut data, 8, 11);
        write_u64(&mut data, 16, 22);
        write_u64(&mut data, 24, 33);
        write_u64(&mut data, 32, 44);
        write_u64(&mut data, 40, 55);
        write_u64(&mut data, 48, 66);
        write_u64(&mut data, 56, 77);
        write_u64(&mut data, 64, 88);
        write_u64(&mut data, 72, 99);
        write_u64(&mut data, 80, 111);
        write_u64(&mut data, 88, 122);
        write_u64(&mut data, 96, 133);
        write_u64(&mut data, 104, 144);

        write_pubkey(&mut data, 112, Pubkey::new_from_array([1; 32]));
        write_pubkey(&mut data, 144, Pubkey::new_from_array([2; 32]));
        write_pubkey(&mut data, 176, Pubkey::new_from_array([3; 32]));
        write_pubkey(&mut data, 208, Pubkey::new_from_array([4; 32]));
        write_pubkey(&mut data, 240, Pubkey::new_from_array([5; 32]));
        write_pubkey(&mut data, 272, Pubkey::new_from_array([6; 32]));
        write_pubkey(&mut data, 304, Pubkey::new_from_array([7; 32]));

        write_u64(&mut data, 336, 155);
        write_u64(&mut data, 344, 166);

        if include_cashback {
            write_u64(&mut data, 352, 177);
            write_u64(&mut data, 360, 188);
        }

        data
    }

    fn build_create_pool_payload(is_mayhem_mode: bool) -> Vec<u8> {
        let mut data = vec![0u8; 326];

        write_i64(&mut data, 0, 1_713_498_953);
        data[8..10].copy_from_slice(&42u16.to_le_bytes());
        write_pubkey(&mut data, 10, Pubkey::new_from_array([1; 32]));
        write_pubkey(&mut data, 42, Pubkey::new_from_array([2; 32]));
        write_pubkey(&mut data, 74, Pubkey::new_from_array([3; 32]));
        data[106] = 6;
        data[107] = 9;
        write_u64(&mut data, 108, 11);
        write_u64(&mut data, 116, 22);
        write_u64(&mut data, 124, 33);
        write_u64(&mut data, 132, 44);
        write_u64(&mut data, 140, 55);
        write_u64(&mut data, 148, 66);
        write_u64(&mut data, 156, 77);
        data[164] = 8;
        write_pubkey(&mut data, 165, Pubkey::new_from_array([4; 32]));
        write_pubkey(&mut data, 197, Pubkey::new_from_array([5; 32]));
        write_pubkey(&mut data, 229, Pubkey::new_from_array([6; 32]));
        write_pubkey(&mut data, 261, Pubkey::new_from_array([7; 32]));
        write_pubkey(&mut data, 293, Pubkey::new_from_array([8; 32]));
        data[325] = u8::from(is_mayhem_mode);

        data
    }

    #[test]
    fn test_discriminator_simd() {
        // 测试 SIMD discriminator 提取
        let log = "Program data: Z/RS H8v1d3cAAAAAAAAAAA=";
        let disc = extract_discriminator_simd(log);
        assert!(disc.is_some());
    }

    #[test]
    fn test_parse_performance() {
        // 性能测试
        let log = "Program data: Z/RS H8v1d3cAAAAAAAAAAA=";
        let sig = Signature::default();

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = parse_log(log, sig, 0, 0, Some(0), 0);
        }
        let elapsed = start.elapsed();

        println!("Average parse time: {} ns", elapsed.as_nanos() / 1000);
    }

    #[test]
    fn parse_sell_from_data_preserves_cashback_fields() {
        let event = parse_sell_from_data(&build_sell_payload(true), metadata())
            .expect("expected pumpswap sell event");

        let DexEvent::PumpSwapSell(event) = event else {
            panic!("expected PumpSwapSell event");
        };

        assert_eq!(event.cashback_fee_basis_points, 177);
        assert_eq!(event.cashback, 188);
        assert_eq!(event.coin_creator_fee_basis_points, 155);
        assert_eq!(event.coin_creator_fee, 166);
    }

    #[test]
    fn parse_sell_from_data_keeps_legacy_payload_compatible() {
        let event = parse_sell_from_data(&build_sell_payload(false), metadata())
            .expect("expected legacy pumpswap sell event");

        let DexEvent::PumpSwapSell(event) = event else {
            panic!("expected PumpSwapSell event");
        };

        assert_eq!(event.cashback_fee_basis_points, 0);
        assert_eq!(event.cashback, 0);
        assert_eq!(event.coin_creator_fee_basis_points, 155);
        assert_eq!(event.coin_creator_fee, 166);
    }

    #[test]
    fn parse_buy_from_data_rejects_truncated_min_base_payload() {
        assert!(parse_buy_from_data(&vec![0u8; 396], metadata()).is_none());
        assert!(parse_buy_from_data(&vec![0u8; 397], metadata()).is_some());
    }

    #[test]
    fn parse_create_pool_from_data_reads_mayhem_mode() {
        let event = parse_create_pool_from_data(&build_create_pool_payload(true), metadata())
            .expect("expected pumpswap create pool event");

        let DexEvent::PumpSwapCreatePool(event) = event else {
            panic!("expected PumpSwapCreatePool event");
        };

        assert_eq!(event.index, 42);
        assert!(event.is_mayhem_mode);
        assert!(!event.is_cashback_coin);
    }
}
