//! 指令解析器模块
//!
//! 包含所有 DEX 协议的指令解析器实现

pub mod meteora_amm;
pub mod meteora_damm;
pub mod meteora_dlmm;
pub mod orca_whirlpool;
pub mod program_ids;
pub mod pump;
pub mod pump_amm;
pub mod pump_fees;
pub mod raydium_amm;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_launchlab;
pub mod utils;

// Inner instruction 解析器（16字节 discriminator）
pub mod all_inner;
pub mod inner_common; // 通用零拷贝读取函数
pub mod pump_amm_inner; // PumpSwap inner instruction
pub mod pump_inner; // PumpFun inner instruction
pub mod raydium_clmm_inner; // Raydium CLMM inner instruction // 其他所有协议的 inner instruction（统一文件）
use crate::grpc::types::EventTypeFilter;
// 重新导出主要解析函数
pub use meteora_damm::parse_instruction as parse_meteora_damm_instruction;
pub use pump::parse_instruction as parse_pumpfun_instruction;
pub use pump_amm::parse_instruction as parse_pumpswap_instruction;
pub use raydium_launchlab::parse_instruction as parse_raydium_launchlab_instruction;

// 重新导出工具函数
pub use utils::*;

use crate::core::events::DexEvent;
use program_ids::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[inline(always)]
fn disc8(instruction_data: &[u8]) -> Option<[u8; 8]> {
    instruction_data.get(..8)?.try_into().ok()
}

#[inline(always)]
fn supports_pumpfun_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        pump::discriminators::CREATE
            | pump::discriminators::CREATE_V2
            | pump::discriminators::BUY
            | pump::discriminators::SELL
            | pump::discriminators::BUY_EXACT_SOL_IN
            | pump::discriminators::BUY_V2
            | pump::discriminators::BUY_EXACT_QUOTE_IN_V2
            | pump::discriminators::SELL_V2
    )
}

#[inline(always)]
fn supports_pumpswap_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        pump_amm::discriminators::BUY
            | pump_amm::discriminators::SELL
            | pump_amm::discriminators::CREATE_POOL
            | pump_amm::discriminators::BUY_EXACT_QUOTE_IN
            | pump_amm::discriminators::DEPOSIT
            | pump_amm::discriminators::WITHDRAW
    )
}

#[inline(always)]
fn supports_pump_fees_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        pump_fees::CREATE_FEE_SHARING_IX
            | pump_fees::INITIALIZE_FEE_CONFIG_IX
            | pump_fees::RESET_FEE_SHARING_IX
            | pump_fees::RESET_FEE_SHARING_V2_IX
            | pump_fees::REVOKE_FEE_SHARING_IX
            | pump_fees::TRANSFER_FEE_SHARING_IX
            | pump_fees::UPDATE_ADMIN_IX
            | pump_fees::UPDATE_FEE_CONFIG_IX
            | pump_fees::UPDATE_FEE_SHARES_IX
            | pump_fees::UPDATE_FEE_SHARES_V2_IX
            | pump_fees::UPSERT_FEE_TIERS_IX
    )
}

#[inline(always)]
fn supports_launchlab_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        raydium_launchlab::discriminators::BUY_EXACT_IN
            | raydium_launchlab::discriminators::BUY_EXACT_OUT
            | raydium_launchlab::discriminators::SELL_EXACT_IN
            | raydium_launchlab::discriminators::SELL_EXACT_OUT
            | raydium_launchlab::discriminators::INITIALIZE
            | raydium_launchlab::discriminators::INITIALIZE_V2
            | raydium_launchlab::discriminators::INITIALIZE_WITH_TOKEN_2022
    )
}

#[inline(always)]
fn supports_cpmm_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        raydium_cpmm::discriminators::SWAP_BASE_IN
            | raydium_cpmm::discriminators::SWAP_BASE_OUT
            | raydium_cpmm::discriminators::INITIALIZE
            | raydium_cpmm::discriminators::DEPOSIT
            | raydium_cpmm::discriminators::WITHDRAW
    )
}

