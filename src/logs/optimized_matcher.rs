//! 优化的日志匹配器 - 高性能日志识别
//!
//! 使用预计算的字符串常量和优化的匹配策略

use crate::core::events::DexEvent;
use crate::grpc::types::{EventType, EventTypeFilter};
use solana_sdk::signature::Signature;
use memchr::memmem;
use once_cell::sync::Lazy;
use super::perf_hints::{likely, unlikely};

/// SIMD 优化的字符串查找器 - 预编译一次，重复使用
static PUMPFUN_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
static RAYDIUM_AMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
static RAYDIUM_CLMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6"));
static RAYDIUM_CPMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD"));
static BONK_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b"));
static PROGRAM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program"));
static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: "));
static PUMPFUN_CREATE_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: GB7IKAUcB3c"));
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

    pub const BONK_INVOKE: &str = "Program Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b invoke";
    pub const BONK_SUCCESS: &str = "Program Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b success";
    pub const BONK_ID: &str = "Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b";

    pub const RAYDIUM_CLMM_INVOKE: &str = "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 invoke";
    pub const RAYDIUM_CLMM_SUCCESS: &str = "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 success";
    pub const RAYDIUM_CLMM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6";

    pub const RAYDIUM_CPMM_INVOKE: &str = "Program CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD invoke";
    pub const RAYDIUM_CPMM_SUCCESS: &str = "Program CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD success";
    pub const RAYDIUM_CPMM_ID: &str = "CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD";

    pub const RAYDIUM_AMM_V4_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    // 常用的日志模式
    pub const PROGRAM_DATA: &str = "Program data: ";
    pub const PROGRAM_LOG: &str = "Program log: ";

    // PumpFun 事件 discriminator (base64)
    pub const PUMPFUN_CREATE_DISCRIMINATOR: &str = "GB7IKAUcB3c";  // [24, 30, 200, 40, 5, 28, 7, 119]
}

/// 快速日志类型枚举
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LogType {
    PumpFun,
    RaydiumLaunchpad,
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

    // Raydium Launchpad (Bonk)
    if BONK_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumLaunchpad;
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
    if PUMPSWAP_LOWER_FINDER.find(log_bytes).is_some() || PUMPSWAP_UPPER_FINDER.find(log_bytes).is_some() {
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

/// 优化的统一日志解析器（带事件类型过滤）
#[inline(always)]
pub fn parse_log_optimized(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
) -> Option<DexEvent> {
    // 快速类型检测
    let log_type = detect_log_type(log);

    // 提前过滤和解析
    if let Some(filter) = event_type_filter {
        if let Some(ref include_only) = filter.include_only {
            // PumpFun Trade 超快路径（最常见情况）
            if likely(include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade) {
                if likely(log_type == LogType::PumpFun) {
                    // 使用优化解析器：栈分配，无堆分配，内联函数
                    return crate::logs::parse_pumpfun_trade(
                        log, signature, slot, block_time, grpc_recv_us, is_created_buy
                    );
                } else {
                    return None;
                }
            }

            // 提前过滤：如果该协议的所有事件都不在过滤范围内，直接跳过解析
            let should_parse = match log_type {
                LogType::PumpFun => include_only.iter().any(|t| matches!(t,
                    EventType::PumpFunTrade | EventType::PumpFunCreate |
                    EventType::PumpFunComplete | EventType::PumpFunMigrate)),
                LogType::RaydiumAmm => include_only.iter().any(|t| matches!(t,
                    EventType::RaydiumAmmV4Swap | EventType::RaydiumAmmV4Deposit |
                    EventType::RaydiumAmmV4Withdraw | EventType::RaydiumAmmV4Initialize2 |
                    EventType::RaydiumAmmV4WithdrawPnl)),
                LogType::RaydiumClmm => include_only.iter().any(|t| matches!(t,
                    EventType::RaydiumClmmSwap | EventType::RaydiumClmmCreatePool |
                    EventType::RaydiumClmmOpenPosition | EventType::RaydiumClmmClosePosition |
                    EventType::RaydiumClmmIncreaseLiquidity | EventType::RaydiumClmmDecreaseLiquidity |
                    EventType::RaydiumClmmOpenPositionWithTokenExtNft | EventType::RaydiumClmmCollectFee)),
                LogType::RaydiumCpmm => include_only.iter().any(|t| matches!(t,
                    EventType::RaydiumCpmmSwap | EventType::RaydiumCpmmDeposit |
                    EventType::RaydiumCpmmWithdraw | EventType::RaydiumCpmmInitialize)),
                _ => true,
            };

            if unlikely(!should_parse) {
                return None;
            }
        }
    }

    // 根据类型直接调用相应的解析器，传入grpc_recv_us
    let event = match log_type {
        LogType::PumpFun => crate::logs::parse_pumpfun_log(log, signature, slot, block_time, grpc_recv_us, is_created_buy),
        LogType::RaydiumLaunchpad => crate::logs::parse_raydium_launchpad_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::PumpAmm => crate::logs::parse_pump_amm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::RaydiumClmm => crate::logs::parse_raydium_clmm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::RaydiumCpmm => crate::logs::parse_raydium_cpmm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::RaydiumAmm => crate::logs::parse_raydium_amm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::OrcaWhirlpool => crate::logs::parse_orca_whirlpool_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::MeteoraAmm => crate::logs::parse_meteora_amm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::MeteoraDamm => crate::logs::parse_meteora_damm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::MeteoraDlmm => crate::logs::parse_meteora_dlmm_log(log, signature, slot, block_time, grpc_recv_us),
        LogType::Unknown => None,
    };

    // 应用精确的事件类型过滤
    if let Some(event) = event {
        if let Some(filter) = event_type_filter {
            let event_type = match &event {
                DexEvent::PumpFunTrade(_) => EventType::PumpFunTrade,
                DexEvent::PumpFunCreate(_) => EventType::PumpFunCreate,
                DexEvent::PumpFunComplete(_) => EventType::PumpFunComplete,
                DexEvent::PumpFunMigrate(_) => EventType::PumpFunMigrate,
                DexEvent::RaydiumAmmV4Swap(_) => EventType::RaydiumAmmV4Swap,
                DexEvent::RaydiumClmmSwap(_) => EventType::RaydiumClmmSwap,
                DexEvent::RaydiumCpmmSwap(_) => EventType::RaydiumCpmmSwap,
                _ => return Some(event),
            };

            if likely(filter.should_include(event_type)) {
                return Some(event);
            } else {
                return None;
            }
        }
        Some(event)
    } else {
        None
    }
}

/// SIMD 优化的 PumpFun Create 事件检测（扫描所有日志）
#[inline]
pub fn detect_pumpfun_create(logs: &[String]) -> bool {
    logs.iter().any(|log| {
        PUMPFUN_CREATE_FINDER.find(log.as_bytes()).is_some()
    })
}

/// 性能测试辅助函数
#[cfg(test)]
pub mod performance_tests {
    use super::*;
    use std::time::Instant;

    pub fn benchmark_log_detection(log: &str, iterations: usize) -> (u128, u128) {
        // 测试原始方法
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = crate::logs::parse_log_unified(log, Default::default(), 0, None);
        }
        let old_time = start.elapsed().as_nanos();

        // 测试优化方法
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = parse_log_optimized(log, Default::default(), 0, None);
        }
        let new_time = start.elapsed().as_nanos();

        (old_time, new_time)
    }
}