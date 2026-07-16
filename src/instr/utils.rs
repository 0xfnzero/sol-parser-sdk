//! 指令解析通用工具函数

use crate::core::events::EventMetadata;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

/// 创建事件元数据的通用函数
pub fn create_metadata(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: i64,
    grpc_recv_us: i64,
) -> EventMetadata {
    EventMetadata { signature, slot, tx_index, block_time_us, grpc_recv_us, recent_blockhash: None }
}

/// 创建事件元数据的兼容性函数（用于指令解析）
#[inline(always)]
pub fn create_metadata_simple(
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    _program_id: Pubkey,
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

/// 从指令数据中读取 u64（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8).map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u32（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4).map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u16（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2).map(|slice| u16::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u8
#[inline(always)]
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 从指令数据中读取 i32（小端序）- SIMD 优化
#[inline(always)]
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    data.get(offset..offset + 4).map(|slice| i32::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取 u128（小端序）- SIMD 优化
#[inline(always)]
pub fn read_u128_le(data: &[u8], offset: usize) -> Option<u128> {
    data.get(offset..offset + 16).map(|slice| u128::from_le_bytes(slice.try_into().unwrap()))
}

/// 从指令数据中读取布尔值
#[inline(always)]
pub fn read_bool(data: &[u8], offset: usize) -> Option<bool> {
    data.get(offset).map(|&b| b != 0)
}

/// IDL 自定义类型 `OptionBool`（Anchor：`struct { bool }`）在 **指令参数** 中与 `bool` 相同，Borsh 仅占 **1 字节**。
/// 勿与 Rust `Option<bool>` 的 Borsh 编码（discriminator + inner，共 2 字节）混淆。
#[inline(always)]
pub fn read_option_bool_idl(data: &[u8], offset: usize) -> Option<bool> {
    match data.get(offset).copied()? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// 从指令数据中读取公钥 - SIMD 优化
#[inline(always)]
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    data.get(offset..offset + 32).and_then(|slice| Pubkey::try_from(slice).ok())
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
pub fn calculate_price_impact_bps(_amount_in: u64, amount_out: u64, expected_out: u64) -> u16 {
    if expected_out == 0 {
        return 0;
    }

    let impact = ((expected_out.saturating_sub(amount_out)) * 10000) / expected_out;
    impact.min(10000) as u16
}

/// Read bytes from instruction data
pub fn read_bytes(data: &[u8], offset: usize, length: usize) -> Option<&[u8]> {
    if data.len() < offset + length {
        return None;
    }
    Some(&data[offset..offset + length])
}

/// `create_v2` 指令 payload（**不含** 8 字节 discriminator）：`name, symbol, uri, creator, is_mayhem_mode, is_cashback_enabled`（IDL）。
/// 其中 `is_cashback_enabled` 为 `OptionBool`，链上与 `bool` 同为 1 字节。
/// `mint` / `bonding_curve` / `user` 在账户里，不在 data 中。
#[inline]
pub fn parse_create_v2_tail_fields(
    data_after_discriminator: &[u8],
) -> Option<(Pubkey, bool, bool)> {
    let mut offset = 0usize;
    let (_, l) = read_str_unchecked(data_after_discriminator, offset)?;
    offset += l;
    let (_, l) = read_str_unchecked(data_after_discriminator, offset)?;
    offset += l;
    let (_, l) = read_str_unchecked(data_after_discriminator, offset)?;
    offset += l;
    if data_after_discriminator.len() < offset + 32 + 1 {
        return None;
    }
    let creator = read_pubkey(data_after_discriminator, offset)?;
    offset += 32;
    let is_mayhem_mode = read_bool(data_after_discriminator, offset)?;
    offset += 1;
    let is_cashback_enabled = if offset < data_after_discriminator.len() {
        read_option_bool_idl(data_after_discriminator, offset).unwrap_or(false)
    } else {
        false
    };
    Some((creator, is_mayhem_mode, is_cashback_enabled))
}

/// Read string with 4-byte length prefix (Borsh format)
/// Returns (string slice, total bytes consumed including length prefix)
#[inline]
pub fn read_str_unchecked(data: &[u8], offset: usize) -> Option<(&str, usize)> {
    if data.len() < offset + 4 {
        return None;
    }
    let len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
    if data.len() < offset + 4 + len {
        return None;
    }
    let string_bytes = &data[offset + 4..offset + 4 + len];
    let s = std::str::from_utf8(string_bytes).ok()?;
    Some((s, 4 + len))
}

/// 从指令数据中读取u64向量（简化版本）
pub fn read_vec_u64(_data: &[u8], _offset: usize) -> Option<Vec<u64>> {
    // 简化版本：返回默认的两个元素向量
    // 实际实现需要根据具体的数据格式来解析
    Some(vec![0, 0])
}

/// 快速读取 Pubkey（从字节数组）
#[inline(always)]
pub fn read_pubkey_fast(bytes: &[u8]) -> Pubkey {
    crate::logs::utils::read_pubkey(bytes, 0).unwrap_or_default()
}

/// 获取指令账户访问器
/// 返回一个可以通过索引获取 Pubkey 的闭包
pub fn get_instruction_account_getter<'a>(
    meta: &'a TransactionStatusMeta,
    transaction: &'a Option<Transaction>,
    account_keys: Option<&'a Vec<Vec<u8>>>,
    // 地址表
    loaded_writable_addresses: &'a [Vec<u8>],
    loaded_readonly_addresses: &'a [Vec<u8>],
    index: &(i32, i32), // (outer_index, inner_index)
) -> Option<impl Fn(usize) -> Pubkey + 'a> {
    // 1. 获取指令的账户索引数组
    let accounts = if index.1 >= 0 {
        // 内层指令 - 使用二分查找优化 (inner_instructions 按 index 升序排列)
        let outer_idx = index.0 as u32;
        meta.inner_instructions
            .binary_search_by_key(&outer_idx, |i| i.index)
            .ok()
            .and_then(|pos| meta.inner_instructions.get(pos))
            .or_else(|| {
                // 回退到线性查找（以防数据未排序）
                meta.inner_instructions.iter().find(|i| i.index == outer_idx)
            })?
            .instructions
            .get(index.1 as usize)?
            .accounts
            .as_slice()
    } else {
        // 外层指令
        transaction
            .as_ref()?
            .message
            .as_ref()?
            .instructions
            .get(index.0 as usize)?
            .accounts
            .as_slice()
    };

    // 2. 创建高性能的账户查找闭包
    Some(move |acc_index: usize| -> Pubkey {
        // 获取账户在交易中的索引
        let account_index = match accounts.get(acc_index) {
            Some(&idx) => idx as usize,
            None => return Pubkey::default(),
        };
        // 早期返回优化
        let Some(keys) = account_keys else {
            return Pubkey::default();
        };
        // 主账户列表
        if let Some(key_bytes) = keys.get(account_index) {
            return read_pubkey_fast(key_bytes);
        }
        // 可写地址
        let writable_offset = account_index.saturating_sub(keys.len());
        if let Some(key_bytes) = loaded_writable_addresses.get(writable_offset) {
            return read_pubkey_fast(key_bytes);
        }
        // 只读地址
        let readonly_offset = writable_offset.saturating_sub(loaded_writable_addresses.len());
        if let Some(key_bytes) = loaded_readonly_addresses.get(readonly_offset) {
            return read_pubkey_fast(key_bytes);
        }
        Pubkey::default()
    })
}

use crate::core::clock::now_us;
/// 预构建的 inner_instructions 索引，用于 O(1) 查找
use std::collections::HashMap;

/// InnerInstructions 索引缓存
pub struct InnerInstructionsIndex<'a> {
    /// outer_index -> &InnerInstructions
    index_map: HashMap<u32, &'a yellowstone_grpc_proto::prelude::InnerInstructions>,
}