#[inline(always)]
fn supports_clmm_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        raydium_clmm::discriminators::SWAP
            | raydium_clmm::discriminators::SWAP_V2
            | raydium_clmm::discriminators::INCREASE_LIQUIDITY_V2
            | raydium_clmm::discriminators::DECREASE_LIQUIDITY_V2
            | raydium_clmm::discriminators::CREATE_POOL
            | raydium_clmm::discriminators::CREATE_CUSTOMIZABLE_POOL
            | raydium_clmm::discriminators::OPEN_POSITION
            | raydium_clmm::discriminators::OPEN_POSITION_V2
            | raydium_clmm::discriminators::OPEN_POSITION_WITH_TOKEN_22_NFT
            | raydium_clmm::discriminators::CLOSE_POSITION
    )
}

#[inline(always)]
fn supports_raydium_amm_v4_instruction(instruction_data: &[u8]) -> bool {
    matches!(
        instruction_data.first().copied(),
        Some(raydium_amm::discriminators::SWAP_BASE_IN)
            | Some(raydium_amm::discriminators::SWAP_BASE_OUT)
            | Some(raydium_amm::discriminators::DEPOSIT)
            | Some(raydium_amm::discriminators::WITHDRAW)
            | Some(raydium_amm::discriminators::INITIALIZE2)
            | Some(raydium_amm::discriminators::WITHDRAW_PNL)
    )
}

#[inline(always)]
fn supports_orca_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        orca_whirlpool::discriminators::SWAP
            | orca_whirlpool::discriminators::SWAP_V2
            | orca_whirlpool::discriminators::INCREASE_LIQUIDITY
            | orca_whirlpool::discriminators::DECREASE_LIQUIDITY
            | orca_whirlpool::discriminators::INITIALIZE_POOL
    )
}

#[inline(always)]
fn supports_meteora_pools_instruction(disc: [u8; 8]) -> bool {
    matches!(
        disc,
        meteora_amm::discriminators::SWAP
            | meteora_amm::discriminators::ADD_LIQUIDITY
            | meteora_amm::discriminators::REMOVE_LIQUIDITY
            | meteora_amm::discriminators::CREATE_POOL
    )
}

#[inline(always)]
fn supports_meteora_damm_v2_instruction(instruction_data: &[u8]) -> bool {
    let Some(disc) = disc8(instruction_data) else {
        return false;
    };
    if disc == meteora_damm::discriminators::INITIALIZE_POOL {
        return true;
    }
    let Some(cpi_disc) = instruction_data.get(8..16).and_then(|bytes| bytes.try_into().ok()) else {
        return false;
    };
    matches!(
        cpi_disc,
        meteora_damm::discriminators::SWAP_LOG
            | meteora_damm::discriminators::SWAP2_LOG
            | meteora_damm::discriminators::CREATE_POSITION_LOG
            | meteora_damm::discriminators::CLOSE_POSITION_LOG
            | meteora_damm::discriminators::ADD_LIQUIDITY_LOG
            | meteora_damm::discriminators::REMOVE_LIQUIDITY_LOG
    )
}

#[inline(always)]
fn supports_meteora_dlmm_instruction(instruction_data: &[u8]) -> bool {
    matches!(instruction_data.first().copied(), Some(0 | 1 | 2 | 7 | 8 | 11 | 13 | 14))
}

