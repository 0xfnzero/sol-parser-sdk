//! Optimized log matcher with early discriminator filtering
//!
//! Performance strategy:
//! 1. SIMD-based log type detection (~50ns)
//! 2. Extract discriminator BEFORE full parsing (~50ns)
//! 3. Check filter at discriminator level - skip parsing if not needed
//! 4. Only parse events user actually configured
//! 5. Compiler-optimized base64 decoding (auto-vectorized with target-cpu=native)

use super::perf_hints::{likely, unlikely};
use crate::core::events::{DexEvent, EventMetadata};
use crate::grpc::types::{EventType, EventTypeFilter};
use crate::instr::program_ids;
use memchr::memmem;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

/// SIMD 优化的字符串查找器 - 预编译一次，重复使用
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
static RAYDIUM_AMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
static RAYDIUM_CLMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"));
static RAYDIUM_CPMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"));
static RAYDIUM_LAUNCHLAB_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj"));
static PROGRAM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program"));
static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: "));
static PUMPFUN_CREATE_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: G3KpTd7rY3Y"));
static WHIRL_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"whirL"));
static METEORA_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"meteora"));
static METEORA_LB_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"LB"));
static METEORA_DLMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"DLMM"));
static PUMPSWAP_LOWER_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"pumpswap"));
static PUMPSWAP_UPPER_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"PumpSwap"));

/// 预计算的程序 ID 字符串常量
pub mod program_id_strings {
    pub const PUMPFUN_INVOKE: &str = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke";
    pub const PUMPFUN_SUCCESS: &str = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success";
    pub const PUMPFUN_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

    pub const RAYDIUM_LAUNCHLAB_INVOKE: &str =
        "Program LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj invoke";
    pub const RAYDIUM_LAUNCHLAB_SUCCESS: &str =
        "Program LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj success";
    pub const RAYDIUM_LAUNCHLAB_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";

    pub const RAYDIUM_CLMM_INVOKE: &str =
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK invoke";
    pub const RAYDIUM_CLMM_SUCCESS: &str =
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK success";
    pub const RAYDIUM_CLMM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

    pub const RAYDIUM_CPMM_INVOKE: &str =
        "Program CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C invoke";
    pub const RAYDIUM_CPMM_SUCCESS: &str =
        "Program CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C success";
    pub const RAYDIUM_CPMM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

    pub const RAYDIUM_AMM_V4_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    // 常用的日志模式
    pub const PROGRAM_DATA: &str = "Program data: ";
    pub const PROGRAM_LOG: &str = "Program log: ";

    // PumpFun 事件 discriminator (base64)
    pub const PUMPFUN_CREATE_DISCRIMINATOR: &str = "GB7IKAUcB3c"; // [24, 30, 200, 40, 5, 28, 7, 119]
}

/// 快速日志类型枚举
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LogType {
    PumpFun,
    RaydiumLaunchlab,
    PumpAmm,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumAmm,
    OrcaWhirlpool,
    MeteoraAmm,
    MeteoraDamm,
    MeteoraDlmm,
    Unknown,
}

/// SIMD 优化的日志类型检测器 - 激进早期退出
#[inline(always)]
pub fn detect_log_type(log: &str) -> LogType {
    let log_bytes = log.as_bytes();

    // 第一步：快速长度检查 - 太短的日志直接跳过
    if log_bytes.len() < 20 {
        return LogType::Unknown;
    }

    // 第二步：检查是否有 "Program data:" - 这是事件日志的标志
    let has_program_data = PROGRAM_DATA_FINDER.find(log_bytes).is_some();

    // 只有 "Program data:" 日志才可能是交易事件
    if unlikely(!has_program_data) {
        return LogType::Unknown;
    }

    // 第三步：使用 SIMD 快速检测具体协议
    // Raydium AMM - 高频，有明确程序ID（最常见）
    if likely(RAYDIUM_AMM_FINDER.find(log_bytes).is_some()) {
        return LogType::RaydiumAmm;
    }

    // Raydium CLMM
    if RAYDIUM_CLMM_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumClmm;
    }

    // Raydium CPMM
    if RAYDIUM_CPMM_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumCpmm;
    }

    // Raydium LaunchLab (RaydiumLaunchlab)
    if RAYDIUM_LAUNCHLAB_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumLaunchlab;
    }

    // Orca Whirlpool
    if WHIRL_FINDER.find(log_bytes).is_some() {
        return LogType::OrcaWhirlpool;
    }

    // Meteora - SIMD 优化
    if let Some(pos) = METEORA_FINDER.find(log_bytes) {
        let rest = &log_bytes[pos..];
        if METEORA_LB_FINDER.find(rest).is_some() {
            return LogType::MeteoraDamm;
        } else if METEORA_DLMM_FINDER.find(rest).is_some() {
            return LogType::MeteoraDlmm;
        } else {
            return LogType::MeteoraAmm;
        }
    }

    // Pump AMM
    if PUMPSWAP_LOWER_FINDER.find(log_bytes).is_some()
        || PUMPSWAP_UPPER_FINDER.find(log_bytes).is_some()
    {
        return LogType::PumpAmm;
    }

    // PumpFun - 特殊处理：可能有程序ID，也可能直接是base64数据
    // 1. 先检查是否包含程序ID（高频事件）
    if likely(PUMPFUN_FINDER.find(log_bytes).is_some()) {
        return LogType::PumpFun;
    }

    // 2. 兜底：有 "Program data:" 但无法识别协议的，尝试作为 PumpFun 解析
    // PumpFun的日志格式：Program data: <base64>
    // 只要日志够长且包含Program data，就认为可能是PumpFun
    if log.len() > 30 {
        return LogType::PumpFun;
    }

    LogType::Unknown
}

// ============================================================================
// Discriminator constants (compile-time computed) - All protocols
// ============================================================================
mod discriminators {
    // PumpFun discriminators
    pub const PUMPFUN_CREATE: u64 = u64::from_le_bytes([27, 114, 169, 77, 222, 235, 99, 118]);
    pub const PUMPFUN_TRADE: u64 = u64::from_le_bytes([189, 219, 127, 211, 78, 230, 97, 238]);
    pub const PUMPFUN_MIGRATE: u64 = u64::from_le_bytes([189, 233, 93, 185, 92, 148, 234, 148]);
    pub const PUMPFUN_MIGRATE_BONDING_CURVE_CREATOR: u64 =
        u64::from_le_bytes([155, 167, 104, 220, 213, 108, 243, 3]);
    // Raydium LaunchLab event discriminators. `TRADE` intentionally equals
    // PumpFun's TradeEvent discriminator, so gRPC must route logs with program
    // context instead of discriminator alone.
    pub const RAYDIUM_LAUNCHLAB_POOL_CREATE: u64 =
        u64::from_le_bytes([151, 215, 226, 9, 118, 161, 115, 174]);
    pub const RAYDIUM_LAUNCHLAB_TRADE: u64 =
        u64::from_le_bytes([189, 219, 127, 211, 78, 230, 97, 238]);
    // Pump fees (`idls/pump_fees.json` event discriminators)
    pub const PUMP_FEES_CREATE_FEE_SHARING_CONFIG: u64 =
        u64::from_le_bytes([133, 105, 170, 200, 184, 116, 251, 88]);
    pub const PUMP_FEES_INITIALIZE_FEE_CONFIG: u64 =
        u64::from_le_bytes([89, 138, 244, 230, 10, 56, 226, 126]);
    pub const PUMP_FEES_RESET_FEE_SHARING_CONFIG: u64 =
        u64::from_le_bytes([203, 204, 151, 226, 120, 55, 214, 243]);
    pub const PUMP_FEES_REVOKE_FEE_SHARING_AUTHORITY: u64 =
        u64::from_le_bytes([114, 23, 101, 60, 14, 190, 153, 62]);
    pub const PUMP_FEES_TRANSFER_FEE_SHARING_AUTHORITY: u64 =
        u64::from_le_bytes([124, 143, 198, 245, 77, 184, 8, 236]);
    pub const PUMP_FEES_UPDATE_ADMIN: u64 =
        u64::from_le_bytes([225, 152, 171, 87, 246, 63, 66, 234]);
    pub const PUMP_FEES_UPDATE_FEE_CONFIG: u64 =
        u64::from_le_bytes([90, 23, 65, 35, 62, 244, 188, 208]);
    pub const PUMP_FEES_UPDATE_FEE_SHARES: u64 =
        u64::from_le_bytes([21, 186, 196, 184, 91, 228, 225, 203]);
    pub const PUMP_FEES_UPSERT_FEE_TIERS: u64 =
        u64::from_le_bytes([171, 89, 169, 187, 122, 186, 33, 204]);

    // PumpSwap discriminators
    pub const PUMPSWAP_BUY: u64 = u64::from_le_bytes([103, 244, 82, 31, 44, 245, 119, 119]);
    pub const PUMPSWAP_SELL: u64 = u64::from_le_bytes([62, 47, 55, 10, 165, 3, 220, 42]);
    pub const PUMPSWAP_CREATE_POOL: u64 =
        u64::from_le_bytes([177, 49, 12, 210, 160, 118, 167, 116]);
    pub const PUMPSWAP_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([120, 248, 61, 83, 31, 142, 107, 144]);
    pub const PUMPSWAP_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([22, 9, 133, 26, 160, 44, 71, 192]);

    // Raydium CLMM discriminators
    pub const RAYDIUM_CLMM_SWAP: u64 = u64::from_le_bytes([64, 198, 205, 232, 38, 8, 113, 226]);
    pub const RAYDIUM_CLMM_INCREASE_LIQUIDITY: u64 =
        u64::from_le_bytes([49, 79, 105, 212, 32, 34, 30, 84]);
    pub const RAYDIUM_CLMM_DECREASE_LIQUIDITY: u64 =
        u64::from_le_bytes([58, 222, 86, 58, 68, 50, 85, 56]);
    pub const RAYDIUM_CLMM_LIQUIDITY_CHANGE: u64 =
        u64::from_le_bytes([126, 240, 175, 206, 158, 88, 153, 107]);
    pub const RAYDIUM_CLMM_CONFIG_CHANGE: u64 =
        u64::from_le_bytes([247, 189, 7, 119, 106, 112, 95, 151]);
    pub const RAYDIUM_CLMM_CREATE_PERSONAL_POSITION: u64 =
        u64::from_le_bytes([100, 30, 87, 249, 196, 223, 154, 206]);
    pub const RAYDIUM_CLMM_LIQUIDITY_CALCULATE: u64 =
        u64::from_le_bytes([237, 112, 148, 230, 57, 84, 180, 162]);
    pub const RAYDIUM_CLMM_OPEN_LIMIT_ORDER: u64 =
        u64::from_le_bytes([106, 24, 71, 85, 57, 169, 158, 216]);
    pub const RAYDIUM_CLMM_INCREASE_LIMIT_ORDER: u64 =
        u64::from_le_bytes([11, 120, 13, 204, 199, 87, 19, 200]);
    pub const RAYDIUM_CLMM_DECREASE_LIMIT_ORDER: u64 =
        u64::from_le_bytes([70, 48, 40, 221, 219, 237, 212, 163]);
    pub const RAYDIUM_CLMM_SETTLE_LIMIT_ORDER: u64 =
        u64::from_le_bytes([88, 119, 77, 164, 125, 124, 10, 194]);
    pub const RAYDIUM_CLMM_UPDATE_REWARD_INFOS: u64 =
        u64::from_le_bytes([109, 127, 186, 78, 114, 65, 37, 236]);
    pub const RAYDIUM_CLMM_CREATE_POOL: u64 = u64::from_le_bytes([25, 94, 75, 47, 112, 99, 53, 63]);
    pub const RAYDIUM_CLMM_COLLECT_PERSONAL_FEE: u64 =
        u64::from_le_bytes([166, 174, 105, 192, 81, 161, 83, 105]);
    pub const RAYDIUM_CLMM_COLLECT_PROTOCOL_FEE: u64 =
        u64::from_le_bytes([206, 87, 17, 79, 45, 41, 213, 61]);

