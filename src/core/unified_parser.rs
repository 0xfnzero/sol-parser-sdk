//! 统一解析器 - 简化的单一入口解析器
//!
//! 提供完整的交易解析能力，支持指令和日志数据处理

use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 主要解析函数 - 解析完整交易并返回所有 DEX 事件
///
/// 参数：
/// - instruction_data: 交易指令数据
/// - accounts: 账户列表
/// - logs: 交易日志
/// - signature: 交易签名
/// - slot: 区块高度
/// - block_time: 区块时间
/// - program_id: 程序 ID
pub fn parse_transaction_events(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
    program_id: &Pubkey,
) -> Vec<DexEvent> {
    let mut instruction_events = Vec::new();
    let mut log_events = Vec::new();

    // 1. 解析指令事件
    if let Some(instr_event) = crate::instr::parse_instruction_unified(
        instruction_data, accounts, signature, slot, tx_index, block_time, program_id
    ) {
        instruction_events.push(instr_event);
    }

    // 2. 解析日志事件
    for log in logs {
        if let Some(log_event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
            log_events.push(log_event);
        }
    }

    // 3. 合并指令和日志事件
    instruction_events.extend(log_events);
    instruction_events
}

/// 简化版本 - 仅解析日志事件
pub fn parse_logs_only(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    for log in logs {
        if let Some(event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
            events.push(event);
        }
    }

    events
}

/// 事件监听器 trait - 用户可以实现此 trait 来处理解析出的事件
pub trait EventListener {
    fn on_dex_event(&self, event: &DexEvent);
}

/// 使用监听器解析交易的便捷函数
pub fn parse_transaction_with_listener<T: EventListener>(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
    program_id: &Pubkey,
    listener: &T,
) {
    let events = parse_transaction_events(
        instruction_data, accounts, logs, signature, slot, tx_index, block_time, program_id
    );

    for event in &events {
        listener.on_dex_event(event);
    }
}

/// 流式解析交易事件 - 每解析出一个事件就立即回调
///
/// 这个版本不做事件合并，确保每个事件都能立即被处理
/// 适用于需要实时响应的场景
pub fn parse_transaction_events_streaming<F>(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
    program_id: &Pubkey,
    mut callback: F,
) where
    F: FnMut(DexEvent)
{
    // 1. 先解析指令事件（如果有） - 立即回调
    if let Some(instr_event) = crate::instr::parse_instruction_unified(
        instruction_data, accounts, signature, slot, tx_index, block_time, program_id
    ) {
        callback(instr_event);  // 立即回调指令事件
    }

    // 2. 逐个解析日志事件 - 每个事件立即回调
    for log in logs {
        if let Some(log_event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
            callback(log_event);  // 立即回调日志事件，不等待其他日志
        }
    }

    // 注意：这里完全不做事件合并和缓存，确保每个事件都是立即回调
    // 回调顺序：先指令事件，然后按日志顺序回调日志事件
}

/// 流式解析日志事件 - 每解析出一个事件就立即回调
pub fn parse_logs_streaming<F>(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    mut callback: F,
) where
    F: FnMut(DexEvent)
{
    for log in logs {
        if let Some(event) = crate::logs::parse_log_unified(log, signature, slot, block_time) {
            callback(event);
        }
    }
}

/// 流式事件监听器 trait - 适用于流式处理
pub trait StreamingEventListener {
    fn on_dex_event_streaming(&mut self, event: DexEvent);
}

/// 使用流式监听器解析交易的便捷函数
pub fn parse_transaction_with_streaming_listener<T: StreamingEventListener>(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time: Option<i64>,
    program_id: &Pubkey,
    listener: &mut T,
) {
    parse_transaction_events_streaming(
        instruction_data,
        accounts,
        logs,
        signature,
        slot,
        tx_index,
        block_time,
        program_id,
        |event| listener.on_dex_event_streaming(event)
    );
}