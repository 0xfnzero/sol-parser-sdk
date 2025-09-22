//! 日志解析器模块
//!
//! 包含所有 DEX 协议的日志解析器实现

pub mod utils;
pub mod bonk;
pub mod pumpfun;
pub mod pumpswap;
pub mod raydium_clmm;
pub mod raydium_cpmm;

// 重新导出主要解析函数
pub use bonk::parse_log as parse_bonk_log;
pub use pumpfun::parse_log as parse_pumpfun_log;
pub use pumpswap::parse_log as parse_pumpswap_log;
pub use raydium_clmm::parse_log as parse_raydium_clmm_log;
pub use raydium_cpmm::parse_log as parse_raydium_cpmm_log;

// 重新导出工具函数
pub use utils::*;

use solana_sdk::{signature::Signature};
use crate::core::events::DexEvent;

/// 统一的日志解析入口函数
pub fn parse_log_unified(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    // 尝试按优先级解析不同 DEX 的日志

    // PumpFun
    if pumpfun::is_pumpfun_log(log) {
        if let Some(event) = parse_pumpfun_log(log, signature, slot, block_time) {
            return Some(event);
        }
    }

    // Bonk
    if bonk::is_bonk_log(log) {
        if let Some(event) = parse_bonk_log(log, signature, slot, block_time) {
            return Some(event);
        }
    }

    // PumpSwap
    if pumpswap::is_pumpswap_log(log) {
        if let Some(event) = parse_pumpswap_log(log, signature, slot, block_time) {
            return Some(event);
        }
    }

    // Raydium CLMM
    if raydium_clmm::is_raydium_clmm_log(log) {
        if let Some(event) = parse_raydium_clmm_log(log, signature, slot, block_time) {
            return Some(event);
        }
    }

    // Raydium CPMM
    if raydium_cpmm::is_raydium_cpmm_log(log) {
        if let Some(event) = parse_raydium_cpmm_log(log, signature, slot, block_time) {
            return Some(event);
        }
    }

    None
}