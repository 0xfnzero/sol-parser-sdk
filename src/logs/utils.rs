//! 日志解析通用工具函数
//!
//! 提供字节数据解析的基础工具，不使用 BorshDeserialize

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::EventMetadata;
use base64::{Engine as _, engine::general_purpose};

/// 从日志中提取程序数据
#[inline]
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    if let Some(data_start) = log.find("Program data: ") {
        let data_part = &log[data_start + 14..];
        general_purpose::STANDARD.decode(data_part.trim()).ok()
    } else {
        None
    }
}

/// 从字节数组中读取 u64（小端序）- SIMD 优化
#[inline]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8)
        .map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u32（小端序）- SIMD 优化
#[inline]
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 i64（小端序）- SIMD 优化
pub fn read_i64_le(data: &[u8], offset: usize) -> Option<i64> {
    data.get(offset..offset + 8)
        .map(|slice| i64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 i32（小端序）- SIMD 优化
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    data.get(offset..offset + 4)
        .map(|slice| i32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u128（小端序）- SIMD 优化
pub fn read_u128_le(data: &[u8], offset: usize) -> Option<u128> {
    data.get(offset..offset + 16)
        .map(|slice| u128::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u16（小端序）- SIMD 优化
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2)
        .map(|slice| u16::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u8
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 从字节数组中读取 Pubkey（32字节）- SIMD 优化
#[inline]
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    data.get(offset..offset + 32)
        .and_then(|slice| {
            let key_bytes: [u8; 32] = slice.try_into().ok()?;
            Some(Pubkey::new_from_array(key_bytes))
        })
}

/// 从字节数组中读取字符串
pub fn read_string(data: &[u8], offset: usize) -> Option<(String, usize)> {
    if data.len() < offset + 4 {
        return None;
    }

    let len = read_u32_le(data, offset)? as usize;
    if data.len() < offset + 4 + len {
        return None;
    }

    let string_bytes = &data[offset + 4..offset + 4 + len];
    // 使用 from_utf8_lossy 避免额外的错误处理和内存分配
    let string = std::str::from_utf8(string_bytes).ok()?.to_string();
    Some((string, 4 + len))
}

/// 读取布尔值
pub fn read_bool(data: &[u8], offset: usize) -> Option<bool> {
    if data.len() <= offset {
        return None;
    }
    Some(data[offset] == 1)
}

/// 创建事件元数据的通用函数
pub fn create_metadata_simple(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    program_id: Pubkey,
    grpc_recv_us: i64,
) -> EventMetadata {
    EventMetadata {
        signature,
        slot,
        tx_index: None,
        block_time_us: block_time.unwrap_or(0) * 1_000_000,
        grpc_recv_us,
    }
}

/// 创建默认事件元数据的通用函数（不需要程序ID）
pub fn create_metadata_default(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
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
        tx_index: None,  // 日志解析无tx_index信息
        block_time_us: block_time.unwrap_or(0) * 1_000_000,
        grpc_recv_us: current_time,
    }
}

/// 文本回退解析工具
pub mod text_parser {

    /// 从文本中提取数字
    pub fn extract_number_from_text(text: &str, field: &str) -> Option<u64> {
        if let Some(start) = text.find(&format!("{}:", field)) {
            let after_colon = &text[start + field.len() + 1..];
            if let Some(end) = after_colon.find(' ').or_else(|| after_colon.find(',')) {
                after_colon[..end].trim().parse().ok()
            } else {
                after_colon.trim().parse().ok()
            }
        } else {
            None
        }
    }

    /// 从文本中提取字段值
    pub fn extract_text_field(text: &str, field: &str) -> Option<String> {
        if let Some(start) = text.find(&format!("{}:", field)) {
            let after_colon = &text[start + field.len() + 1..];
            if let Some(end) = after_colon.find(',').or_else(|| after_colon.find(' ')) {
                Some(after_colon[..end].trim().to_string())
            } else {
                Some(after_colon.trim().to_string())
            }
        } else {
            None
        }
    }

    /// 检测交易类型
    pub fn detect_trade_type(log: &str) -> Option<bool> {
        if log.contains("buy") || log.contains("Buy") {
            Some(true)
        } else if log.contains("sell") || log.contains("Sell") {
            Some(false)
        } else {
            None
        }
    }
}