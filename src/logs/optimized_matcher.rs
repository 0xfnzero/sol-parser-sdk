//! 优化的日志匹配器 - 高性能日志识别
//!
//! 使用预计算的字符串常量和优化的匹配策略

use crate::core::events::DexEvent;
use crate::grpc::types::{EventType, EventTypeFilter};
use solana_sdk::signature::Signature;
use memchr::memmem;
use once_cell::sync::Lazy;

/// SIMD 优化的字符串查找器 - 预编译一次，重复使用
static PUMPFUN_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
static RAYDIUM_AMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
static RAYDIUM_CLMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6"));
static RAYDIUM_CPMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD"));
static BONK_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b"));
static PROGRAM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program"));
static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program data: "));

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
#[inline]
pub fn detect_log_type(log: &str) -> LogType {
    let log_bytes = log.as_bytes();

    // 第一步：快速长度检查 - 太短的日志直接跳过
    if log_bytes.len() < 20 {
        return LogType::Unknown;
    }

    // 第二步：检查是否有 "Program data:" - 这是事件日志的标志
    let has_program_data = PROGRAM_DATA_FINDER.find(log_bytes).is_some();

    // 只有 "Program data:" 日志才可能是交易事件
    if !has_program_data {
        return LogType::Unknown;
    }

    // 第三步：使用 SIMD 快速检测具体协议
    // PumpFun - 最常见
    // 优先检查程序ID，如果没有ID但有长数据，也可能是PumpFun
    if PUMPFUN_FINDER.find(log_bytes).is_some() {
        return LogType::PumpFun;
    }

    // Raydium AMM - 高频
    if RAYDIUM_AMM_FINDER.find(log_bytes).is_some() {
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

    // 以下协议使用较少，放在后面检查

    // Orca Whirlpool
    if log.contains("whirL") {
        return LogType::OrcaWhirlpool;
    }

    // Meteora - 统一检查前缀
    if let Some(pos) = log.find("meteora") {
        if log[pos..].contains("LB") {
            return LogType::MeteoraDamm;
        } else if log[pos..].contains("DLMM") {
            return LogType::MeteoraDlmm;
        } else {
            return LogType::MeteoraAmm;
        }
    }

    // Pump AMM
    if log.contains("pumpswap") || log.contains("PumpSwap") {
        return LogType::PumpAmm;
    }

    // 兜底：有 "Program data:" 但无法识别协议的，可能是PumpFun
    // PumpFun的日志只有 "Program data: <base64>" 没有其他特征
    if log.len() > 50 {
        return LogType::PumpFun;
    }

    LogType::Unknown
}

/// 优化的统一日志解析器（带事件类型过滤）
#[inline]
pub fn parse_log_optimized(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    // 快速类型检测
    let log_type = detect_log_type(log);

    // 提前过滤和解析
    if let Some(filter) = event_type_filter {
        if let Some(ref include_only) = filter.include_only {
            // PumpFun Trade 专用快速路径
            if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
                if log_type == LogType::PumpFun {
                    return crate::logs::pumpfun::parse_log_fast_filter(
                        log, signature, slot, block_time, grpc_recv_us,
                        crate::logs::pumpfun::discriminators::TRADE_EVENT
                    );
                } else {
                    // 不是 PumpFun 日志，直接跳过
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

            if !should_parse {
                return None;
            }
        }
    }

    // 根据类型直接调用相应的解析器，传入grpc_recv_us
    let event = match log_type {
        LogType::PumpFun => crate::logs::parse_pumpfun_log(log, signature, slot, block_time, grpc_recv_us),
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

            if filter.should_include(event_type) {
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