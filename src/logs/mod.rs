//! Log parser module
//!
//! Contains log parsers for all DEX protocols

// Allow dead code for fallback text parsers (kept for future use)
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod discriminator_lut;
pub mod meteora_amm;
pub mod meteora_damm;
pub mod meteora_dbc;
pub mod meteora_dlmm;
pub mod optimized_matcher;
pub mod orca_whirlpool;
pub mod perf_hints;
pub mod pump;
pub mod pump_amm;
pub mod pump_fees;
pub mod raydium_amm;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_launchlab;
pub mod utils;
pub mod zero_copy_parser;

// 导出关键的 utils 函数
pub use discriminator_lut::{
    discriminator_to_name, discriminator_to_protocol, lookup_discriminator,
    parse_with_discriminator,
};
pub use utils::extract_discriminator_fast;
pub use zero_copy_parser::parse_pumpfun_trade;

// 重新导出主要解析函数
pub use meteora_amm::parse_log as parse_meteora_amm_log;
pub use meteora_damm::parse_log as parse_meteora_damm_log;
pub use meteora_dbc::parse_log as parse_meteora_dbc_log;
pub use meteora_dlmm::parse_log as parse_meteora_dlmm_log;
pub use orca_whirlpool::parse_log as parse_orca_whirlpool_log;
pub use pump::parse_log as parse_pumpfun_log;
pub use pump_amm::parse_log as parse_pump_amm_log;
pub use raydium_amm::parse_log as parse_raydium_amm_log;
pub use raydium_clmm::parse_log as parse_raydium_clmm_log;
pub use raydium_cpmm::parse_log as parse_raydium_cpmm_log;
pub use raydium_launchlab::parse_log as parse_raydium_launchlab_log;

// 重新导出工具函数
pub use utils::*;

use crate::core::clock::now_us;
use crate::core::events::DexEvent;
use solana_sdk::signature::Signature;

/// 主日志解析入口函数
#[inline(always)] // 零延迟优化：内联热路径
/// `recent_blockhash`: pass as `Some(&buf)` so it is only cloned when an event is produced (not per log line).
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&crate::grpc::types::EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
) -> Option<DexEvent> {
    optimized_matcher::parse_log_optimized(
        log,
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
        event_type_filter,
        is_created_buy,
        recent_blockhash,
    )
}

/// Program-aware log parser for gRPC/RPC transaction logs.
///
/// Solana `Program data:` lines do not include the program id. Passing the
/// current invoke stack's program id prevents cross-protocol discriminator
/// collisions from being misparsed.
#[inline(always)]
pub fn parse_log_with_program_id(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&crate::grpc::types::EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
    program_id: Option<&solana_sdk::pubkey::Pubkey>,
) -> Option<DexEvent> {
    optimized_matcher::parse_log_optimized_with_program_id(
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

/// 统一的日志解析入口函数（优化版本）
#[inline(always)] // 零延迟优化：内联热路径
pub fn parse_log_unified(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let grpc_recv_us = now_us();
    optimized_matcher::parse_log_optimized(
        log,
        signature,
        slot,
        0,
        block_time_us,
        grpc_recv_us,
        None,
        false,
        None,
    )
}