#[inline(always)]
pub(crate) fn instruction_data_may_parse(program_id: &Pubkey, instruction_data: &[u8]) -> bool {
    if instruction_data.is_empty() {
        return false;
    }
    if *program_id == RAYDIUM_AMM_V4_PROGRAM_ID {
        return supports_raydium_amm_v4_instruction(instruction_data);
    }
    if *program_id == METEORA_DLMM_PROGRAM_ID {
        return supports_meteora_dlmm_instruction(instruction_data);
    }
    if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        return supports_meteora_damm_v2_instruction(instruction_data);
    }

    let Some(disc) = disc8(instruction_data) else {
        return false;
    };
    if *program_id == PUMPFUN_PROGRAM_ID {
        supports_pumpfun_instruction(disc)
    } else if *program_id == PUMPSWAP_PROGRAM_ID {
        supports_pumpswap_instruction(disc)
    } else if *program_id == PUMP_FEES_PROGRAM_ID {
        supports_pump_fees_instruction(disc)
    } else if *program_id == RAYDIUM_LAUNCHLAB_PROGRAM_ID {
        supports_launchlab_instruction(disc)
    } else if *program_id == RAYDIUM_CPMM_PROGRAM_ID {
        supports_cpmm_instruction(disc)
    } else if *program_id == RAYDIUM_CLMM_PROGRAM_ID {
        supports_clmm_instruction(disc)
    } else if *program_id == ORCA_WHIRLPOOL_PROGRAM_ID {
        supports_orca_instruction(disc)
    } else if *program_id == METEORA_POOLS_PROGRAM_ID {
        supports_meteora_pools_instruction(disc)
    } else {
        false
    }
}

#[inline(always)]
pub(crate) fn normal_instruction_data_may_parse(
    program_id: &Pubkey,
    instruction_data: &[u8],
) -> bool {
    if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        return disc8(instruction_data)
            .is_some_and(|disc| disc == meteora_damm::discriminators::INITIALIZE_POOL);
    }
    instruction_data_may_parse(program_id, instruction_data)
}

#[inline(always)]
fn filter_parsed_event(
    event: Option<DexEvent>,
    event_type_filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    let event = event?;
    if event_type_filter.map(|f| f.should_include_dex_event(&event)).unwrap_or(true) {
        Some(event)
    } else {
        None
    }
}

