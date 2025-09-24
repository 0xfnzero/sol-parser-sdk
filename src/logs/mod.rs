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
pub mod zero_copy_parser;
pub mod perf_hints;

// 导出关键的 utils 函数
pub use utils::extract_discriminator_fast;
pub use zero_copy_parser::parse_pumpfun_trade;

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

/// 主日志解析入口函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&crate::grpc::types::EventTypeFilter>,
    is_created_buy: bool,
) -> Option<DexEvent> {
    optimized_matcher::parse_log_optimized(log, signature, slot, block_time, grpc_recv_us, event_type_filter, is_created_buy)
}

/// 统一的日志解析入口函数（优化版本）
pub fn parse_log_unified(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let grpc_recv_us = unsafe {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        (ts.tv_sec as i64) * 1_000_000 + (ts.tv_nsec as i64) / 1_000
    };
    optimized_matcher::parse_log_optimized(log, signature, slot, block_time, grpc_recv_us, None, false)
}