    // Raydium CPMM discriminators
    pub const RAYDIUM_CPMM_SWAP_BASE_IN: u64 =
        u64::from_le_bytes([143, 190, 90, 218, 196, 30, 51, 222]);
    pub const RAYDIUM_CPMM_SWAP_BASE_OUT: u64 =
        u64::from_le_bytes([55, 217, 98, 86, 163, 74, 180, 173]);
    pub const RAYDIUM_CPMM_CREATE_POOL: u64 =
        u64::from_le_bytes([233, 146, 209, 142, 207, 104, 64, 188]);
    pub const RAYDIUM_CPMM_DEPOSIT: u64 =
        u64::from_le_bytes([242, 35, 198, 137, 82, 225, 242, 182]);
    pub const RAYDIUM_CPMM_WITHDRAW: u64 =
        u64::from_le_bytes([183, 18, 70, 156, 148, 109, 161, 34]);

    // Raydium AMM V4 discriminators
    pub const RAYDIUM_AMM_SWAP_BASE_IN: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 9]);
    pub const RAYDIUM_AMM_SWAP_BASE_OUT: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 11]);
    pub const RAYDIUM_AMM_DEPOSIT: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 3]);
    pub const RAYDIUM_AMM_WITHDRAW: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 4]);
    pub const RAYDIUM_AMM_INITIALIZE2: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 1]);
    pub const RAYDIUM_AMM_WITHDRAW_PNL: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 7]);

    // Orca Whirlpool discriminators
    pub const ORCA_TRADED: u64 = u64::from_le_bytes([225, 202, 73, 175, 147, 43, 160, 150]);
    pub const ORCA_LIQUIDITY_INCREASED: u64 =
        u64::from_le_bytes([30, 7, 144, 181, 102, 254, 155, 161]);
    pub const ORCA_LIQUIDITY_DECREASED: u64 =
        u64::from_le_bytes([166, 1, 36, 71, 112, 202, 181, 171]);
    pub const ORCA_POOL_INITIALIZED: u64 =
        u64::from_le_bytes([100, 118, 173, 87, 12, 198, 254, 229]);

    // Meteora AMM discriminators
    pub const METEORA_AMM_SWAP: u64 = u64::from_le_bytes([81, 108, 227, 190, 205, 208, 10, 196]);
    pub const METEORA_AMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([31, 94, 125, 90, 227, 52, 61, 186]);
    pub const METEORA_AMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([116, 244, 97, 232, 103, 31, 152, 58]);
    pub const METEORA_AMM_BOOTSTRAP_LIQUIDITY: u64 =
        u64::from_le_bytes([121, 127, 38, 136, 92, 55, 14, 247]);
    pub const METEORA_AMM_POOL_CREATED: u64 =
        u64::from_le_bytes([202, 44, 41, 88, 104, 220, 157, 82]);
    pub const METEORA_AMM_SET_POOL_FEES: u64 =
        u64::from_le_bytes([245, 26, 198, 164, 88, 18, 75, 9]);

    // Meteora DAMM V2 discriminators
    pub const METEORA_DAMM_SWAP: u64 = u64::from_le_bytes([27, 60, 21, 213, 138, 170, 187, 147]);
    pub const METEORA_DAMM_SWAP2: u64 = u64::from_le_bytes([189, 66, 51, 168, 38, 80, 117, 153]);
    pub const METEORA_DAMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([175, 242, 8, 157, 30, 247, 185, 169]);
    pub const METEORA_DAMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([87, 46, 88, 98, 175, 96, 34, 91]);
    pub const METEORA_DAMM_INITIALIZE_POOL: u64 =
        u64::from_le_bytes([228, 50, 246, 85, 203, 66, 134, 37]);
    pub const METEORA_DAMM_CREATE_POSITION: u64 =
        u64::from_le_bytes([156, 15, 119, 198, 29, 181, 221, 55]);
    pub const METEORA_DAMM_CLOSE_POSITION: u64 =
        u64::from_le_bytes([20, 145, 144, 68, 143, 142, 214, 178]);

    // Meteora DBC discriminators. Some values intentionally overlap DAMM V2,
    // so they must be routed with program context.
    pub const METEORA_DBC_SWAP: u64 = u64::from_le_bytes([27, 60, 21, 213, 138, 170, 187, 147]);
    pub const METEORA_DBC_INITIALIZE_POOL: u64 =
        u64::from_le_bytes([228, 50, 246, 85, 203, 66, 134, 37]);
    pub const METEORA_DBC_CURVE_COMPLETE: u64 =
        u64::from_le_bytes([229, 231, 86, 84, 156, 134, 75, 24]);

    // Meteora DLMM discriminators
    pub const METEORA_DLMM_SWAP: u64 = u64::from_le_bytes([143, 190, 90, 218, 196, 30, 51, 222]);
    pub const METEORA_DLMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([181, 157, 89, 67, 143, 182, 52, 72]);
    pub const METEORA_DLMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([80, 85, 209, 72, 24, 206, 35, 178]);
    pub const METEORA_DLMM_INITIALIZE_POOL: u64 =
        u64::from_le_bytes([95, 180, 10, 172, 84, 174, 232, 40]);
    pub const METEORA_DLMM_INITIALIZE_BIN_ARRAY: u64 =
        u64::from_le_bytes([11, 18, 155, 194, 33, 115, 238, 119]);
    pub const METEORA_DLMM_CREATE_POSITION: u64 =
        u64::from_le_bytes([123, 233, 11, 43, 146, 180, 97, 119]);
    pub const METEORA_DLMM_CLOSE_POSITION: u64 =
        u64::from_le_bytes([94, 168, 102, 45, 59, 122, 137, 54]);
    pub const METEORA_DLMM_CLAIM_FEE: u64 = u64::from_le_bytes([152, 70, 208, 111, 104, 91, 44, 1]);
}

/// Optimized unified log parser with **discriminator predecode, decode-on-match** strategy
///
/// **Performance Strategy**:
/// 1. Decode only the first 8 event bytes to read the discriminator
/// 2. Check filter BEFORE decoding the full event payload
/// 3. Decode matching payloads once to a stack buffer; rare large events use heap
/// 4. Parse only the specific event type requested
///
/// **Key optimization**: narrow filters skip full base64 decoding entirely.
/// Old: decode full payload -> check filter -> parse
/// New: decode discriminator prefix -> check filter -> decode matching payload once
#[inline(always)]
/// `recent_blockhash`: pass as `Option<&[u8]>`; only cloned when an event is built (low latency).
pub fn parse_log_optimized(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
) -> Option<DexEvent> {
    parse_log_optimized_inner(
        log,
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
        event_type_filter,
        is_created_buy,
        recent_blockhash,
        None,
    )
}

/// Program-aware log parser for gRPC/RPC transaction logs.
///
/// `Program data:` lines do not carry the emitting program id. The caller should
/// pass the current invoke stack's program id so discriminators shared by
/// multiple Anchor programs are routed correctly.
#[inline(always)]
pub fn parse_log_optimized_with_program_id(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
    program_id: Option<&Pubkey>,
) -> Option<DexEvent> {
    parse_log_optimized_inner(
        log,
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
        event_type_filter,
        is_created_buy,
        recent_blockhash,
        program_id,
    )
}

#[inline(always)]
fn decode_base64_discriminator(trimmed: &str) -> Option<u64> {
    let bytes = trimmed.as_bytes();
    if bytes.len() < 12 {
        return None;
    }

    let mut discriminator_buf = [0u8; 9];
    let decoded_len = {
        use base64_simd::AsOut;
        let decoded =
            base64_simd::STANDARD.decode(&bytes[..12], discriminator_buf.as_mut().as_out()).ok()?;
        decoded.len()
    };
    if decoded_len < 8 {
        return None;
    }

    Some(unsafe { (discriminator_buf.as_ptr() as *const u64).read_unaligned() })
}

#[inline(always)]
fn filter_includes_known_program(program_id: &Pubkey, filter: &EventTypeFilter) -> bool {
    match *program_id {
        program_ids::PUMPFUN_PROGRAM_ID => filter.includes_pumpfun(),
        program_ids::PUMP_FEES_PROGRAM_ID => filter.includes_pump_fees(),
        program_ids::PUMPSWAP_PROGRAM_ID => filter.includes_pumpswap(),
        program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID => filter.includes_raydium_launchlab(),
        program_ids::RAYDIUM_CLMM_PROGRAM_ID => filter.includes_raydium_clmm(),
        program_ids::RAYDIUM_CPMM_PROGRAM_ID => filter.includes_raydium_cpmm(),
        program_ids::RAYDIUM_AMM_V4_PROGRAM_ID => filter.includes_raydium_amm_v4(),
        program_ids::ORCA_WHIRLPOOL_PROGRAM_ID => filter.includes_orca_whirlpool(),
        program_ids::METEORA_POOLS_PROGRAM_ID => filter.includes_meteora_pools(),
        program_ids::METEORA_DAMM_V2_PROGRAM_ID => filter.includes_meteora_damm_v2(),
        program_ids::METEORA_DLMM_PROGRAM_ID => filter.includes_meteora_dlmm(),
        program_ids::METEORA_DBC_PROGRAM_ID => filter.includes_meteora_dbc(),
        _ => true,
    }
}

#[inline(always)]
fn filter_wants_supported_logs(filter: &EventTypeFilter) -> bool {
    filter.includes_pumpfun()
        || filter.includes_pump_fees()
        || filter.includes_pumpswap()
        || filter.includes_raydium_launchlab()
        || filter.includes_raydium_clmm()
        || filter.includes_raydium_cpmm()
        || filter.includes_raydium_amm_v4()
        || filter.includes_orca_whirlpool()
        || filter.includes_meteora_pools()
        || filter.includes_meteora_damm_v2()
        || filter.includes_meteora_dlmm()
        || filter.includes_meteora_dbc()
}

#[inline(always)]
fn filter_wants_pumpfun_trade_event(filter: &EventTypeFilter) -> bool {
    filter.should_include(EventType::PumpFunTrade)
        || filter.should_include(EventType::PumpFunBuy)
        || filter.should_include(EventType::PumpFunSell)
        || filter.should_include(EventType::PumpFunBuyExactSolIn)
}

