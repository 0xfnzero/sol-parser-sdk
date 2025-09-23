//! 指令解析器模块
//!
//! 包含所有 DEX 协议的指令解析器实现

pub mod utils;
pub mod program_ids;
pub mod bonk;
pub mod pumpfun;
pub mod pumpswap;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_amm_v4;
pub mod orca_whirlpool;
pub mod meteora_pools;
pub mod meteora_damm_v2;

// 重新导出主要解析函数
pub use bonk::parse_instruction as parse_bonk_instruction;
pub use pumpfun::parse_instruction as parse_pumpfun_instruction;
pub use pumpswap::parse_instruction as parse_pumpswap_instruction;
pub use raydium_clmm::parse_instruction as parse_raydium_clmm_instruction;
pub use raydium_cpmm::parse_instruction as parse_raydium_cpmm_instruction;
pub use raydium_amm_v4::parse_instruction as parse_raydium_amm_v4_instruction;
pub use orca_whirlpool::parse_instruction as parse_orca_whirlpool_instruction;
pub use meteora_pools::parse_instruction as parse_meteora_pools_instruction;
pub use meteora_damm_v2::parse_instruction as parse_meteora_damm_v2_instruction;

// 重新导出工具函数
pub use utils::*;

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::DexEvent;
use program_ids::*;

/// 统一的指令解析入口函数
#[inline]
pub fn parse_instruction_unified(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    program_id: &Pubkey,
) -> Option<DexEvent> {
    // 快速检查指令数据长度，避免无效解析
    if instruction_data.is_empty() {
        return None;
    }

    // 根据程序 ID 路由到相应的解析器，按使用频率排序

    // PumpFun (最常用)
    if *program_id == PUMPFUN_PROGRAM_ID {
        return parse_pumpfun_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Raydium AMM V4 (高频)
    if *program_id == RAYDIUM_AMM_V4_PROGRAM_ID {
        return parse_raydium_amm_v4_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Raydium CLMM
    if *program_id == RAYDIUM_CLMM_PROGRAM_ID {
        return parse_raydium_clmm_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Orca Whirlpool
    if *program_id == ORCA_WHIRLPOOL_PROGRAM_ID {
        return parse_orca_whirlpool_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Raydium CPMM
    if *program_id == RAYDIUM_CPMM_PROGRAM_ID {
        return parse_raydium_cpmm_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Meteora DAMM V2
    if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        return parse_meteora_damm_v2_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Bonk
    if *program_id == BONK_PROGRAM_ID {
        return parse_bonk_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // PumpSwap
    if *program_id == PUMPSWAP_PROGRAM_ID {
        return parse_pumpswap_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    // Meteora Pools
    if *program_id == METEORA_POOLS_PROGRAM_ID {
        return parse_meteora_pools_instruction(instruction_data, accounts, signature, slot, block_time);
    }

    None
}