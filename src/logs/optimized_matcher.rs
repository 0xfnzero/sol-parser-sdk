//! 优化的日志匹配器 - 高性能日志识别
//!
//! 使用预计算的字符串常量和优化的匹配策略

use crate::core::events::DexEvent;
use solana_sdk::signature::Signature;

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
    Bonk,
    PumpSwap,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumAmmV4,
    OrcaWhirlpool,
    MeteoraPool,
    MeteoraDammV2,
    Unknown,
}

/// 优化的日志类型检测器
#[inline]
pub fn detect_log_type(log: &str) -> LogType {
    // 快速检查：如果没有 "Program" 就跳过
    if !log.contains("Program") {
        return LogType::Unknown;
    }

    // 按使用频率排序，最常见的协议优先检查

    // PumpFun - 最常见
    if log.contains(program_id_strings::PUMPFUN_ID) ||
       (log.contains(program_id_strings::PROGRAM_DATA) && log.len() > 50) {
        return LogType::PumpFun;
    }

    // Raydium AMM V4 - 高频
    if log.contains(program_id_strings::RAYDIUM_AMM_V4_ID) {
        return LogType::RaydiumAmmV4;
    }

    // Raydium CLMM
    if log.contains(program_id_strings::RAYDIUM_CLMM_ID) {
        return LogType::RaydiumClmm;
    }

    // Raydium CPMM
    if log.contains(program_id_strings::RAYDIUM_CPMM_ID) {
        return LogType::RaydiumCpmm;
    }

    // Bonk
    if log.contains(program_id_strings::BONK_ID) ||
       (log.contains("bonk") && log.contains(program_id_strings::PROGRAM_DATA)) {
        return LogType::Bonk;
    }

    // Orca Whirlpool
    if log.contains("whirL") && log.contains(program_id_strings::PROGRAM_DATA) {
        return LogType::OrcaWhirlpool;
    }

    // Meteora DAMM V2
    if log.contains("meteora") && log.contains("LB") {
        return LogType::MeteoraDammV2;
    }

    // Meteora Pools
    if log.contains("meteora") && log.contains(program_id_strings::PROGRAM_DATA) {
        return LogType::MeteoraPool;
    }

    // PumpSwap (与 PumpFun 相似)
    if log.contains("pumpswap") || log.contains("PumpSwap") {
        return LogType::PumpSwap;
    }

    LogType::Unknown
}

/// 优化的统一日志解析器
#[inline]
pub fn parse_log_optimized(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    // 快速类型检测
    let log_type = detect_log_type(log);

    // 根据类型直接调用相应的解析器，避免重复检查
    match log_type {
        LogType::PumpFun => crate::logs::parse_pumpfun_log(log, signature, slot, block_time),
        LogType::Bonk => crate::logs::parse_bonk_log(log, signature, slot, block_time),
        LogType::PumpSwap => crate::logs::parse_pumpswap_log(log, signature, slot, block_time),
        LogType::RaydiumClmm => crate::logs::parse_raydium_clmm_log(log, signature, slot, block_time),
        LogType::RaydiumCpmm => crate::logs::parse_raydium_cpmm_log(log, signature, slot, block_time),
        LogType::RaydiumAmmV4 => crate::logs::parse_raydium_amm_v4_log(log, signature, slot, block_time),
        LogType::OrcaWhirlpool => crate::logs::parse_orca_whirlpool_log(log, signature, slot, block_time),
        LogType::MeteoraPool => crate::logs::parse_meteora_pools_log(log, signature, slot, block_time),
        LogType::MeteoraDammV2 => crate::logs::parse_meteora_damm_v2_log(log, signature, slot, block_time),
        LogType::Unknown => None,
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