/// 统一的指令解析入口函数
#[inline]
pub fn parse_instruction_unified(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    program_id: &Pubkey,
) -> Option<DexEvent> {
    // 快速检查指令数据长度，避免无效解析
    if instruction_data.is_empty() {
        return None;
    }

    // 根据程序 ID 路由到相应的解析器，按使用频率排序

    // Pumpfun
    if *program_id == PUMPFUN_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpfun() {
            return None;
        }
        return filter_parsed_event(
            parse_pumpfun_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            ),
            event_type_filter,
        );
    }
    // PumpSwap (Pump AMM)
    else if *program_id == PUMPSWAP_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpswap() {
            return None;
        }
        return filter_parsed_event(
            parse_pumpswap_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Meteora DAMM
    else if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_meteora_damm_v2() {
            return None;
        }
        return filter_parsed_event(
            parse_meteora_damm_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            ),
            event_type_filter,
        );
    }
    // Pump fees (`pfeeUx...`)
    else if *program_id == PUMP_FEES_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pump_fees() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::pump_fees::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            ),
            event_type_filter,
        );
    }
    // RaydiumLaunchlab / Raydium LaunchLab
    else if *program_id == RAYDIUM_LAUNCHLAB_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_raydium_launchlab() {
            return None;
        }
        return filter_parsed_event(
            parse_raydium_launchlab_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Raydium CPMM
    else if *program_id == RAYDIUM_CPMM_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_raydium_cpmm() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::raydium_cpmm::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Raydium CLMM
    else if *program_id == RAYDIUM_CLMM_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_raydium_clmm() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::raydium_clmm::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Raydium AMM V4
    else if *program_id == RAYDIUM_AMM_V4_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_raydium_amm_v4() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::raydium_amm::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Orca Whirlpool
    else if *program_id == ORCA_WHIRLPOOL_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_orca_whirlpool() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::orca_whirlpool::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Meteora Pools / AMM
    else if *program_id == METEORA_POOLS_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_meteora_pools() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::meteora_amm::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }
    // Meteora DLMM
    else if *program_id == METEORA_DLMM_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_meteora_dlmm() {
            return None;
        }
        return filter_parsed_event(
            crate::instr::meteora_dlmm::parse_instruction(
                instruction_data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            ),
            event_type_filter,
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data8(disc: [u8; 8]) -> Vec<u8> {
        let mut data = Vec::from(disc);
        data.extend_from_slice(&1u64.to_le_bytes());
        data.extend_from_slice(&2u64.to_le_bytes());
        data
    }

    #[test]
    fn instruction_data_gate_covers_supported_normal_instruction_protocols() {
        assert!(instruction_data_may_parse(
            &PUMPSWAP_PROGRAM_ID,
            &data8(pump_amm::discriminators::CREATE_POOL)
        ));
        assert!(instruction_data_may_parse(
            &PUMP_FEES_PROGRAM_ID,
            &data8(pump_fees::UPDATE_FEE_SHARES_IX)
        ));
        assert!(instruction_data_may_parse(
            &RAYDIUM_LAUNCHLAB_PROGRAM_ID,
            &data8(raydium_launchlab::discriminators::BUY_EXACT_IN)
        ));
        assert!(instruction_data_may_parse(
            &RAYDIUM_CPMM_PROGRAM_ID,
            &data8(raydium_cpmm::discriminators::SWAP_BASE_IN)
        ));
        assert!(instruction_data_may_parse(
            &RAYDIUM_CLMM_PROGRAM_ID,
            &data8(raydium_clmm::discriminators::SWAP_V2)
        ));
        assert!(instruction_data_may_parse(
            &RAYDIUM_AMM_V4_PROGRAM_ID,
            &[raydium_amm::discriminators::SWAP_BASE_IN, 1, 0, 0, 0, 0, 0, 0, 0]
        ));
        assert!(instruction_data_may_parse(
            &ORCA_WHIRLPOOL_PROGRAM_ID,
            &data8(orca_whirlpool::discriminators::SWAP)
        ));
        assert!(instruction_data_may_parse(
            &METEORA_POOLS_PROGRAM_ID,
            &data8(meteora_amm::discriminators::CREATE_POOL)
        ));
        assert!(instruction_data_may_parse(
            &METEORA_DAMM_V2_PROGRAM_ID,
            &data8(meteora_damm::discriminators::INITIALIZE_POOL)
        ));
        assert!(instruction_data_may_parse(&METEORA_DLMM_PROGRAM_ID, &[11, 1, 2, 3]));
    }

    #[test]
    fn instruction_data_gate_rejects_unknown_program_and_event_cpi_layouts() {
        assert!(!instruction_data_may_parse(&Pubkey::new_unique(), &data8([1; 8])));
        assert!(!instruction_data_may_parse(&PUMPSWAP_PROGRAM_ID, &data8([0xff; 8])));
        assert!(!instruction_data_may_parse(
            &PUMPFUN_PROGRAM_ID,
            &data8(pump::discriminators::MIGRATE_BONDING_CURVE_CREATOR)
        ));

        let mut pumpswap_event_cpi = Vec::from(pump_amm_inner::discriminators::CREATE_POOL);
        pumpswap_event_cpi.extend_from_slice(&[0; 64]);
        assert!(!instruction_data_may_parse(&PUMPSWAP_PROGRAM_ID, &pumpswap_event_cpi));
    }

    #[test]
    fn normal_instruction_gate_keeps_meteora_damm_event_cpi_on_event_path() {
        let mut event_cpi = Vec::new();
        event_cpi.extend_from_slice(&[228, 69, 165, 46, 81, 203, 154, 29]);
        event_cpi.extend_from_slice(&meteora_damm::discriminators::SWAP_LOG);
        event_cpi.extend_from_slice(&[0; 64]);

        assert!(instruction_data_may_parse(&METEORA_DAMM_V2_PROGRAM_ID, &event_cpi));
        assert!(!normal_instruction_data_may_parse(&METEORA_DAMM_V2_PROGRAM_ID, &event_cpi));
        assert!(normal_instruction_data_may_parse(
            &METEORA_DAMM_V2_PROGRAM_ID,
            &data8(meteora_damm::discriminators::INITIALIZE_POOL)
        ));
    }
}