impl<'a> InnerInstructionsIndex<'a> {
    /// 从 TransactionStatusMeta 构建索引
    #[inline]
    pub fn new(meta: &'a TransactionStatusMeta) -> Self {
        let mut index_map = HashMap::with_capacity(meta.inner_instructions.len());
        for inner in &meta.inner_instructions {
            index_map.insert(inner.index, inner);
        }
        Self { index_map }
    }

    /// O(1) 查找 inner_instructions
    #[inline]
    pub fn get(
        &self,
        outer_index: u32,
    ) -> Option<&'a yellowstone_grpc_proto::prelude::InnerInstructions> {
        self.index_map.get(&outer_index).copied()
    }
}

/// 使用预构建索引的账户获取器（O(1) 查找）
pub fn get_instruction_account_getter_indexed<'a>(
    inner_index: &InnerInstructionsIndex<'a>,
    transaction: &'a Option<Transaction>,
    account_keys: Option<&'a Vec<Vec<u8>>>,
    loaded_writable_addresses: &'a [Vec<u8>],
    loaded_readonly_addresses: &'a [Vec<u8>],
    index: &(i32, i32),
) -> Option<impl Fn(usize) -> Pubkey + 'a> {
    let accounts = if index.1 >= 0 {
        // O(1) 查找
        inner_index.get(index.0 as u32)?.instructions.get(index.1 as usize)?.accounts.as_slice()
    } else {
        transaction
            .as_ref()?
            .message
            .as_ref()?
            .instructions
            .get(index.0 as usize)?
            .accounts
            .as_slice()
    };

    Some(move |acc_index: usize| -> Pubkey {
        let account_index = match accounts.get(acc_index) {
            Some(&idx) => idx as usize,
            None => return Pubkey::default(),
        };
        let Some(keys) = account_keys else {
            return Pubkey::default();
        };
        if let Some(key_bytes) = keys.get(account_index) {
            return read_pubkey_fast(key_bytes);
        }
        let writable_offset = account_index.saturating_sub(keys.len());
        if let Some(key_bytes) = loaded_writable_addresses.get(writable_offset) {
            return read_pubkey_fast(key_bytes);
        }
        let readonly_offset = writable_offset.saturating_sub(loaded_writable_addresses.len());
        if let Some(key_bytes) = loaded_readonly_addresses.get(readonly_offset) {
            return read_pubkey_fast(key_bytes);
        }
        Pubkey::default()
    })
}

#[cfg(test)]
mod option_bool_tests {
    use super::*;

    #[test]
    fn read_option_bool_idl_strict() {
        assert_eq!(read_option_bool_idl(&[0], 0), Some(false));
        assert_eq!(read_option_bool_idl(&[1], 0), Some(true));
        assert_eq!(read_option_bool_idl(&[2], 0), None);
    }

    #[test]
    fn parse_create_v2_tail_matches_anchor_len() {
        // name "a", "b", "c" + creator (32) + mayhem (1) + OptionBool cashback (1) = 49 bytes payload
        let mut p = Vec::new();
        p.extend_from_slice(&(1u32.to_le_bytes()));
        p.push(b'a');
        p.extend_from_slice(&(1u32.to_le_bytes()));
        p.push(b'b');
        p.extend_from_slice(&(1u32.to_le_bytes()));
        p.push(b'c');
        p.extend_from_slice(&[0u8; 32]);
        p.push(1u8); // mayhem
        p.push(1u8); // cashback
        assert_eq!(p.len(), 49);
        let (creator, mayhem, cb) = parse_create_v2_tail_fields(&p).expect("parse");
        assert_eq!(creator, Pubkey::default());
        assert!(mayhem);
        assert!(cb);
    }
}