#[inline(always)]
fn unscoped_filter_allows_discriminator(discriminator: u64, filter: &EventTypeFilter) -> bool {
    match discriminator {
        // Shared by Pump.fun trade and Raydium LaunchLab/RaydiumLaunchlab trade.
        discriminators::PUMPFUN_TRADE => {
            filter_wants_pumpfun_trade_event(filter)
                || filter.should_include(EventType::RaydiumLaunchlabTrade)
        }
        // Shared by Raydium CPMM swap-base-in and Meteora DLMM swap.
        discriminators::RAYDIUM_CPMM_SWAP_BASE_IN => {
            filter.should_include(EventType::RaydiumCpmmSwap)
                || filter.should_include(EventType::MeteoraDlmmSwap)
        }
        _ => discriminator_to_event_type(discriminator)
            .map(|event_type| filter.should_include(event_type))
            .unwrap_or_else(|| filter_wants_supported_logs(filter)),
    }
}

#[inline(always)]
fn filter_allows_discriminator(
    program_id: Option<&Pubkey>,
    discriminator: u64,
    event_type_filter: Option<&EventTypeFilter>,
) -> bool {
    let Some(filter) = event_type_filter else {
        return true;
    };

    if let Some(program_id) = program_id {
        if *program_id == program_ids::PUMPFUN_PROGRAM_ID
            && discriminator == discriminators::PUMPFUN_TRADE
        {
            return filter_wants_pumpfun_trade_event(filter);
        }
        if let Some(event_type) =
            program_scoped_discriminator_to_event_type(program_id, discriminator)
        {
            return filter.should_include(event_type);
        }
        return filter_includes_known_program(program_id, filter);
    }

    unscoped_filter_allows_discriminator(discriminator, filter)
}

#[inline(always)]
fn apply_event_type_filter(
    event: DexEvent,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    if let Some(filter) = event_type_filter {
        if !filter.should_include_dex_event(&event) {
            return None;
        }
    }
    Some(event)
}

