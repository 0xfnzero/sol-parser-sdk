//! 日志解析器模块
//!
//! 包含所有 DEX 协议的日志解析器实现

pub mod utils;
pub mod optimized_matcher;
pub mod raydium_launchpad;
pub mod pumpfun;
pub mod pump_amm;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_amm;
pub mod orca_whirlpool;
pub mod meteora_amm;
pub mod meteora_damm;
pub mod meteora_dlmm;

// 重新导出主要解析函数
pub use raydium_launchpad::parse_log as parse_raydium_launchpad_log;
pub use pumpfun::parse_log as parse_pumpfun_log;
pub use pump_amm::parse_log as parse_pump_amm_log;
pub use raydium_clmm::parse_log as parse_raydium_clmm_log;
pub use raydium_cpmm::parse_log as parse_raydium_cpmm_log;
pub use raydium_amm::parse_log as parse_raydium_amm_log;
pub use orca_whirlpool::parse_log as parse_orca_whirlpool_log;
pub use meteora_amm::parse_log as parse_meteora_amm_log;
pub use meteora_damm::parse_log as parse_meteora_damm_log;
pub use meteora_dlmm::parse_log as parse_meteora_dlmm_log;

// 重新导出工具函数
pub use utils::*;

use solana_sdk::{signature::Signature};
use crate::core::events::DexEvent;

/// 统一的日志解析入口函数（优化版本，带grpc时间）
pub fn parse_log_unified_with_grpc_time(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    optimized_matcher::parse_log_optimized(log, signature, slot, block_time, grpc_recv_us)
}

/// 统一的日志解析入口函数（优化版本）
pub fn parse_log_unified(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    // 使用当前时间作为grpc_recv_us
    let grpc_recv_us = unsafe {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
    };
    optimized_matcher::parse_log_optimized(log, signature, slot, block_time, grpc_recv_us)
}

/// 传统的日志解析函数（保留用于比较）
pub fn parse_log_unified_legacy(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    // 尝试按优先级解析不同 DEX 的日志

    let grpc_recv_us = unsafe {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
    };

    // PumpFun
    if pumpfun::is_pumpfun_log(log) {
        if let Some(event) = parse_pumpfun_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Raydium Launchpad
    if raydium_launchpad::is_raydium_launchpad_log(log) {
        if let Some(event) = parse_raydium_launchpad_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Pump AMM
    if pump_amm::is_pump_amm_log(log) {
        if let Some(event) = parse_pump_amm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Raydium CLMM
    if raydium_clmm::is_raydium_clmm_log(log) {
        if let Some(event) = parse_raydium_clmm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Raydium CPMM
    if raydium_cpmm::is_raydium_cpmm_log(log) {
        if let Some(event) = parse_raydium_cpmm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Raydium AMM
    if raydium_amm::is_raydium_amm_log(log) {
        if let Some(event) = parse_raydium_amm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Orca Whirlpool
    if orca_whirlpool::is_orca_whirlpool_log(log) {
        if let Some(event) = parse_orca_whirlpool_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Meteora AMM
    if meteora_amm::is_meteora_amm_log(log) {
        if let Some(event) = parse_meteora_amm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Meteora DAMM
    if meteora_damm::is_meteora_damm_log(log) {
        if let Some(event) = parse_meteora_damm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    // Meteora DLMM
    if meteora_dlmm::is_meteora_dlmm_log(log) {
        if let Some(event) = parse_meteora_dlmm_log(log, signature, slot, block_time, grpc_recv_us) {
            return Some(event);
        }
    }

    None
}