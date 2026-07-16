//! 通用 Inner Instruction 解析工具
//!
//! 提供零拷贝、高性能的通用读取函数，供所有协议的 inner instruction 解析器使用

/// 零拷贝读取 u8
///
/// # Safety
///
/// Caller must ensure `offset` is within `data`.
#[inline(always)]
pub unsafe fn read_u8_unchecked(data: &[u8], offset: usize) -> u8 {
    *data.get_unchecked(offset)
}

/// 零拷贝读取 u16
///
/// # Safety
///
/// Caller must ensure `offset..offset + 2` is within `data`.
#[inline(always)]
pub unsafe fn read_u16_unchecked(data: &[u8], offset: usize) -> u16 {
    let ptr = data.as_ptr().add(offset) as *const u16;
    u16::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 u32
///
/// # Safety
///
/// Caller must ensure `offset..offset + 4` is within `data`.
#[inline(always)]
pub unsafe fn read_u32_unchecked(data: &[u8], offset: usize) -> u32 {
    let ptr = data.as_ptr().add(offset) as *const u32;
    u32::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 u64
///
/// # Safety
///
/// Caller must ensure `offset..offset + 8` is within `data`.
#[inline(always)]
pub unsafe fn read_u64_unchecked(data: &[u8], offset: usize) -> u64 {
    let ptr = data.as_ptr().add(offset) as *const u64;
    u64::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 u128
///
/// # Safety
///
/// Caller must ensure `offset..offset + 16` is within `data`.
#[inline(always)]
pub unsafe fn read_u128_unchecked(data: &[u8], offset: usize) -> u128 {
    let ptr = data.as_ptr().add(offset) as *const u128;
    u128::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 i32
///
/// # Safety
///
/// Caller must ensure `offset..offset + 4` is within `data`.
#[inline(always)]
pub unsafe fn read_i32_unchecked(data: &[u8], offset: usize) -> i32 {
    let ptr = data.as_ptr().add(offset) as *const i32;
    i32::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 i64
///
/// # Safety
///
/// Caller must ensure `offset..offset + 8` is within `data`.
#[inline(always)]
pub unsafe fn read_i64_unchecked(data: &[u8], offset: usize) -> i64 {
    let ptr = data.as_ptr().add(offset) as *const i64;
    i64::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 i128
///
/// # Safety
///
/// Caller must ensure `offset..offset + 16` is within `data`.
#[inline(always)]
pub unsafe fn read_i128_unchecked(data: &[u8], offset: usize) -> i128 {
    let ptr = data.as_ptr().add(offset) as *const i128;
    i128::from_le(ptr.read_unaligned())
}

/// 零拷贝读取 bool
///
/// # Safety
///
/// Caller must ensure `offset` is within `data`.
#[inline(always)]
pub unsafe fn read_bool_unchecked(data: &[u8], offset: usize) -> bool {
    *data.get_unchecked(offset) == 1
}

/// 零拷贝读取 Pubkey (32 bytes)
///
/// # Safety
///
/// Caller must ensure `offset..offset + 32` is within `data`.
#[inline(always)]
pub unsafe fn read_pubkey_unchecked(data: &[u8], offset: usize) -> solana_sdk::pubkey::Pubkey {
    use solana_sdk::pubkey::Pubkey;
    let ptr = data.as_ptr().add(offset);
    let mut bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), 32);
    Pubkey::new_from_array(bytes)
}

/// 零拷贝读取字符串（带长度前缀）
///
/// # Safety
///
/// Caller must ensure the length-prefixed bytes are readable and valid UTF-8.
#[inline(always)]
pub unsafe fn read_string_unchecked(data: &[u8], offset: usize) -> Option<(String, usize)> {
    if data.len() < offset + 4 {
        return None;
    }

    let len = read_u32_unchecked(data, offset) as usize;
    if data.len() < offset + 4 + len {
        return None;
    }

    let string_bytes = &data[offset + 4..offset + 4 + len];
    let s = std::str::from_utf8_unchecked(string_bytes);
    Some((s.to_string(), 4 + len))
}

/// 检查数据长度是否足够
#[inline(always)]
pub fn check_length(data: &[u8], required: usize) -> bool {
    data.len() >= required
}
