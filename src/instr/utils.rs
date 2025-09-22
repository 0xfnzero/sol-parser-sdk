//! 指令解析通用工具函数

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::EventMetadata;

/// 创建事件元数据的通用函数
pub fn create_metadata(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    program_id: Pubkey,
) -> EventMetadata {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time.map(|ts| ts * 1000),
        program_id,
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: current_time,
        handle_us: current_time,
    }
}

/// 从指令数据中读取 u64（小端序）
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    if data.len() < offset + 8 {
        return None;
    }
    Some(u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?))
}

/// 从指令数据中读取 u32（小端序）
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    if data.len() < offset + 4 {
        return None;
    }
    Some(u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?))
}

/// 从指令数据中读取 u16（小端序）
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    if data.len() < offset + 2 {
        return None;
    }
    Some(u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?))
}

/// 从指令数据中读取 u8
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 从指令数据中读取 i32（小端序）
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    if data.len() < offset + 4 {
        return None;
    }
    Some(i32::from_le_bytes(data[offset..offset + 4].try_into().ok()?))
}

/// 从指令数据中读取 u128（小端序）
pub fn read_u128_le(data: &[u8], offset: usize) -> Option<u128> {
    if data.len() < offset + 16 {
        return None;
    }
    Some(u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?))
}

/// 从指令数据中读取布尔值
pub fn read_bool(data: &[u8], offset: usize) -> Option<bool> {
    data.get(offset).map(|&b| b != 0)
}

/// 从指令数据中读取公钥
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    if data.len() < offset + 32 {
        return None;
    }
    Pubkey::try_from(&data[offset..offset + 32]).ok()
}

/// 从账户列表中获取账户
pub fn get_account(accounts: &[Pubkey], index: usize) -> Option<Pubkey> {
    accounts.get(index).copied()
}

/// 计算滑点基点
pub fn calculate_slippage_bps(amount_in: u64, amount_out_min: u64) -> u16 {
    if amount_in == 0 {
        return 0;
    }

    // 简化的滑点计算
    let slippage = ((amount_in.saturating_sub(amount_out_min)) * 10000) / amount_in;
    slippage.min(10000) as u16
}

/// 计算价格影响基点
pub fn calculate_price_impact_bps(amount_in: u64, amount_out: u64, expected_out: u64) -> u16 {
    if expected_out == 0 {
        return 0;
    }

    let impact = ((expected_out.saturating_sub(amount_out)) * 10000) / expected_out;
    impact.min(10000) as u16
}