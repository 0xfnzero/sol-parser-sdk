//! 日志解析通用工具函数
//!
//! 提供字节数据解析的基础工具，不使用 BorshDeserialize

use crate::core::clock::now_us;
use crate::core::events::EventMetadata;
#[cfg(target_os = "windows")]
use crate::core::now_micros;
use base64::{engine::general_purpose, Engine as _};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 从日志中提取程序数据（使用 SIMD 优化查找）
#[inline]
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    use memchr::memmem;

    let log_bytes = log.as_bytes();
    let pos = memmem::find(log_bytes, b"Program data: ")?;

    let data_part = &log[pos + 14..];
    general_purpose::STANDARD.decode(data_part.trim()).ok()
}

/// 快速提取 discriminator（只解码前16字节，避免完整解码）
#[inline]
pub fn extract_discriminator_fast(log: &str) -> Option<[u8; 8]> {
    use memchr::memmem;

    let log_bytes = log.as_bytes();
    let pos = memmem::find(log_bytes, b"Program data: ")?;

    let data_part = log[pos + 14..].trim();

    // Base64 编码：每4个字符解码为3个字节
    // 要获取8字节，需要至少 ceil(8/3)*4 = 12 个 base64 字符
    if data_part.len() < 12 {
        return None;
    }

    // 取前16个字符（解码为12字节，包含8字节 discriminator）
    let prefix = &data_part[..16];

    let mut buf = [0u8; 12];
    let decoded_len = general_purpose::STANDARD.decode_slice(prefix.as_bytes(), &mut buf).ok()?;

    if decoded_len >= 8 {
        Some(buf[0..8].try_into().unwrap())
    } else {
        None
    }
}

/// 从字节数组中读取 u64（小端序）- SIMD 优化
#[inline]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8).map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u32（小端序）- SIMD 优化
#[inline]
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4).map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 i64（小端序）- SIMD 优化
pub fn read_i64_le(data: &[u8], offset: usize) -> Option<i64> {
    data.get(offset..offset + 8).map(|slice| i64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 i32（小端序）- SIMD 优化
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    data.get(offset..offset + 4).map(|slice| i32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u128（小端序）- SIMD 优化
pub fn read_u128_le(data: &[u8], offset: usize) -> Option<u128> {
    data.get(offset..offset + 16).map(|slice| u128::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u16（小端序）- SIMD 优化
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2).map(|slice| u16::from_le_bytes(slice.try_into().unwrap()))
}

/// 从字节数组中读取 u8
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 从字节数组中读取 Pubkey（32字节）- SIMD 优化
#[inline]
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    data.get(offset..offset + 32).and_then(|slice| {
        let key_bytes: [u8; 32] = slice.try_into().ok()?;
        Some(Pubkey::new_from_array(key_bytes))
    })
}

/// 从字节数组中读取字符串（分配版本，向后兼容）
pub fn read_string(data: &[u8], offset: usize) -> Option<(String, usize)> {
    let (string_ref, consumed) = read_string_ref(data, offset)?;
    Some((string_ref.to_string(), consumed))
}

/// 从字节数组中读取字符串引用（零拷贝版本）
///
/// ## 零延迟优化
/// 返回 &str 引用而不是 String，避免 50-100ns 的堆分配开销
///
/// ## 用法
/// ```ignore
/// let (name_ref, consumed) = read_string_ref(data, offset)?;
/// // 直接使用引用，无需分配
/// println!("Name: {}", name_ref);
/// ```
#[inline(always)] // 零延迟优化：内联热路径
pub fn read_string_ref(data: &[u8], offset: usize) -> Option<(&str, usize)> {
    if data.len() < offset + 4 {
        return None;
    }

    let len = read_u32_le(data, offset)? as usize;
    if data.len() < offset + 4 + len {
        return None;
    }

    let string_bytes = &data[offset + 4..offset + 4 + len];
    let string_ref = std::str::from_utf8(string_bytes).ok()?; // 零拷贝
    Some((string_ref, 4 + len))
}

/// 读取布尔值
pub fn read_bool(data: &[u8], offset: usize) -> Option<bool> {
    if data.len() <= offset {
        return None;
    }
    Some(data[offset] == 1)
}

/// 将 timestamp (seconds, nanos) 转换为微秒表示
pub fn timestamp_to_microseconds(seconds: i64, nanos: i32) -> i128 {
    // 用 i128 避免溢出
    seconds as i128 * 1_000_000 + (nanos as i128 / 1_000)
}

/// 创建事件元数据的通用函数
pub fn create_metadata_simple(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    program_id: Pubkey,
    grpc_recv_us: i64,
) -> EventMetadata {
    EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    }
}

/// 创建默认事件元数据的通用函数（不需要程序ID）
pub fn create_metadata_default(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> EventMetadata {
    let current_time = now_us();
    EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us: current_time,
        recent_blockhash: None,
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

    /// 从文本中提取字段值（分配版本，向后兼容）
    pub fn extract_text_field(text: &str, field: &str) -> Option<String> {
        extract_text_field_ref(text, field).map(|s| s.to_string())
    }

    /// 从文本中提取字段值引用（零拷贝版本）
    ///
    /// ## 零延迟优化
    /// 返回 &str 引用而不是 String，避免 50-100ns 的堆分配开销
    ///
    /// ## 用法
    /// ```ignore
    /// let value_ref = extract_text_field_ref(log, "amount")?;
    /// let amount: u64 = value_ref.parse().ok()?;
    /// ```
    #[inline(always)] // 零延迟优化：内联热路径
    pub fn extract_text_field_ref<'a>(text: &'a str, field: &str) -> Option<&'a str> {
        let start = text.find(&format!("{}:", field))?;
        let after_colon = &text[start + field.len() + 1..];
        if let Some(end) = after_colon.find(',').or_else(|| after_colon.find(' ')) {
            Some(after_colon[..end].trim())
        } else {
            Some(after_colon.trim())
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
