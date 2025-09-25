//! 指令解析通用工具函数

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::EventMetadata;

/// 创建事件元数据的通用函数
pub fn create_metadata(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: i64,
    grpc_recv_us: i64,
) -> EventMetadata {
    EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
    }
}

/// 创建事件元数据的兼容性函数（用于指令解析）
#[inline(always)]
pub fn create_metadata_simple(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
    _program_id: Pubkey,
) -> EventMetadata {
    // 优化：macOS 使用 CLOCK_REALTIME（Linux 可用 CLOCK_REALTIME_COARSE）
    let current_time = unsafe {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        #[cfg(target_os = "linux")]
        libc::clock_gettime(libc::CLOCK_REALTIME_COARSE, &mut ts);
        #[cfg(not(target_os = "linux"))]
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        (ts.tv_sec as i64 * 1_000_000) + (ts.tv_nsec as i64 / 1_000)
    };

    EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time.map_or(0, |t| t * 1_000_000),
        grpc_recv_us: current_time,
    }
}

/// 从指令数据中读取 u64（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8)
        .map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u32（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u16（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2)
        .map(|slice| u16::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u8
#[inline(always)]
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 从指令数据中读取 i32（小端序）- SIMD 优化
#[inline(always)]
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    data.get(offset..offset + 4)
        .map(|slice| i32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u128（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u128_le(data: &[u8], offset: usize) -> Option<u128> {
    data.get(offset..offset + 16)
        .map(|slice| u128::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取布尔值
#[inline(always)]
pub fn read_bool(data: &[u8], offset: usize) -> Option<bool> {
    data.get(offset).map(|&b| b != 0)
}

/// 从指令数据中读取公钥 - SIMD 优化
#[inline(always)]
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    data.get(offset..offset + 32)
        .and_then(|slice| Pubkey::try_from(slice).ok())
}

/// 从账户列表中获取账户
#[inline(always)]
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

/// 从指令数据中读取字节数组
pub fn read_bytes(data: &[u8], offset: usize, length: usize) -> Option<&[u8]> {
    if data.len() < offset + length {
        return None;
    }
    Some(&data[offset..offset + length])
}

/// 从指令数据中读取u64向量（简化版本）
pub fn read_vec_u64(data: &[u8], _offset: usize) -> Option<Vec<u64>> {
    // 简化版本：返回默认的两个元素向量
    // 实际实现需要根据具体的数据格式来解析
    Some(vec![0, 0])
}