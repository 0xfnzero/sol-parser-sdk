//! 指令解析器模块
//!
//! 包含所有 DEX 协议的指令解析器实现

pub mod utils;
pub mod program_ids;
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
pub use raydium_launchpad::parse_instruction as parse_raydium_launchpad_instruction;
pub use pumpfun::parse_instruction as parse_pumpfun_instruction;
pub use pump_amm::parse_instruction as parse_pump_amm_instruction;
pub use raydium_clmm::parse_instruction as parse_raydium_clmm_instruction;
pub use raydium_cpmm::parse_instruction as parse_raydium_cpmm_instruction;
pub use raydium_amm::parse_instruction as parse_raydium_amm_instruction;
pub use orca_whirlpool::parse_instruction as parse_orca_whirlpool_instruction;
pub use meteora_amm::parse_instruction as parse_meteora_amm_instruction;
pub use meteora_damm::parse_instruction as parse_meteora_damm_instruction;
pub use meteora_dlmm::parse_instruction as parse_meteora_dlmm_instruction;

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
    tx_index: Option<u64>,
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
        return parse_pumpfun_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Raydium AMM (高频)
    if *program_id == RAYDIUM_AMM_V4_PROGRAM_ID {
        return parse_raydium_amm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Raydium CLMM
    if *program_id == RAYDIUM_CLMM_PROGRAM_ID {
        return parse_raydium_clmm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Orca Whirlpool
    if *program_id == ORCA_WHIRLPOOL_PROGRAM_ID {
        return parse_orca_whirlpool_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Raydium CPMM
    if *program_id == RAYDIUM_CPMM_PROGRAM_ID {
        return parse_raydium_cpmm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Meteora DAMM
    if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        return parse_meteora_damm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Meteora DLMM
    if *program_id == METEORA_DLMM_PROGRAM_ID {
        return parse_meteora_dlmm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Raydium Launchpad
    if *program_id == BONK_PROGRAM_ID {
        return parse_raydium_launchpad_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Pump AMM
    if *program_id == PUMPSWAP_PROGRAM_ID {
        return parse_pump_amm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    // Meteora AMM
    if *program_id == METEORA_POOLS_PROGRAM_ID {
        return parse_meteora_amm_instruction(instruction_data, accounts, signature, slot, tx_index, block_time);
    }

    None
}