#[inline(always)]
fn parse_log_optimized_inner(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
    program_id: Option<&Pubkey>,
) -> Option<DexEvent> {
    // Step 1: Find "Program data: " prefix using SIMD
    let log_bytes = log.as_bytes();
    let pos = PROGRAM_DATA_FINDER.find(log_bytes)?;
    let data_start = pos + 14; // "Program data: " length

    if log_bytes.len() <= data_start {
        return None;
    }

    // Step 2: Decode base64 ONCE. Normal swap logs stay on the stack; rare large
    // IDL events (for example pump-fees vectors) fall back to heap instead of being dropped.
    const STACK_DECODE_CAP: usize = 2048;
    let data_part = &log[data_start..];
    let trimmed = data_part.trim();

    // Decode the discriminator prefix before touching the full payload. This is
    // the fastest reject path for users subscribing to a narrow event set.
    let discriminator = decode_base64_discriminator(trimmed)?;
    if !filter_allows_discriminator(program_id, discriminator, event_type_filter) {
        return None;
    }

    // SIMD-accelerated base64 decoding (AVX2/SSE4/NEON)
    use base64_simd::AsOut;
    let max_decoded_len = (trimmed.len() / 4).saturating_mul(3).saturating_add(3);
    let mut stack_buf = [0u8; STACK_DECODE_CAP];
    let heap_buf: Vec<u8>;
    let program_data: &[u8] = if max_decoded_len <= STACK_DECODE_CAP {
        let decoded_len = {
            let decoded_slice = base64_simd::STANDARD
                .decode(trimmed.as_bytes(), stack_buf.as_mut().as_out())
                .ok()?;
            decoded_slice.len()
        };
        &stack_buf[..decoded_len]
    } else {
        heap_buf = base64_simd::STANDARD.decode_to_vec(trimmed.as_bytes()).ok()?;
        heap_buf.as_slice()
    };

    if program_data.len() < 8 {
        return None;
    }

    debug_assert_eq!(discriminator, unsafe {
        (program_data.as_ptr() as *const u64).read_unaligned()
    });

    // Step 6: Parse the specific event type (data already decoded)
    let data = &program_data[8..]; // Skip discriminator

    use crate::core::events::*;

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: recent_blockhash.map(|s| bs58::encode(s).into_string()),
    };

    if let Some(program_id) = program_id {
        return parse_program_scoped_event(
            program_id,
            discriminator,
            data,
            metadata,
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            event_type_filter,
            is_created_buy,
        );
    }

    // ========================================================================
    // Hot-path optimization: Fast check for top 5 most common discriminators
    // This avoids the large match statement for ~80% of events
    // Expected savings: 5-20ns per hot event
    // ========================================================================

    // Check hot-path discriminators first (ordered by frequency)
    if likely(discriminator == discriminators::PUMPFUN_TRADE) {
        // PumpFun Trade - Most common (~40% of all events)
        let event = crate::logs::pump::parse_trade_from_data(data, metadata, is_created_buy)?;
        return apply_event_type_filter(event, event_type_filter);
    }

    if likely(discriminator == discriminators::RAYDIUM_CLMM_SWAP) {
        // Raydium CLMM Swap - High frequency (~20% of events)
        return apply_event_type_filter(
            crate::logs::raydium_clmm::parse_swap_from_data(data, metadata)?,
            event_type_filter,
        );
    }

    if likely(discriminator == discriminators::RAYDIUM_AMM_SWAP_BASE_IN) {
        // Raydium AMM Swap Base In - High frequency (~15% of events)
        return apply_event_type_filter(
            crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)?,
            event_type_filter,
        );
    }

    if likely(discriminator == discriminators::PUMPSWAP_BUY) {
        // PumpSwap Buy - Medium frequency (~10% of events)
        return apply_event_type_filter(
            crate::logs::pump_amm::parse_buy_from_data(data, metadata)?,
            event_type_filter,
        );
    }

    if discriminator == discriminators::PUMPSWAP_SELL {
        // PumpSwap Sell - Medium frequency (~5% of events)
        return apply_event_type_filter(
            crate::logs::pump_amm::parse_sell_from_data(data, metadata)?,
            event_type_filter,
        );
    }

    // ========================================================================
    // Cold path: Handle remaining ~10% of events via match statement
    // ========================================================================

    let event = match discriminator {
        // Note: Hot-path discriminators (PUMPFUN_TRADE, RAYDIUM_CLMM_SWAP, RAYDIUM_AMM_SWAP_BASE_IN,
        // PUMPSWAP_BUY, PUMPSWAP_SELL) are handled above and never reach this match statement

        // PumpFun events (cold path)
        discriminators::PUMPFUN_CREATE => crate::logs::pump::parse_create_from_data(data, metadata),
        discriminators::PUMPFUN_MIGRATE => {
            crate::logs::pump::parse_migrate_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_CREATE_FEE_SHARING_CONFIG => {
            crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_INITIALIZE_FEE_CONFIG => {
            crate::logs::pump_fees::parse_initialize_fee_config_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_RESET_FEE_SHARING_CONFIG => {
            crate::logs::pump_fees::parse_reset_fee_sharing_config_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_REVOKE_FEE_SHARING_AUTHORITY => {
            crate::logs::pump_fees::parse_revoke_fee_sharing_authority_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_TRANSFER_FEE_SHARING_AUTHORITY => {
            crate::logs::pump_fees::parse_transfer_fee_sharing_authority_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_UPDATE_ADMIN => {
            crate::logs::pump_fees::parse_update_admin_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_UPDATE_FEE_CONFIG => {
            crate::logs::pump_fees::parse_update_fee_config_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_UPDATE_FEE_SHARES => {
            crate::logs::pump_fees::parse_update_fee_shares_from_data(data, metadata)
        }
        discriminators::PUMP_FEES_UPSERT_FEE_TIERS => {
            crate::logs::pump_fees::parse_upsert_fee_tiers_from_data(data, metadata)
        }
        discriminators::PUMPFUN_MIGRATE_BONDING_CURVE_CREATOR => {
            crate::logs::pump::parse_migrate_bonding_curve_creator_from_data(data, metadata)
        }
        discriminators::PUMPSWAP_CREATE_POOL => {
            crate::logs::pump_amm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::PUMPSWAP_ADD_LIQUIDITY => {
            crate::logs::pump_amm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::PUMPSWAP_REMOVE_LIQUIDITY => {
            crate::logs::pump_amm::parse_remove_liquidity_from_data(data, metadata)
        }

        // ========== Other protocols - route by discriminator ==========
        // Raydium CLMM - use from_data functions (cold path)
        discriminators::RAYDIUM_CLMM_INCREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_DECREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_LIQUIDITY_CHANGE => {
            crate::logs::raydium_clmm::parse_liquidity_change_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_CONFIG_CHANGE => {
            crate::logs::raydium_clmm::parse_config_change_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_CREATE_PERSONAL_POSITION => {
            crate::logs::raydium_clmm::parse_create_personal_position_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_LIQUIDITY_CALCULATE => {
            crate::logs::raydium_clmm::parse_liquidity_calculate_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_OPEN_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_open_limit_order_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_INCREASE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_increase_limit_order_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_DECREASE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_decrease_limit_order_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_SETTLE_LIMIT_ORDER => {
            crate::logs::raydium_clmm::parse_settle_limit_order_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_UPDATE_REWARD_INFOS => {
            crate::logs::raydium_clmm::parse_update_reward_infos_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_CREATE_POOL => {
            crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_COLLECT_PERSONAL_FEE => {
            crate::logs::raydium_clmm::parse_collect_personal_fee_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_COLLECT_PROTOCOL_FEE => {
            crate::logs::raydium_clmm::parse_collect_protocol_fee_from_data(data, metadata)
        }

        // Raydium CPMM - use from_data functions (single decode)
        discriminators::RAYDIUM_CPMM_SWAP_BASE_IN => {
            crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CPMM_SWAP_BASE_OUT => {
            crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
        }
        // Note: RAYDIUM_CPMM_CREATE_POOL discriminator conflicts with RAYDIUM_CLMM_CREATE_POOL
        // CPMM create pool is rare, handled via log content detection if needed
        discriminators::RAYDIUM_CPMM_DEPOSIT => {
            crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CPMM_WITHDRAW => {
            crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
        }

        // Raydium AMM V4 - use from_data functions (single decode)
        discriminators::RAYDIUM_AMM_SWAP_BASE_IN => {
            crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_SWAP_BASE_OUT => {
            crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_DEPOSIT => {
            crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_WITHDRAW => {
            crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_INITIALIZE2 => {
            crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_WITHDRAW_PNL => {
            crate::logs::raydium_amm::parse_withdraw_pnl_from_data(data, metadata)
        }

        // Orca Whirlpool - use from_data functions (single decode)
        discriminators::ORCA_TRADED => {
            crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
        }
        discriminators::ORCA_LIQUIDITY_INCREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
        }
        discriminators::ORCA_LIQUIDITY_DECREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
        }
        discriminators::ORCA_POOL_INITIALIZED => {
            crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
        }

        // Meteora AMM - use from_data functions (single decode)
        discriminators::METEORA_AMM_SWAP => {
            crate::logs::meteora_amm::parse_swap_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_ADD_LIQUIDITY => {
            crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_REMOVE_LIQUIDITY => {
            crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_BOOTSTRAP_LIQUIDITY => {
            crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_POOL_CREATED => {
            crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_SET_POOL_FEES => {
            crate::logs::meteora_amm::parse_set_pool_fees_from_data(data, metadata)
        }

        // Meteora DAMM V2
        discriminators::METEORA_DAMM_SWAP => {
            crate::logs::meteora_damm::parse_swap_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_SWAP2 => {
            crate::logs::meteora_damm::parse_swap2_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_ADD_LIQUIDITY => {
            crate::logs::meteora_damm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_REMOVE_LIQUIDITY => {
            crate::logs::meteora_damm::parse_remove_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_INITIALIZE_POOL => {
            crate::logs::meteora_damm::parse_initialize_pool_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_CREATE_POSITION => {
            crate::logs::meteora_damm::parse_create_position_from_data(data, metadata)
        }
        discriminators::METEORA_DAMM_CLOSE_POSITION => {
            crate::logs::meteora_damm::parse_close_position_from_data(data, metadata)
        }

        // NOTE: Meteora DLMM discriminators conflict with Raydium CPMM!
        // METEORA_DLMM_SWAP == RAYDIUM_CPMM_SWAP_BASE_IN
        // Handle DLMM in fallback using log content detection

        // Unknown discriminator - try fallback protocols
        _ => {
            // Try Meteora DLMM (has discriminator conflict with Raydium CPMM)
            if let Some(event) = crate::logs::parse_meteora_dlmm_log(
                log,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            ) {
                return apply_event_type_filter(event, event_type_filter);
            }
            None
        }
    }?;
    apply_event_type_filter(event, event_type_filter)
}

#[inline(always)]
fn program_scoped_discriminator_to_event_type(
    program_id: &Pubkey,
    discriminator: u64,
) -> Option<EventType> {
    match *program_id {
        program_ids::PUMPFUN_PROGRAM_ID => match discriminator {
            discriminators::PUMPFUN_CREATE => Some(EventType::PumpFunCreate),
            discriminators::PUMPFUN_TRADE => Some(EventType::PumpFunTrade),
            discriminators::PUMPFUN_MIGRATE => Some(EventType::PumpFunMigrate),
            discriminators::PUMPFUN_MIGRATE_BONDING_CURVE_CREATOR => {
                Some(EventType::PumpFunMigrateBondingCurveCreator)
            }
            _ => None,
        },
        program_ids::PUMP_FEES_PROGRAM_ID => match discriminator {
            discriminators::PUMP_FEES_CREATE_FEE_SHARING_CONFIG => {
                Some(EventType::PumpFeesCreateFeeSharingConfig)
            }
            discriminators::PUMP_FEES_INITIALIZE_FEE_CONFIG => {
                Some(EventType::PumpFeesInitializeFeeConfig)
            }
            discriminators::PUMP_FEES_RESET_FEE_SHARING_CONFIG => {
                Some(EventType::PumpFeesResetFeeSharingConfig)
            }
            discriminators::PUMP_FEES_REVOKE_FEE_SHARING_AUTHORITY => {
                Some(EventType::PumpFeesRevokeFeeSharingAuthority)
            }
            discriminators::PUMP_FEES_TRANSFER_FEE_SHARING_AUTHORITY => {
                Some(EventType::PumpFeesTransferFeeSharingAuthority)
            }
            discriminators::PUMP_FEES_UPDATE_ADMIN => Some(EventType::PumpFeesUpdateAdmin),
            discriminators::PUMP_FEES_UPDATE_FEE_CONFIG => Some(EventType::PumpFeesUpdateFeeConfig),
            discriminators::PUMP_FEES_UPDATE_FEE_SHARES => Some(EventType::PumpFeesUpdateFeeShares),
            discriminators::PUMP_FEES_UPSERT_FEE_TIERS => Some(EventType::PumpFeesUpsertFeeTiers),
            _ => None,
        },
        program_ids::PUMPSWAP_PROGRAM_ID => match discriminator {
            discriminators::PUMPSWAP_BUY => Some(EventType::PumpSwapBuy),
            discriminators::PUMPSWAP_SELL => Some(EventType::PumpSwapSell),
            discriminators::PUMPSWAP_CREATE_POOL => Some(EventType::PumpSwapCreatePool),
            discriminators::PUMPSWAP_ADD_LIQUIDITY => Some(EventType::PumpSwapLiquidityAdded),
            discriminators::PUMPSWAP_REMOVE_LIQUIDITY => Some(EventType::PumpSwapLiquidityRemoved),
            _ => None,
        },
        program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID => match discriminator {
            discriminators::RAYDIUM_LAUNCHLAB_TRADE => Some(EventType::RaydiumLaunchlabTrade),
            discriminators::RAYDIUM_LAUNCHLAB_POOL_CREATE => {
                Some(EventType::RaydiumLaunchlabPoolCreate)
            }
            _ => None,
        },
        program_ids::RAYDIUM_CLMM_PROGRAM_ID => match discriminator {
            discriminators::RAYDIUM_CLMM_SWAP => Some(EventType::RaydiumClmmSwap),
            discriminators::RAYDIUM_CLMM_INCREASE_LIQUIDITY => {
                Some(EventType::RaydiumClmmIncreaseLiquidity)
            }
            discriminators::RAYDIUM_CLMM_DECREASE_LIQUIDITY => {
                Some(EventType::RaydiumClmmDecreaseLiquidity)
            }
            discriminators::RAYDIUM_CLMM_LIQUIDITY_CHANGE => {
                Some(EventType::RaydiumClmmLiquidityChange)
            }
            discriminators::RAYDIUM_CLMM_CONFIG_CHANGE => Some(EventType::RaydiumClmmConfigChange),
            discriminators::RAYDIUM_CLMM_CREATE_PERSONAL_POSITION => {
                Some(EventType::RaydiumClmmCreatePersonalPosition)
            }
            discriminators::RAYDIUM_CLMM_LIQUIDITY_CALCULATE => {
                Some(EventType::RaydiumClmmLiquidityCalculate)
            }
            discriminators::RAYDIUM_CLMM_OPEN_LIMIT_ORDER => {
                Some(EventType::RaydiumClmmOpenLimitOrder)
            }
            discriminators::RAYDIUM_CLMM_INCREASE_LIMIT_ORDER => {
                Some(EventType::RaydiumClmmIncreaseLimitOrder)
            }
            discriminators::RAYDIUM_CLMM_DECREASE_LIMIT_ORDER => {
                Some(EventType::RaydiumClmmDecreaseLimitOrder)
            }
            discriminators::RAYDIUM_CLMM_SETTLE_LIMIT_ORDER => {
                Some(EventType::RaydiumClmmSettleLimitOrder)
            }
            discriminators::RAYDIUM_CLMM_UPDATE_REWARD_INFOS => {
                Some(EventType::RaydiumClmmUpdateRewardInfos)
            }
            discriminators::RAYDIUM_CLMM_CREATE_POOL => Some(EventType::RaydiumClmmCreatePool),
            discriminators::RAYDIUM_CLMM_COLLECT_PERSONAL_FEE
            | discriminators::RAYDIUM_CLMM_COLLECT_PROTOCOL_FEE => {
                Some(EventType::RaydiumClmmCollectFee)
            }
            _ => None,
        },
        program_ids::RAYDIUM_CPMM_PROGRAM_ID => match discriminator {
            discriminators::RAYDIUM_CPMM_SWAP_BASE_IN
            | discriminators::RAYDIUM_CPMM_SWAP_BASE_OUT => Some(EventType::RaydiumCpmmSwap),
            discriminators::RAYDIUM_CPMM_CREATE_POOL => Some(EventType::RaydiumCpmmInitialize),
            discriminators::RAYDIUM_CPMM_DEPOSIT => Some(EventType::RaydiumCpmmDeposit),
            discriminators::RAYDIUM_CPMM_WITHDRAW => Some(EventType::RaydiumCpmmWithdraw),
            _ => None,
        },
        program_ids::RAYDIUM_AMM_V4_PROGRAM_ID => match discriminator {
            discriminators::RAYDIUM_AMM_SWAP_BASE_IN
            | discriminators::RAYDIUM_AMM_SWAP_BASE_OUT => Some(EventType::RaydiumAmmV4Swap),
            discriminators::RAYDIUM_AMM_DEPOSIT => Some(EventType::RaydiumAmmV4Deposit),
            discriminators::RAYDIUM_AMM_WITHDRAW => Some(EventType::RaydiumAmmV4Withdraw),
            discriminators::RAYDIUM_AMM_INITIALIZE2 => Some(EventType::RaydiumAmmV4Initialize2),
            discriminators::RAYDIUM_AMM_WITHDRAW_PNL => Some(EventType::RaydiumAmmV4WithdrawPnl),
            _ => None,
        },
        program_ids::ORCA_WHIRLPOOL_PROGRAM_ID => match discriminator {
            discriminators::ORCA_TRADED => Some(EventType::OrcaWhirlpoolSwap),
            discriminators::ORCA_LIQUIDITY_INCREASED => {
                Some(EventType::OrcaWhirlpoolLiquidityIncreased)
            }
            discriminators::ORCA_LIQUIDITY_DECREASED => {
                Some(EventType::OrcaWhirlpoolLiquidityDecreased)
            }
            discriminators::ORCA_POOL_INITIALIZED => Some(EventType::OrcaWhirlpoolPoolInitialized),
            _ => None,
        },
        program_ids::METEORA_POOLS_PROGRAM_ID => match discriminator {
            discriminators::METEORA_AMM_SWAP => Some(EventType::MeteoraPoolsSwap),
            discriminators::METEORA_AMM_ADD_LIQUIDITY => Some(EventType::MeteoraPoolsAddLiquidity),
            discriminators::METEORA_AMM_REMOVE_LIQUIDITY => {
                Some(EventType::MeteoraPoolsRemoveLiquidity)
            }
            discriminators::METEORA_AMM_BOOTSTRAP_LIQUIDITY => {
                Some(EventType::MeteoraPoolsBootstrapLiquidity)
            }
            discriminators::METEORA_AMM_POOL_CREATED => Some(EventType::MeteoraPoolsPoolCreated),
            discriminators::METEORA_AMM_SET_POOL_FEES => Some(EventType::MeteoraPoolsSetPoolFees),
            _ => None,
        },
        program_ids::METEORA_DAMM_V2_PROGRAM_ID => match discriminator {
            discriminators::METEORA_DAMM_SWAP | discriminators::METEORA_DAMM_SWAP2 => {
                Some(EventType::MeteoraDammV2Swap)
            }
            discriminators::METEORA_DAMM_ADD_LIQUIDITY => {
                Some(EventType::MeteoraDammV2AddLiquidity)
            }
            discriminators::METEORA_DAMM_REMOVE_LIQUIDITY => {
                Some(EventType::MeteoraDammV2RemoveLiquidity)
            }
            discriminators::METEORA_DAMM_INITIALIZE_POOL => {
                Some(EventType::MeteoraDammV2InitializePool)
            }
            discriminators::METEORA_DAMM_CREATE_POSITION => {
                Some(EventType::MeteoraDammV2CreatePosition)
            }
            discriminators::METEORA_DAMM_CLOSE_POSITION => {
                Some(EventType::MeteoraDammV2ClosePosition)
            }
            _ => None,
        },
        program_ids::METEORA_DBC_PROGRAM_ID => match discriminator {
            discriminators::METEORA_DBC_SWAP => Some(EventType::MeteoraDbcSwap),
            discriminators::METEORA_DBC_INITIALIZE_POOL => {
                Some(EventType::MeteoraDbcInitializePool)
            }
            discriminators::METEORA_DBC_CURVE_COMPLETE => Some(EventType::MeteoraDbcCurveComplete),
            _ => None,
        },
        program_ids::METEORA_DLMM_PROGRAM_ID => match discriminator {
            discriminators::METEORA_DLMM_SWAP => Some(EventType::MeteoraDlmmSwap),
            discriminators::METEORA_DLMM_ADD_LIQUIDITY => Some(EventType::MeteoraDlmmAddLiquidity),
            discriminators::METEORA_DLMM_REMOVE_LIQUIDITY => {
                Some(EventType::MeteoraDlmmRemoveLiquidity)
            }
            discriminators::METEORA_DLMM_INITIALIZE_POOL => {
                Some(EventType::MeteoraDlmmInitializePool)
            }
            discriminators::METEORA_DLMM_INITIALIZE_BIN_ARRAY => {
                Some(EventType::MeteoraDlmmInitializeBinArray)
            }
            discriminators::METEORA_DLMM_CREATE_POSITION => {
                Some(EventType::MeteoraDlmmCreatePosition)
            }
            discriminators::METEORA_DLMM_CLOSE_POSITION => {
                Some(EventType::MeteoraDlmmClosePosition)
            }
            discriminators::METEORA_DLMM_CLAIM_FEE => Some(EventType::MeteoraDlmmClaimFee),
            _ => None,
        },
        _ => None,
    }
}

#[inline(always)]
fn parse_program_scoped_event(
    program_id: &Pubkey,
    discriminator: u64,
    data: &[u8],
    metadata: EventMetadata,
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
) -> Option<DexEvent> {
    if let Some(filter) = event_type_filter {
        if *program_id == program_ids::PUMPFUN_PROGRAM_ID
            && discriminator == discriminators::PUMPFUN_TRADE
        {
            if !filter_wants_pumpfun_trade_event(filter) {
                return None;
            }
        } else if let Some(event_type) =
            program_scoped_discriminator_to_event_type(program_id, discriminator)
        {
            if !filter.should_include(event_type) {
                return None;
            }
        }
    }

    match *program_id {
        program_ids::PUMPFUN_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_pumpfun() {
                    return None;
                }
            }
            match discriminator {
                discriminators::PUMPFUN_TRADE => {
                    let event =
                        crate::logs::pump::parse_trade_from_data(data, metadata, is_created_buy)?;
                    filter_pumpfun_trade_variant(event, event_type_filter)
                }
                discriminators::PUMPFUN_CREATE => {
                    crate::logs::pump::parse_create_from_data(data, metadata)
                }
                discriminators::PUMPFUN_MIGRATE => {
                    crate::logs::pump::parse_migrate_from_data(data, metadata)
                }
                discriminators::PUMPFUN_MIGRATE_BONDING_CURVE_CREATOR => {
                    crate::logs::pump::parse_migrate_bonding_curve_creator_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::PUMP_FEES_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_pump_fees() {
                    return None;
                }
            }
            match discriminator {
                discriminators::PUMP_FEES_CREATE_FEE_SHARING_CONFIG => {
                    crate::logs::pump_fees::parse_create_fee_sharing_config_from_data(
                        data, metadata,
                    )
                }
                discriminators::PUMP_FEES_INITIALIZE_FEE_CONFIG => {
                    crate::logs::pump_fees::parse_initialize_fee_config_from_data(data, metadata)
                }
                discriminators::PUMP_FEES_RESET_FEE_SHARING_CONFIG => {
                    crate::logs::pump_fees::parse_reset_fee_sharing_config_from_data(data, metadata)
                }
                discriminators::PUMP_FEES_REVOKE_FEE_SHARING_AUTHORITY => {
                    crate::logs::pump_fees::parse_revoke_fee_sharing_authority_from_data(
                        data, metadata,
                    )
                }
                discriminators::PUMP_FEES_TRANSFER_FEE_SHARING_AUTHORITY => {
                    crate::logs::pump_fees::parse_transfer_fee_sharing_authority_from_data(
                        data, metadata,
                    )
                }
                discriminators::PUMP_FEES_UPDATE_ADMIN => {
                    crate::logs::pump_fees::parse_update_admin_from_data(data, metadata)
                }
                discriminators::PUMP_FEES_UPDATE_FEE_CONFIG => {
                    crate::logs::pump_fees::parse_update_fee_config_from_data(data, metadata)
                }
                discriminators::PUMP_FEES_UPDATE_FEE_SHARES => {
                    crate::logs::pump_fees::parse_update_fee_shares_from_data(data, metadata)
                }
                discriminators::PUMP_FEES_UPSERT_FEE_TIERS => {
                    crate::logs::pump_fees::parse_upsert_fee_tiers_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::PUMPSWAP_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_pumpswap() {
                    return None;
                }
            }
            match discriminator {
                discriminators::PUMPSWAP_BUY => {
                    crate::logs::pump_amm::parse_buy_from_data(data, metadata)
                }
                discriminators::PUMPSWAP_SELL => {
                    crate::logs::pump_amm::parse_sell_from_data(data, metadata)
                }
                discriminators::PUMPSWAP_CREATE_POOL => {
                    crate::logs::pump_amm::parse_create_pool_from_data(data, metadata)
                }
                discriminators::PUMPSWAP_ADD_LIQUIDITY => {
                    crate::logs::pump_amm::parse_add_liquidity_from_data(data, metadata)
                }
                discriminators::PUMPSWAP_REMOVE_LIQUIDITY => {
                    crate::logs::pump_amm::parse_remove_liquidity_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_raydium_launchlab() {
                    return None;
                }
            }
            match discriminator {
                discriminators::RAYDIUM_LAUNCHLAB_TRADE => {
                    crate::logs::raydium_launchlab::parse_trade_from_data(data, metadata)
                }
                discriminators::RAYDIUM_LAUNCHLAB_POOL_CREATE => {
                    crate::logs::raydium_launchlab::parse_pool_create_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::RAYDIUM_CLMM_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_raydium_clmm() {
                    return None;
                }
            }
            match discriminator {
                discriminators::RAYDIUM_CLMM_SWAP => {
                    crate::logs::raydium_clmm::parse_swap_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_INCREASE_LIQUIDITY => {
                    crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_DECREASE_LIQUIDITY => {
                    crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_LIQUIDITY_CHANGE => {
                    crate::logs::raydium_clmm::parse_liquidity_change_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_CONFIG_CHANGE => {
                    crate::logs::raydium_clmm::parse_config_change_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_CREATE_PERSONAL_POSITION => {
                    crate::logs::raydium_clmm::parse_create_personal_position_from_data(
                        data, metadata,
                    )
                }
                discriminators::RAYDIUM_CLMM_LIQUIDITY_CALCULATE => {
                    crate::logs::raydium_clmm::parse_liquidity_calculate_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_OPEN_LIMIT_ORDER => {
                    crate::logs::raydium_clmm::parse_open_limit_order_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_INCREASE_LIMIT_ORDER => {
                    crate::logs::raydium_clmm::parse_increase_limit_order_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_DECREASE_LIMIT_ORDER => {
                    crate::logs::raydium_clmm::parse_decrease_limit_order_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_SETTLE_LIMIT_ORDER => {
                    crate::logs::raydium_clmm::parse_settle_limit_order_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_UPDATE_REWARD_INFOS => {
                    crate::logs::raydium_clmm::parse_update_reward_infos_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_CREATE_POOL => {
                    crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_COLLECT_PERSONAL_FEE => {
                    crate::logs::raydium_clmm::parse_collect_personal_fee_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CLMM_COLLECT_PROTOCOL_FEE => {
                    crate::logs::raydium_clmm::parse_collect_protocol_fee_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::RAYDIUM_CPMM_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_raydium_cpmm() {
                    return None;
                }
            }
            match discriminator {
                discriminators::RAYDIUM_CPMM_SWAP_BASE_IN => {
                    crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CPMM_SWAP_BASE_OUT => {
                    crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CPMM_CREATE_POOL => {
                    crate::logs::raydium_cpmm::parse_create_pool_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CPMM_DEPOSIT => {
                    crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
                }
                discriminators::RAYDIUM_CPMM_WITHDRAW => {
                    crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::RAYDIUM_AMM_V4_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_raydium_amm_v4() {
                    return None;
                }
            }
            match discriminator {
                discriminators::RAYDIUM_AMM_SWAP_BASE_IN => {
                    crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
                }
                discriminators::RAYDIUM_AMM_SWAP_BASE_OUT => {
                    crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
                }
                discriminators::RAYDIUM_AMM_DEPOSIT => {
                    crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
                }
                discriminators::RAYDIUM_AMM_WITHDRAW => {
                    crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
                }
                discriminators::RAYDIUM_AMM_INITIALIZE2 => {
                    crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
                }
                discriminators::RAYDIUM_AMM_WITHDRAW_PNL => {
                    crate::logs::raydium_amm::parse_withdraw_pnl_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::ORCA_WHIRLPOOL_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_orca_whirlpool() {
                    return None;
                }
            }
            match discriminator {
                discriminators::ORCA_TRADED => {
                    crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
                }
                discriminators::ORCA_LIQUIDITY_INCREASED => {
                    crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
                }
                discriminators::ORCA_LIQUIDITY_DECREASED => {
                    crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
                }
                discriminators::ORCA_POOL_INITIALIZED => {
                    crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::METEORA_POOLS_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_meteora_pools() {
                    return None;
                }
            }
            match discriminator {
                discriminators::METEORA_AMM_SWAP => {
                    crate::logs::meteora_amm::parse_swap_from_data(data, metadata)
                }
                discriminators::METEORA_AMM_ADD_LIQUIDITY => {
                    crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_AMM_REMOVE_LIQUIDITY => {
                    crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_AMM_BOOTSTRAP_LIQUIDITY => {
                    crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_AMM_POOL_CREATED => {
                    crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
                }
                discriminators::METEORA_AMM_SET_POOL_FEES => {
                    crate::logs::meteora_amm::parse_set_pool_fees_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::METEORA_DAMM_V2_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_meteora_damm_v2() {
                    return None;
                }
            }
            match discriminator {
                discriminators::METEORA_DAMM_SWAP => {
                    crate::logs::meteora_damm::parse_swap_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_SWAP2 => {
                    crate::logs::meteora_damm::parse_swap2_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_ADD_LIQUIDITY => {
                    crate::logs::meteora_damm::parse_add_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_REMOVE_LIQUIDITY => {
                    crate::logs::meteora_damm::parse_remove_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_INITIALIZE_POOL => {
                    crate::logs::meteora_damm::parse_initialize_pool_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_CREATE_POSITION => {
                    crate::logs::meteora_damm::parse_create_position_from_data(data, metadata)
                }
                discriminators::METEORA_DAMM_CLOSE_POSITION => {
                    crate::logs::meteora_damm::parse_close_position_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::METEORA_DBC_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_meteora_dbc() {
                    return None;
                }
            }
            match discriminator {
                discriminators::METEORA_DBC_SWAP => {
                    crate::logs::meteora_dbc::parse_swap_from_data(data, metadata)
                }
                discriminators::METEORA_DBC_INITIALIZE_POOL => {
                    crate::logs::meteora_dbc::parse_initialize_pool_from_data(data, metadata)
                }
                discriminators::METEORA_DBC_CURVE_COMPLETE => {
                    crate::logs::meteora_dbc::parse_curve_complete_from_data(data, metadata)
                }
                _ => None,
            }
        }
        program_ids::METEORA_DLMM_PROGRAM_ID => {
            if let Some(filter) = event_type_filter {
                if !filter.includes_meteora_dlmm() {
                    return None;
                }
            }
            match discriminator {
                discriminators::METEORA_DLMM_SWAP => {
                    crate::logs::meteora_dlmm::parse_swap_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_ADD_LIQUIDITY => {
                    crate::logs::meteora_dlmm::parse_add_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_REMOVE_LIQUIDITY => {
                    crate::logs::meteora_dlmm::parse_remove_liquidity_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_INITIALIZE_POOL => {
                    crate::logs::meteora_dlmm::parse_initialize_pool_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_INITIALIZE_BIN_ARRAY => {
                    crate::logs::meteora_dlmm::parse_initialize_bin_array_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_CREATE_POSITION => {
                    crate::logs::meteora_dlmm::parse_create_position_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_CLOSE_POSITION => {
                    crate::logs::meteora_dlmm::parse_close_position_from_data(data, metadata)
                }
                discriminators::METEORA_DLMM_CLAIM_FEE => {
                    crate::logs::meteora_dlmm::parse_claim_fee_from_data(data, metadata)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[inline(always)]
fn filter_pumpfun_trade_variant(
    event: DexEvent,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    if let Some(filter) = event_type_filter {
        if let Some(ref include_only) = filter.include_only {
            let has_specific_filter = !include_only.contains(&EventType::PumpFunTrade)
                && include_only.iter().any(|t| {
                    matches!(
                        t,
                        EventType::PumpFunBuy
                            | EventType::PumpFunSell
                            | EventType::PumpFunBuyExactSolIn
                            | EventType::PumpFunCreate
                            | EventType::PumpFunCreateV2
                    )
                });
            if has_specific_filter {
                let event_type_matches = match &event {
                    DexEvent::PumpFunBuy(_) => include_only.iter().any(|t| {
                        matches!(t, EventType::PumpFunBuy | EventType::PumpFunBuyExactSolIn)
                    }),
                    DexEvent::PumpFunSell(_) => include_only.contains(&EventType::PumpFunSell),
                    DexEvent::PumpFunBuyExactSolIn(_) => include_only.iter().any(|t| {
                        matches!(t, EventType::PumpFunBuy | EventType::PumpFunBuyExactSolIn)
                    }),
                    DexEvent::PumpFunTrade(_) => include_only.contains(&EventType::PumpFunTrade),
                    DexEvent::PumpFunCreate(_) | DexEvent::PumpFunCreateV2(_) => {
                        include_only.iter().any(|t| {
                            matches!(t, EventType::PumpFunCreate | EventType::PumpFunCreateV2)
                        })
                    }
                    _ => false,
                };
                if !event_type_matches {
                    return None;
                }
            }
        }
        if filter.exclude_types.is_some() && !filter.should_include_dex_event(&event) {
            return None;
        }
    }
    Some(event)
}

/// Map discriminator to EventType (compile-time optimized match)
#[inline(always)]
fn discriminator_to_event_type(discriminator: u64) -> Option<EventType> {
    match discriminator {
        discriminators::PUMPFUN_CREATE => Some(EventType::PumpFunCreate),
        discriminators::PUMPFUN_TRADE => Some(EventType::PumpFunTrade),
        discriminators::PUMPFUN_MIGRATE => Some(EventType::PumpFunMigrate),
        discriminators::PUMP_FEES_CREATE_FEE_SHARING_CONFIG => {
            Some(EventType::PumpFeesCreateFeeSharingConfig)
        }
        discriminators::PUMP_FEES_INITIALIZE_FEE_CONFIG => {
            Some(EventType::PumpFeesInitializeFeeConfig)
        }
        discriminators::PUMP_FEES_RESET_FEE_SHARING_CONFIG => {
            Some(EventType::PumpFeesResetFeeSharingConfig)
        }
        discriminators::PUMP_FEES_REVOKE_FEE_SHARING_AUTHORITY => {
            Some(EventType::PumpFeesRevokeFeeSharingAuthority)
        }
        discriminators::PUMP_FEES_TRANSFER_FEE_SHARING_AUTHORITY => {
            Some(EventType::PumpFeesTransferFeeSharingAuthority)
        }
        discriminators::PUMP_FEES_UPDATE_ADMIN => Some(EventType::PumpFeesUpdateAdmin),
        discriminators::PUMP_FEES_UPDATE_FEE_CONFIG => Some(EventType::PumpFeesUpdateFeeConfig),
        discriminators::PUMP_FEES_UPDATE_FEE_SHARES => Some(EventType::PumpFeesUpdateFeeShares),
        discriminators::PUMP_FEES_UPSERT_FEE_TIERS => Some(EventType::PumpFeesUpsertFeeTiers),
        discriminators::PUMPFUN_MIGRATE_BONDING_CURVE_CREATOR => {
            Some(EventType::PumpFunMigrateBondingCurveCreator)
        }
        discriminators::PUMPSWAP_BUY => Some(EventType::PumpSwapBuy),
        discriminators::PUMPSWAP_SELL => Some(EventType::PumpSwapSell),
        discriminators::PUMPSWAP_CREATE_POOL => Some(EventType::PumpSwapCreatePool),
        discriminators::PUMPSWAP_ADD_LIQUIDITY => Some(EventType::PumpSwapLiquidityAdded),
        discriminators::PUMPSWAP_REMOVE_LIQUIDITY => Some(EventType::PumpSwapLiquidityRemoved),
        discriminators::RAYDIUM_LAUNCHLAB_POOL_CREATE => {
            Some(EventType::RaydiumLaunchlabPoolCreate)
        }
        discriminators::RAYDIUM_CLMM_SWAP => Some(EventType::RaydiumClmmSwap),
        discriminators::RAYDIUM_CLMM_INCREASE_LIQUIDITY => {
            Some(EventType::RaydiumClmmIncreaseLiquidity)
        }
        discriminators::RAYDIUM_CLMM_DECREASE_LIQUIDITY => {
            Some(EventType::RaydiumClmmDecreaseLiquidity)
        }
        discriminators::RAYDIUM_CLMM_LIQUIDITY_CHANGE => {
            Some(EventType::RaydiumClmmLiquidityChange)
        }
        discriminators::RAYDIUM_CLMM_CONFIG_CHANGE => Some(EventType::RaydiumClmmConfigChange),
        discriminators::RAYDIUM_CLMM_CREATE_PERSONAL_POSITION => {
            Some(EventType::RaydiumClmmCreatePersonalPosition)
        }
        discriminators::RAYDIUM_CLMM_LIQUIDITY_CALCULATE => {
            Some(EventType::RaydiumClmmLiquidityCalculate)
        }
        discriminators::RAYDIUM_CLMM_OPEN_LIMIT_ORDER => Some(EventType::RaydiumClmmOpenLimitOrder),
        discriminators::RAYDIUM_CLMM_INCREASE_LIMIT_ORDER => {
            Some(EventType::RaydiumClmmIncreaseLimitOrder)
        }
        discriminators::RAYDIUM_CLMM_DECREASE_LIMIT_ORDER => {
            Some(EventType::RaydiumClmmDecreaseLimitOrder)
        }
        discriminators::RAYDIUM_CLMM_SETTLE_LIMIT_ORDER => {
            Some(EventType::RaydiumClmmSettleLimitOrder)
        }
        discriminators::RAYDIUM_CLMM_UPDATE_REWARD_INFOS => {
            Some(EventType::RaydiumClmmUpdateRewardInfos)
        }
        discriminators::RAYDIUM_CLMM_CREATE_POOL => Some(EventType::RaydiumClmmCreatePool),
        discriminators::RAYDIUM_CLMM_COLLECT_PERSONAL_FEE
        | discriminators::RAYDIUM_CLMM_COLLECT_PROTOCOL_FEE => {
            Some(EventType::RaydiumClmmCollectFee)
        }
        discriminators::RAYDIUM_CPMM_SWAP_BASE_IN | discriminators::RAYDIUM_CPMM_SWAP_BASE_OUT => {
            Some(EventType::RaydiumCpmmSwap)
        }
        discriminators::RAYDIUM_CPMM_DEPOSIT => Some(EventType::RaydiumCpmmDeposit),
        discriminators::RAYDIUM_CPMM_WITHDRAW => Some(EventType::RaydiumCpmmWithdraw),
        discriminators::RAYDIUM_AMM_SWAP_BASE_IN | discriminators::RAYDIUM_AMM_SWAP_BASE_OUT => {
            Some(EventType::RaydiumAmmV4Swap)
        }
        discriminators::RAYDIUM_AMM_DEPOSIT => Some(EventType::RaydiumAmmV4Deposit),
        discriminators::RAYDIUM_AMM_WITHDRAW => Some(EventType::RaydiumAmmV4Withdraw),
        discriminators::RAYDIUM_AMM_INITIALIZE2 => Some(EventType::RaydiumAmmV4Initialize2),
        discriminators::RAYDIUM_AMM_WITHDRAW_PNL => Some(EventType::RaydiumAmmV4WithdrawPnl),
        discriminators::ORCA_TRADED => Some(EventType::OrcaWhirlpoolSwap),
        discriminators::ORCA_LIQUIDITY_INCREASED => {
            Some(EventType::OrcaWhirlpoolLiquidityIncreased)
        }
        discriminators::ORCA_LIQUIDITY_DECREASED => {
            Some(EventType::OrcaWhirlpoolLiquidityDecreased)
        }
        discriminators::ORCA_POOL_INITIALIZED => Some(EventType::OrcaWhirlpoolPoolInitialized),
        discriminators::METEORA_AMM_SWAP => Some(EventType::MeteoraPoolsSwap),
        discriminators::METEORA_AMM_ADD_LIQUIDITY => Some(EventType::MeteoraPoolsAddLiquidity),
        discriminators::METEORA_AMM_REMOVE_LIQUIDITY => {
            Some(EventType::MeteoraPoolsRemoveLiquidity)
        }
        discriminators::METEORA_AMM_BOOTSTRAP_LIQUIDITY => {
            Some(EventType::MeteoraPoolsBootstrapLiquidity)
        }
        discriminators::METEORA_AMM_POOL_CREATED => Some(EventType::MeteoraPoolsPoolCreated),
        discriminators::METEORA_AMM_SET_POOL_FEES => Some(EventType::MeteoraPoolsSetPoolFees),
        discriminators::METEORA_DAMM_SWAP | discriminators::METEORA_DAMM_SWAP2 => {
            Some(EventType::MeteoraDammV2Swap)
        }
        discriminators::METEORA_DAMM_ADD_LIQUIDITY => Some(EventType::MeteoraDammV2AddLiquidity),
        discriminators::METEORA_DAMM_REMOVE_LIQUIDITY => {
            Some(EventType::MeteoraDammV2RemoveLiquidity)
        }
        discriminators::METEORA_DAMM_INITIALIZE_POOL => {
            Some(EventType::MeteoraDammV2InitializePool)
        }
        discriminators::METEORA_DAMM_CREATE_POSITION => {
            Some(EventType::MeteoraDammV2CreatePosition)
        }
        discriminators::METEORA_DAMM_CLOSE_POSITION => Some(EventType::MeteoraDammV2ClosePosition),
        _ => None,
    }
}

// ============================================================================
// SIMD utilities for log detection
// ============================================================================
#[inline]
pub fn detect_pumpfun_create(logs: &[String]) -> bool {
    logs.iter().any(|log| PUMPFUN_CREATE_FINDER.find(log.as_bytes()).is_some())
}

/// SIMD 优化的 "invoke [" 查找器
static INVOKE_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"invoke ["));

/// 从日志中解析指令调用信息 (SIMD 优化版本)
/// 返回 (program_id, depth)
#[inline]
pub fn parse_invoke_info(log: &str) -> Option<(&str, usize)> {
    let log_bytes = log.as_bytes();

    // SIMD 快速查找 "invoke ["
    let invoke_start = INVOKE_FINDER.find(log_bytes)?;
    let bracket_start = invoke_start + 8; // "invoke [" 长度

    // 边界检查
    if bracket_start >= log_bytes.len() {
        return None;
    }

    // 解析深度数字，直到遇到 ']'
    let mut depth = 0usize;
    for &byte in &log_bytes[bracket_start..] {
        match byte {
            b'0'..=b'9' => {
                depth = depth * 10 + (byte - b'0') as usize;
            }
            b']' => break,
            _ => return None, // 遇到非数字非']'字符，解析失败
        }
    }

    // 提取程序ID：从 "Program " 开始到 " invoke" 结束
    if invoke_start < 8 {
        return None; // 没有足够空间放 "Program "
    }

    let program_start = 8; // "Program " 的长度
    let program_end = invoke_start - 1; // " invoke" 前面的空格位置

    if program_end <= program_start {
        return None;
    }

    let program_id = std::str::from_utf8(&log_bytes[program_start..program_end]).ok()?;

    Some((program_id, depth))
}

/// Parse `Program <id> success` or `Program <id> failed: ...` completion lines.
#[inline]
pub fn parse_program_complete_info(log: &str) -> Option<&str> {
    let rest = log.strip_prefix("Program ")?;
    if let Some(pos) = rest.find(" success") {
        return Some(&rest[..pos]);
    }
    if let Some(pos) = rest.find(" failed:") {
        return Some(&rest[..pos]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::PumpFunTradeEvent;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    #[test]
    fn program_scoped_launchlab_trade_is_not_parsed_as_pumpfun() {
        let pool = Pubkey::new_unique();
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::RAYDIUM_LAUNCHLAB_TRADE.to_le_bytes());
        raw.extend_from_slice(pool.as_ref());
        for value in 0u64..13 {
            raw.extend_from_slice(&(100 + value).to_le_bytes());
        }
        raw.push(1); // TradeDirection::Sell
        raw.push(2); // PoolStatus::Trade
        raw.push(1); // exact_in

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let filter = EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabTrade]);
        let event = parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&filter),
            false,
            None,
            Some(&program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID),
        )
        .expect("launchlab trade should parse");

        match event {
            DexEvent::RaydiumLaunchlabTrade(trade) => {
                assert_eq!(trade.pool_state, pool);
                assert_eq!(trade.amount_in, 107);
                assert_eq!(trade.amount_out, 108);
                assert!(!trade.is_buy);
                assert!(trade.exact_in);
            }
            other => panic!("expected RaydiumLaunchlabTrade, got {other:?}"),
        }
    }

    #[test]
    fn program_scoped_dlmm_initialize_bin_array_parses_and_filters() {
        let pool = Pubkey::new_unique();
        let bin_array = Pubkey::new_unique();
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::METEORA_DLMM_INITIALIZE_BIN_ARRAY.to_le_bytes());
        raw.extend_from_slice(pool.as_ref());
        raw.extend_from_slice(bin_array.as_ref());
        raw.extend_from_slice(&(-12i64).to_le_bytes());

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let matching_filter =
            EventTypeFilter::include_only(vec![EventType::MeteoraDlmmInitializeBinArray]);
        let event = parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&matching_filter),
            false,
            None,
            Some(&program_ids::METEORA_DLMM_PROGRAM_ID),
        )
        .expect("DLMM initialize bin array should parse");

        match event {
            DexEvent::MeteoraDlmmInitializeBinArray(event) => {
                assert_eq!(event.pool, pool);
                assert_eq!(event.bin_array, bin_array);
                assert_eq!(event.index, -12);
            }
            other => panic!("expected MeteoraDlmmInitializeBinArray, got {other:?}"),
        }

        let non_matching_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDlmmSwap]);
        assert!(parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&non_matching_filter),
            false,
            None,
            Some(&program_ids::METEORA_DLMM_PROGRAM_ID),
        )
        .is_none());
    }

    #[test]
    fn pumpfun_trade_filter_remains_generic_when_combined_with_specific_type() {
        let filter =
            EventTypeFilter::include_only(vec![EventType::PumpFunTrade, EventType::PumpFunBuy]);
        let event = DexEvent::PumpFunSell(PumpFunTradeEvent {
            metadata: EventMetadata::default(),
            is_buy: false,
            ix_name: "sell".to_string(),
            ..Default::default()
        });

        assert!(filter_pumpfun_trade_variant(event, Some(&filter)).is_some());
    }

    #[test]
    fn pumpfun_buy_family_filter_matches_both_buy_variants() {
        let buy = DexEvent::PumpFunBuy(PumpFunTradeEvent {
            metadata: EventMetadata::default(),
            is_buy: true,
            ix_name: "buy_exact_quote_in_v2".to_string(),
            ..Default::default()
        });
        let exact_sol = DexEvent::PumpFunBuyExactSolIn(PumpFunTradeEvent {
            metadata: EventMetadata::default(),
            is_buy: true,
            ix_name: "buy_exact_sol_in".to_string(),
            ..Default::default()
        });

        let buy_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        assert!(filter_pumpfun_trade_variant(buy.clone(), Some(&buy_filter)).is_some());
        assert!(filter_pumpfun_trade_variant(exact_sol.clone(), Some(&buy_filter)).is_some());

        let exact_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuyExactSolIn]);
        assert!(filter_pumpfun_trade_variant(buy, Some(&exact_filter)).is_some());
        assert!(filter_pumpfun_trade_variant(exact_sol, Some(&exact_filter)).is_some());
    }

    #[test]
    fn discriminator_prefix_filter_handles_program_scoped_collisions() {
        let dlmm_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDlmmSwap]);
        assert!(filter_allows_discriminator(
            Some(&program_ids::METEORA_DLMM_PROGRAM_ID),
            discriminators::METEORA_DLMM_SWAP,
            Some(&dlmm_filter),
        ));
        assert!(!filter_allows_discriminator(
            Some(&program_ids::RAYDIUM_CPMM_PROGRAM_ID),
            discriminators::METEORA_DLMM_SWAP,
            Some(&dlmm_filter),
        ));

        let raydium_launchlab_filter =
            EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabTrade]);
        assert!(filter_allows_discriminator(
            Some(&program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID),
            discriminators::RAYDIUM_LAUNCHLAB_TRADE,
            Some(&raydium_launchlab_filter),
        ));
        assert!(!filter_allows_discriminator(
            Some(&program_ids::PUMPFUN_PROGRAM_ID),
            discriminators::PUMPFUN_TRADE,
            Some(&raydium_launchlab_filter),
        ));

        let pumpfun_buy_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        assert!(filter_allows_discriminator(
            Some(&program_ids::PUMPFUN_PROGRAM_ID),
            discriminators::PUMPFUN_TRADE,
            Some(&pumpfun_buy_filter),
        ));

        let dbc_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDbcSwap]);
        assert!(filter_allows_discriminator(
            Some(&program_ids::METEORA_DBC_PROGRAM_ID),
            discriminators::METEORA_DBC_SWAP,
            Some(&dbc_filter),
        ));
        assert!(!filter_allows_discriminator(
            Some(&program_ids::METEORA_DAMM_V2_PROGRAM_ID),
            discriminators::METEORA_DAMM_SWAP,
            Some(&dbc_filter),
        ));
        assert!(!filter_allows_discriminator(
            Some(&program_ids::METEORA_DBC_PROGRAM_ID),
            discriminators::METEORA_DLMM_CLOSE_POSITION,
            Some(&raydium_launchlab_filter),
        ));
    }

    #[test]
    fn program_scoped_pumpfun_buy_filter_parses_trade_log_variant() {
        let log = "Program data: vdt/007mYe5StuUGXKtQJzSLsEK5h79gIdGUQz7vyn59ApMQyeYlr3cK4wUAAAAA7dnMPhkDAAAB5uPeR/hOJigYiGhz2PiTzeNML3vtbrwijyhrJHoTgitivC1qAAAAALjcux8HAAAAt2E0T9e8AwC4MJgjAAAAALfJIQNGvgIA4ATIfOuY+lzkf4A4Bv0seUXSlSSVmuwA3tl4FPOPeEZfAAAAAAAAACBRDgAAAAAAbf5L76S20PsQ+d4EfYrWKDprZOVyf9lJPbA04mYiiiweAAAAAAAAAGmFBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAAAGJ1eQAAAAAAAAAAAAAAAAAAAAAAiBMAAAAAAACQKAcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHcK4wUAAAAAuNy7HwcAAAC4MJgjAAAAAA==";
        let filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        let event = parse_log_optimized_with_program_id(
            log,
            Signature::default(),
            426270756,
            268,
            Some(1781382243600841),
            1781382243601307,
            Some(&filter),
            false,
            None,
            Some(&program_ids::PUMPFUN_PROGRAM_ID),
        )
        .expect("PumpFun TradeEvent log should parse under PumpFunBuy filter");

        match event {
            DexEvent::PumpFunBuy(trade) => {
                assert_eq!(trade.ix_name, "buy");
                assert_eq!(trade.sol_amount, 98_765_431);
                assert_eq!(trade.token_amount, 3_406_962_678_253);
                assert_eq!(trade.virtual_sol_reserves, 30_597_176_504);
                assert_eq!(trade.virtual_token_reserves, 1_052_057_862_955_447);
                assert_eq!(trade.real_sol_reserves, 597_176_504);
                assert_eq!(trade.real_token_reserves, 772_157_862_955_447);
            }
            other => panic!("expected PumpFunBuy, got {other:?}"),
        }
    }

    #[test]
    fn unscoped_pumpfun_buy_filter_parses_trade_log_variant() {
        let log = "Program data: vdt/007mYe5StuUGXKtQJzSLsEK5h79gIdGUQz7vyn59ApMQyeYlr3cK4wUAAAAA7dnMPhkDAAAB5uPeR/hOJigYiGhz2PiTzeNML3vtbrwijyhrJHoTgitivC1qAAAAALjcux8HAAAAt2E0T9e8AwC4MJgjAAAAALfJIQNGvgIA4ATIfOuY+lzkf4A4Bv0seUXSlSSVmuwA3tl4FPOPeEZfAAAAAAAAACBRDgAAAAAAbf5L76S20PsQ+d4EfYrWKDprZOVyf9lJPbA04mYiiiweAAAAAAAAAGmFBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAAAGJ1eQAAAAAAAAAAAAAAAAAAAAAAiBMAAAAAAACQKAcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHcK4wUAAAAAuNy7HwcAAAC4MJgjAAAAAA==";
        let filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        let event = parse_log_optimized(
            log,
            Signature::default(),
            426270756,
            268,
            Some(1781382243600841),
            1781382243601307,
            Some(&filter),
            false,
            None,
        )
        .expect("Unscoped PumpFun TradeEvent log should parse under PumpFunBuy filter");

        match event {
            DexEvent::PumpFunBuy(trade) => {
                assert_eq!(trade.ix_name, "buy");
                assert_eq!(trade.sol_amount, 98_765_431);
                assert_eq!(trade.token_amount, 3_406_962_678_253);
                assert_eq!(trade.virtual_sol_reserves, 30_597_176_504);
                assert_eq!(trade.virtual_token_reserves, 1_052_057_862_955_447);
                assert_eq!(trade.real_sol_reserves, 597_176_504);
                assert_eq!(trade.real_token_reserves, 772_157_862_955_447);
            }
            other => panic!("expected PumpFunBuy, got {other:?}"),
        }
    }

    #[test]
    fn discriminator_prefix_filter_keeps_unscoped_collision_candidates() {
        let dlmm_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDlmmSwap]);
        assert!(filter_allows_discriminator(
            None,
            discriminators::METEORA_DLMM_SWAP,
            Some(&dlmm_filter),
        ));

        let cpmm_filter = EventTypeFilter::include_only(vec![EventType::RaydiumCpmmInitialize]);
        assert!(filter_allows_discriminator(
            None,
            discriminators::RAYDIUM_CPMM_CREATE_POOL,
            Some(&cpmm_filter),
        ));

        let pumpfun_buy_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        assert!(filter_allows_discriminator(
            None,
            discriminators::PUMPFUN_TRADE,
            Some(&pumpfun_buy_filter),
        ));
    }

    #[test]
    fn unscoped_collision_does_not_emit_wrong_protocol_event_after_filter() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::METEORA_DLMM_SWAP.to_le_bytes());
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // CPMM pool_state shape
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // CPMM user shape
        raw.extend_from_slice(&1u64.to_le_bytes());
        raw.extend_from_slice(&2u64.to_le_bytes());
        raw.extend_from_slice(&3u64.to_le_bytes());
        raw.push(1);

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let filter = EventTypeFilter::include_only(vec![EventType::MeteoraDlmmSwap]);
        assert!(parse_log_optimized(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&filter),
            false,
            None,
        )
        .is_none());
    }

    #[test]
    fn unscoped_pumpfun_launchlab_collision_does_not_emit_wrong_protocol_event() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::PUMPFUN_TRADE.to_le_bytes());
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // mint
        raw.extend_from_slice(&1u64.to_le_bytes()); // sol_amount
        raw.extend_from_slice(&2u64.to_le_bytes()); // token_amount
        raw.push(1); // is_buy
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // user
        raw.extend_from_slice(&3i64.to_le_bytes()); // timestamp
        for value in 4u64..=9 {
            raw.extend_from_slice(&value.to_le_bytes());
        }
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // fee_recipient
        raw.extend_from_slice(&10u64.to_le_bytes()); // fee_basis_points
        raw.extend_from_slice(&11u64.to_le_bytes()); // fee
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // creator
        raw.extend_from_slice(&12u64.to_le_bytes()); // creator_fee_basis_points
        raw.extend_from_slice(&13u64.to_le_bytes()); // creator_fee

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let filter = EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabTrade]);
        assert!(parse_log_optimized(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&filter),
            false,
            None,
        )
        .is_none());
    }

    #[test]
    fn discriminator_prefix_decode_reads_first_event_bytes() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::PUMP_FEES_UPDATE_ADMIN.to_le_bytes());
        raw.extend_from_slice(Pubkey::new_unique().as_ref());
        raw.extend_from_slice(Pubkey::new_unique().as_ref());

        let encoded = STANDARD.encode(raw);
        assert_eq!(
            decode_base64_discriminator(&encoded),
            Some(discriminators::PUMP_FEES_UPDATE_ADMIN)
        );
    }

    #[test]
    fn program_scoped_damm_add_liquidity_parses_from_decoded_data() {
        let pool = Pubkey::new_unique();
        let position = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::METEORA_DAMM_ADD_LIQUIDITY.to_le_bytes());
        raw.extend_from_slice(pool.as_ref());
        raw.extend_from_slice(position.as_ref());
        raw.extend_from_slice(owner.as_ref());
        raw.extend_from_slice(&123u128.to_le_bytes());
        raw.extend_from_slice(&10u64.to_le_bytes());
        raw.extend_from_slice(&20u64.to_le_bytes());
        raw.extend_from_slice(&30u64.to_le_bytes());
        raw.extend_from_slice(&40u64.to_le_bytes());
        raw.extend_from_slice(&50u64.to_le_bytes());
        raw.extend_from_slice(&60u64.to_le_bytes());

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let filter = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2AddLiquidity]);
        let event = parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&filter),
            false,
            None,
            Some(&program_ids::METEORA_DAMM_V2_PROGRAM_ID),
        )
        .expect("DAMM V2 add-liquidity should parse");

        match event {
            DexEvent::MeteoraDammV2AddLiquidity(event) => {
                assert_eq!(event.pool, pool);
                assert_eq!(event.position, position);
                assert_eq!(event.owner, owner);
                assert_eq!(event.liquidity_delta, 123);
                assert_eq!(event.token_a_amount, 30);
                assert_eq!(event.token_b_amount, 40);
                assert_eq!(event.total_amount_a, 50);
                assert_eq!(event.total_amount_b, 60);
            }
            other => panic!("expected MeteoraDammV2AddLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn program_scoped_dbc_swap_parses_and_does_not_emit_as_damm() {
        let pool = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::METEORA_DBC_SWAP.to_le_bytes());
        raw.extend_from_slice(pool.as_ref());
        raw.extend_from_slice(config.as_ref());
        raw.push(1);
        raw.push(0);
        raw.extend_from_slice(&100u64.to_le_bytes());
        raw.extend_from_slice(&90u64.to_le_bytes());
        raw.extend_from_slice(&100u64.to_le_bytes());
        raw.extend_from_slice(&95u64.to_le_bytes());
        raw.extend_from_slice(&(1u128 << 64).to_le_bytes());
        raw.extend_from_slice(&2u64.to_le_bytes());
        raw.extend_from_slice(&3u64.to_le_bytes());
        raw.extend_from_slice(&0u64.to_le_bytes());
        raw.extend_from_slice(&100u64.to_le_bytes());
        raw.extend_from_slice(&1_777_920_719u64.to_le_bytes());

        let log = format!("Program data: {}", STANDARD.encode(raw));
        let dbc_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDbcSwap]);
        let event = parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&dbc_filter),
            false,
            None,
            Some(&program_ids::METEORA_DBC_PROGRAM_ID),
        )
        .expect("DBC swap should parse with DBC program context");

        match event {
            DexEvent::MeteoraDbcSwap(event) => {
                assert_eq!(event.pool, pool);
                assert_eq!(event.config, config);
                assert_eq!(event.amount_in, 100);
                assert_eq!(event.output_amount, 95);
            }
            other => panic!("expected MeteoraDbcSwap, got {other:?}"),
        }

        let damm_filter = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2Swap]);
        assert!(parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            Some(&damm_filter),
            false,
            None,
            Some(&program_ids::METEORA_DBC_PROGRAM_ID),
        )
        .is_none());
    }

    #[test]
    fn large_program_data_uses_heap_fallback_without_dropping_event() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::PUMP_FEES_CREATE_FEE_SHARING_CONFIG.to_le_bytes());
        raw.extend_from_slice(&1_777_920_719i64.to_le_bytes());
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // mint
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // bonding_curve
        raw.push(0); // pool: None
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // sharing_config
        raw.extend_from_slice(Pubkey::new_unique().as_ref()); // admin
        raw.extend_from_slice(&64u32.to_le_bytes());
        for i in 0..64u16 {
            raw.extend_from_slice(Pubkey::new_unique().as_ref());
            raw.extend_from_slice(&i.to_le_bytes());
        }
        raw.push(1); // PumpFeesConfigStatus::Active

        let encoded = STANDARD.encode(&raw);
        assert!(encoded.len() > 2700, "test must exceed the old fixed stack buffer limit");
        let log = format!("Program data: {encoded}");

        let event = parse_log_optimized_with_program_id(
            &log,
            Signature::default(),
            1,
            2,
            Some(3),
            4,
            None,
            false,
            None,
            Some(&program_ids::PUMP_FEES_PROGRAM_ID),
        )
        .expect("large pump-fees event should parse via heap fallback");

        match event {
            DexEvent::PumpFeesCreateFeeSharingConfig(event) => {
                assert_eq!(event.initial_shareholders.len(), 64);
                assert_eq!(event.status, crate::core::events::PumpFeesConfigStatus::Active);
            }
            other => panic!("expected PumpFeesCreateFeeSharingConfig, got {other:?}"),
        }
    }

    #[test]
    fn completion_parser_extracts_program_id() {
        assert_eq!(
            parse_program_complete_info(
                "Program LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj success"
            ),
            Some("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj")
        );
        assert_eq!(
            parse_program_complete_info(
                "Program CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C failed: custom program error: 0x1"
            ),
            Some("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C")
        );
    }
}
