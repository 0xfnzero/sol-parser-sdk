//! 零拷贝解析器 - 极致性能优化

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use memchr::memmem;
use base64::{Engine as _, engine::general_purpose};
use super::perf_hints::prefetch_read;

/// 零分配 PumpFun Trade 事件解析（栈缓冲区）
#[inline(always)]
pub fn parse_pumpfun_trade(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
    is_created_buy: bool,
) -> Option<DexEvent> {
    // 使用栈缓冲区，避免堆分配（需要足够大以容纳完整的事件数据）
    // PumpFun Trade 事件最大约 350 base64 字符 = 262字节，留出余量用 512 字节
    const MAX_DECODE_SIZE: usize = 512;
    let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

    // SIMD 快速查找 "Program data: "
    let log_bytes = log.as_bytes();
    let pos = memmem::find(log_bytes, b"Program data: ")?;
    let data_part = log[pos + 14..].trim();

    // 快速检查 discriminator（需要至少12个base64字符才能解码出8字节）
    // base64: 每4个字符 = 3个字节，所以12个字符 = 9个字节
    if data_part.len() < 12 {
        return None;
    }

    // 解码 discriminator 到栈缓冲区（12个字符解码为9字节，包含完整8字节discriminator）
    let disc_decoded_len = general_purpose::STANDARD
        .decode_slice(&data_part.as_bytes()[..12], &mut decode_buf[..9])
        .ok()?;

    if disc_decoded_len < 8 {
        return None;
    }

    // 检查是否为 Trade 事件 discriminator
    const TRADE_DISCRIMINATOR: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];

    if decode_buf[..8] != TRADE_DISCRIMINATOR {
        return None;
    }

    // 完整解码事件数据到栈缓冲区
    let decoded_len = general_purpose::STANDARD
        .decode_slice(data_part.as_bytes(), &mut decode_buf)
        .ok()?;

    if decoded_len < 96 {
        return None;
    }

    let data = &decode_buf[8..decoded_len];
    let mut offset = 0;

    // 预取后续数据到 CPU 缓存
    unsafe {
        if data.len() >= 64 {
            prefetch_read(data.as_ptr().add(32));
        }
    }

    // 快速解析字段（内联读取，避免函数调用开销）
    let mint = read_pubkey_inline(data, offset)?;
    offset += 32;

    let sol_amount = read_u64_le_inline(data, offset)?;
    offset += 8;

    let token_amount = read_u64_le_inline(data, offset)?;
    offset += 8;

    let is_buy = read_u8_inline(data, offset)?;
    offset += 1;

    let user = read_pubkey_inline(data, offset)?;
    offset += 32;

    let timestamp = read_i64_le_inline(data, offset)?;
    offset += 8;

    let virtual_sol_reserves = read_u64_le_inline(data, offset)?;
    offset += 8;

    let virtual_token_reserves = read_u64_le_inline(data, offset)?;
    offset += 8;

    let real_sol_reserves = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let real_token_reserves = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let fee_recipient = read_pubkey_inline(data, offset).unwrap_or_default();
    offset += 32;

    let fee_basis_points = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let fee = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let creator = read_pubkey_inline(data, offset).unwrap_or_default();
    offset += 32;

    let creator_fee_basis_points = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let creator_fee = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let track_volume = read_u8_inline(data, offset).unwrap_or(0) != 0;
    offset += 1;

    let total_unclaimed_tokens = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let total_claimed_tokens = read_u64_le_inline(data, offset).unwrap_or(0);
    offset += 8;

    let current_sol_volume = read_u64_le_inline(data, offset).unwrap_or(0);

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index: None,
        block_time_us: block_time.unwrap_or(0) * 1_000_000,
        grpc_recv_us,
    };

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        sol_amount,
        token_amount,
        is_buy: is_buy != 0,
        is_created_buy,
        user,
        timestamp,
        virtual_sol_reserves,
        virtual_token_reserves,
        real_sol_reserves,
        real_token_reserves,
        fee_recipient,
        fee_basis_points,
        fee,
        creator,
        creator_fee_basis_points,
        creator_fee,
        track_volume,
        total_unclaimed_tokens,
        total_claimed_tokens,
        current_sol_volume,
        last_update_timestamp: timestamp,
        // 暂时注释，以后会用，AI禁止改动
        // bonding_curve: Pubkey::default(),
        // associated_bonding_curve: Pubkey::default(),
        // associated_user: Pubkey::default(),
        // global: Pubkey::default(),
    }))
}

/// 内联读取 Pubkey（避免函数调用）
#[inline(always)]
fn read_pubkey_inline(data: &[u8], offset: usize) -> Option<Pubkey> {
    if offset + 32 <= data.len() {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[offset..offset + 32]);
        Some(Pubkey::new_from_array(bytes))
    } else {
        None
    }
}

/// 内联读取 u64（避免函数调用）
#[inline(always)]
fn read_u64_le_inline(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[offset..offset + 8]);
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}

/// 内联读取 i64（避免函数调用）
#[inline(always)]
fn read_i64_le_inline(data: &[u8], offset: usize) -> Option<i64> {
    if offset + 8 <= data.len() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[offset..offset + 8]);
        Some(i64::from_le_bytes(bytes))
    } else {
        None
    }
}

/// 内联读取 u8（避免函数调用）
#[inline(always)]
fn read_u8_inline(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}