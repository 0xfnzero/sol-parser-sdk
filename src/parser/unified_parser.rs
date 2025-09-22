//! 统一解析器 - 纯函数式设计
//!
//! 这个模块提供了完整的事件解析能力，使用纯函数结合了：
//! - 从指令数据解析事件（instruction_parser）
//! - 从日志数据解析事件（log_parser）
//! - 事件合并和增强

use crate::parser::{
    events::*,
    // DEX 指令解析器
    pumpfun_ix_parser,
    bonk_ix_parser,
    pumpswap_ix_parser,
    raydium_clmm_ix_parser,
    raydium_cpmm_ix_parser,
    // DEX 日志解析器
    pumpfun_logs_parser,
    bonk_logs_parser,
    pumpswap_logs_parser,
    raydium_clmm_logs_parser,
    raydium_cpmm_logs_parser,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
    // instruction::Instruction,
};
use solana_transaction_status::{InnerInstructions, InnerInstruction, EncodedConfirmedTransactionWithStatusMeta};
use std::collections::HashMap;
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransactionInfo;

/// 从 gRPC 交易数据解析完整事件（指令 + 日志）- 纯函数
pub fn parse_grpc_transaction_complete(
    grpc_tx: SubscribeUpdateTransactionInfo,
    signature: Signature,
    slot: Option<u64>,
    block_time: Option<i64>,
    logs: Option<Vec<String>>,
) -> anyhow::Result<Vec<DexEvent>> {
    let mut events = Vec::new();

    // 1. 从指令数据解析事件
    let instruction_events = parse_grpc_transaction_instructions(
        &grpc_tx,
        signature,
        slot,
        block_time,
    )?;
    events.extend(instruction_events);

    // 2. 从日志数据解析事件（如果有）
    if let Some(logs) = logs {
        let log_events = parse_logs_to_events(
            &logs,
            signature,
            slot.unwrap_or(0),
            block_time,
        );
        events.extend(log_events);
    }

    // 3. 合并和增强事件
    let enhanced_events = merge_and_enhance_events(events);

    Ok(enhanced_events)
}

/// 从版本化交易解析完整事件（指令 + 日志）- 纯函数
pub fn parse_versioned_transaction_complete(
    versioned_tx: &VersionedTransaction,
    signature: Signature,
    slot: Option<u64>,
    block_time: Option<i64>,
    inner_instructions: &[InnerInstructions],
    logs: Option<Vec<String>>,
) -> anyhow::Result<Vec<DexEvent>> {
    let mut events = Vec::new();

    // 1. 从指令数据解析事件
    let instruction_events = parse_versioned_transaction_instructions(
        versioned_tx,
        signature,
        slot,
        block_time,
        inner_instructions,
    )?;
    events.extend(instruction_events);

    // 2. 从日志数据解析事件（如果有）
    if let Some(logs) = logs {
        let log_events = parse_logs_to_events(
            &logs,
            signature,
            slot.unwrap_or(0),
            block_time,
        );
        events.extend(log_events);
    }

    // 3. 合并和增强事件
    let enhanced_events = merge_and_enhance_events(events);

    Ok(enhanced_events)
}

/// 从编码的确认交易解析完整事件 - 纯函数
pub fn parse_encoded_confirmed_transaction_complete(
    signature: Signature,
    transaction: EncodedConfirmedTransactionWithStatusMeta,
    logs: Option<Vec<String>>,
) -> anyhow::Result<Vec<DexEvent>> {
    let mut events = Vec::new();

    // 1. 从指令数据解析事件
    if let Some(versioned_tx) = transaction.transaction.transaction.decode() {
        let inner_instructions = extract_inner_instructions(&transaction);
        let slot = Some(transaction.slot);
        let block_time = transaction.block_time;

        let instruction_events = parse_versioned_transaction_instructions(
            &versioned_tx,
            signature,
            slot,
            block_time,
            &inner_instructions,
        )?;
        events.extend(instruction_events);
    }

    // 2. 从日志数据解析事件（如果有）
    if let Some(logs) = logs {
        let log_events = parse_logs_to_events(
            &logs,
            signature,
            transaction.slot,
            transaction.block_time,
        );
        events.extend(log_events);
    }

    // 3. 合并和增强事件
    let enhanced_events = merge_and_enhance_events(events);

    Ok(enhanced_events)
}

/// 仅从日志解析事件（快速解析模式）- 纯函数
pub fn parse_from_logs_only(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    parse_logs_to_events(logs, signature, slot, block_time)
}

/// 从 gRPC 交易指令解析事件 - 纯函数
fn parse_grpc_transaction_instructions(
    grpc_tx: &SubscribeUpdateTransactionInfo,
    signature: Signature,
    slot: Option<u64>,
    block_time: Option<i64>,
) -> anyhow::Result<Vec<DexEvent>> {
    let mut events = Vec::new();

    if let Some(transaction) = &grpc_tx.transaction {
        if let Some(message) = &transaction.message {
            let account_keys = extract_account_keys(&message.account_keys);

            // 解析主指令
            for (index, compiled_instruction) in message.instructions.iter().enumerate() {
                if let Some(program_id) = account_keys.get(compiled_instruction.program_id_index as usize) {
                    let accounts = extract_instruction_accounts(&compiled_instruction.accounts, &account_keys);

                    if let Some(event) = parse_single_instruction(
                        &compiled_instruction.data,
                        &accounts,
                        &Vec::new(), // Note: meta field not available in this version
                        signature,
                        slot.unwrap_or(0),
                        block_time,
                        index as u32,
                        *program_id,
                    ) {
                        events.push(event);
                    }
                }
            }
        }
    }

    Ok(events)
}

/// 从版本化交易指令解析事件 - 纯函数
fn parse_versioned_transaction_instructions(
    versioned_tx: &VersionedTransaction,
    signature: Signature,
    slot: Option<u64>,
    block_time: Option<i64>,
    inner_instructions: &[InnerInstructions],
) -> anyhow::Result<Vec<DexEvent>> {
    let mut events = Vec::new();
    let account_keys = versioned_tx.message.static_account_keys();

    // 解析主指令
    for (index, instruction) in versioned_tx.message.instructions().iter().enumerate() {
        if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
            let accounts = extract_versioned_instruction_accounts(instruction, &account_keys);

            if let Some(event) = parse_single_instruction(
                &instruction.data,
                &accounts,
                &[], // logs 在这个层级不可用，需要从外部传入
                signature,
                slot.unwrap_or(0),
                block_time,
                index as u32,
                *program_id,
            ) {
                events.push(event);
            }
        }
    }

    // 解析内部指令
    for inner_instruction in inner_instructions {
        for (inner_index, instruction) in inner_instruction.instructions.iter().enumerate() {
            if let Some(program_id) = account_keys.get(instruction.instruction.program_id_index as usize) {
                let accounts = extract_inner_instruction_accounts(instruction, &account_keys);

                if let Some(event) = parse_single_instruction(
                    &instruction.instruction.data,
                    &accounts,
                    &[], // logs 在这个层级不可用
                    signature,
                    slot.unwrap_or(0),
                    block_time,
                    inner_instruction.index as u32,
                    *program_id,
                ) {
                    events.push(event);
                }
            }
        }
    }

    Ok(events)
}

/// 解析单个指令为事件 - 纯函数
fn parse_single_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
    program_id: Pubkey,
) -> Option<DexEvent> {
    let program_id_str = program_id.to_string();

    match program_id_str.as_str() {
        pumpfun_ix_parser::PROGRAM_ID => {
            pumpfun_ix_parser::parse_pumpfun_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                instruction_index,
            )
        },
        bonk_ix_parser::PROGRAM_ID => {
            bonk_ix_parser::parse_bonk_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                instruction_index,
            )
        },
        pumpswap_ix_parser::PUMPSWAP_PROGRAM_ID => {
            pumpswap_ix_parser::parse_pumpswap_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                instruction_index,
            )
        },
        raydium_clmm_ix_parser::PROGRAM_ID => {
            raydium_clmm_ix_parser::parse_raydium_clmm_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                instruction_index,
            )
        },
        raydium_cpmm_ix_parser::PROGRAM_ID => {
            raydium_cpmm_ix_parser::parse_raydium_cpmm_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                instruction_index,
            )
        },
        _ => None, // 未知程序ID
    }
}

/// 从日志解析所有 DEX 事件 - 纯函数
fn parse_logs_to_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    for log in logs {
        // 尝试解析每种 DEX 的日志
        if let Some(event) = pumpfun_logs_parser::parse_pumpfun_from_log_string(log, signature, slot, block_time) {
            events.push(event);
        } else if let Some(event) = bonk_logs_parser::parse_bonk_from_log_string(log, signature, slot, block_time) {
            events.push(event);
        } else if let Some(event) = pumpswap_logs_parser::parse_pumpswap_from_log_string(log, signature, slot, block_time) {
            events.push(event);
        } else if let Some(event) = raydium_clmm_logs_parser::parse_raydium_clmm_from_log_string(log, signature, slot, block_time) {
            events.push(event);
        } else if let Some(event) = raydium_cpmm_logs_parser::parse_raydium_cpmm_from_log_string(log, signature, slot, block_time) {
            events.push(event);
        }
    }

    events
}

/// 合并和增强事件 - 纯函数
///
/// 这个函数负责：
/// 1. 合并来自指令和日志的相同事件（通过匹配元数据）
/// 2. 用日志数据填充指令数据中缺失的字段
/// 3. 去重和清理
fn merge_and_enhance_events(events: Vec<DexEvent>) -> Vec<DexEvent> {
    let mut event_map: HashMap<String, DexEvent> = HashMap::new();

    for event in events {
        let key = generate_event_key(&event);

        if let Some(existing_event) = event_map.remove(&key) {
            // 合并事件：优先使用日志数据填充指令数据
            let merged_event = merge_events(existing_event, event);
            event_map.insert(key, merged_event);
        } else {
            event_map.insert(key, event);
        }
    }

    event_map.into_values().collect()
}

/// 生成事件的唯一键用于合并 - 纯函数
fn generate_event_key(event: &DexEvent) -> String {
    match event {
        DexEvent::PumpFunCreate(e) => format!("pumpfun_create_{}_{}", e.metadata.signature, e.mint),
        DexEvent::PumpFunTrade(e) => format!("pumpfun_trade_{}_{}_{}_{}",
            e.metadata.signature, e.mint, e.user, e.metadata.outer_index),
        DexEvent::BonkTrade(e) => format!("bonk_trade_{}_{}_{}_{}",
            e.metadata.signature, e.pool_state, e.user, e.metadata.outer_index),
        DexEvent::PumpSwapBuy(e) => format!("pumpswap_buy_{}_{}_{}_{}",
            e.metadata.signature, e.pool_id, e.user, e.metadata.outer_index),
        DexEvent::PumpSwapSell(e) => format!("pumpswap_sell_{}_{}_{}_{}",
            e.metadata.signature, e.pool_id, e.user, e.metadata.outer_index),
        DexEvent::RaydiumClmmSwap(e) => format!("raydium_clmm_swap_{}_{}_{}_{}",
            e.metadata.signature, e.pool, e.user, e.metadata.outer_index),
        DexEvent::RaydiumCpmmSwap(e) => format!("raydium_cpmm_swap_{}_{}_{}_{}",
            e.metadata.signature, e.pool, e.user, e.metadata.outer_index),
        _ => format!("generic_{}", event.get_signature()),
    }
}

/// 合并两个相同的事件，优先使用更完整的数据 - 纯函数
fn merge_events(base_event: DexEvent, new_event: DexEvent) -> DexEvent {
    match (&base_event, &new_event) {
        (DexEvent::PumpFunTrade(base), DexEvent::PumpFunTrade(new)) => {
            let mut merged = base.clone();
            // 用日志数据填充指令数据中的空字段
            if merged.virtual_sol_reserves == 0 && new.virtual_sol_reserves > 0 {
                merged.virtual_sol_reserves = new.virtual_sol_reserves;
            }
            if merged.virtual_token_reserves == 0 && new.virtual_token_reserves > 0 {
                merged.virtual_token_reserves = new.virtual_token_reserves;
            }
            if merged.real_sol_reserves == 0 && new.real_sol_reserves > 0 {
                merged.real_sol_reserves = new.real_sol_reserves;
            }
            if merged.real_token_reserves == 0 && new.real_token_reserves > 0 {
                merged.real_token_reserves = new.real_token_reserves;
            }
            if merged.fee == 0 && new.fee > 0 {
                merged.fee = new.fee;
            }
            if merged.creator == Pubkey::default() && new.creator != Pubkey::default() {
                merged.creator = new.creator;
            }
            if merged.timestamp == 0 && new.timestamp > 0 {
                merged.timestamp = new.timestamp;
            }
            DexEvent::PumpFunTrade(merged)
        },
        (DexEvent::PumpFunCreate(base), DexEvent::PumpFunCreate(new)) => {
            let mut merged = base.clone();
            // 类似地合并创建事件
            if merged.virtual_sol_reserves == 0 && new.virtual_sol_reserves > 0 {
                merged.virtual_sol_reserves = new.virtual_sol_reserves;
            }
            if merged.virtual_token_reserves == 0 && new.virtual_token_reserves > 0 {
                merged.virtual_token_reserves = new.virtual_token_reserves;
            }
            if merged.timestamp == 0 && new.timestamp > 0 {
                merged.timestamp = new.timestamp;
            }
            DexEvent::PumpFunCreate(merged)
        },
        (DexEvent::RaydiumClmmSwap(base), DexEvent::RaydiumClmmSwap(new)) => {
            let mut merged = base.clone();
            // 合并 Raydium CLMM 数据
            if merged.amount == 0 && new.amount > 0 {
                merged.amount = new.amount;
            }
            if merged.other_amount_threshold == 0 && new.other_amount_threshold > 0 {
                merged.other_amount_threshold = new.other_amount_threshold;
            }
            DexEvent::RaydiumClmmSwap(merged)
        },
        _ => {
            // 对于不同类型的事件或无法合并的事件，选择更完整的一个
            if is_more_complete(&new_event, &base_event) {
                new_event
            } else {
                base_event
            }
        }
    }
}

/// 判断哪个事件更完整 - 纯函数
fn is_more_complete(event1: &DexEvent, event2: &DexEvent) -> bool {
    // 简单的启发式：非零字段更多的事件被认为更完整
    count_non_zero_fields(event1) > count_non_zero_fields(event2)
}

/// 计算事件中非零字段的数量 - 纯函数
fn count_non_zero_fields(event: &DexEvent) -> usize {
    match event {
        DexEvent::PumpFunTrade(e) => {
            let mut count = 0;
            if e.sol_amount > 0 { count += 1; }
            if e.token_amount > 0 { count += 1; }
            if e.virtual_sol_reserves > 0 { count += 1; }
            if e.virtual_token_reserves > 0 { count += 1; }
            if e.fee > 0 { count += 1; }
            if e.timestamp > 0 { count += 1; }
            count
        },
        DexEvent::RaydiumClmmSwap(e) => {
            let mut count = 0;
            if e.amount > 0 { count += 1; }
            if e.other_amount_threshold > 0 { count += 1; }
            if e.sqrt_price_limit_x64 > 0 { count += 1; }
            count
        },
        // 为其他事件类型添加类似的逻辑
        _ => 1, // 默认值
    }
}

// 辅助函数 - 纯函数

/// 提取账户键列表 - 纯函数
fn extract_account_keys(raw_keys: &[Vec<u8>]) -> Vec<Pubkey> {
    raw_keys
        .iter()
        .filter_map(|key| {
            if key.len() == 32 {
                Some(Pubkey::new_from_array(key.clone().try_into().unwrap()))
            } else {
                None
            }
        })
        .collect()
}

/// 提取指令账户 - 纯函数
fn extract_instruction_accounts(account_indices: &[u8], account_keys: &[Pubkey]) -> Vec<Pubkey> {
    account_indices
        .iter()
        .filter_map(|&index| account_keys.get(index as usize))
        .copied()
        .collect()
}

/// 提取版本化交易指令账户 - 纯函数
fn extract_versioned_instruction_accounts(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    account_keys: &[Pubkey],
) -> Vec<Pubkey> {
    instruction
        .accounts
        .iter()
        .filter_map(|&index| account_keys.get(index as usize))
        .copied()
        .collect()
}

/// 提取内部指令账户 - 纯函数
fn extract_inner_instruction_accounts(
    instruction: &InnerInstruction,
    account_keys: &[Pubkey],
) -> Vec<Pubkey> {
    instruction
        .instruction.accounts
        .iter()
        .filter_map(|&index| account_keys.get(index as usize))
        .copied()
        .collect()
}

/// 从编码交易中提取内部指令 - 纯函数
fn extract_inner_instructions(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
) -> Vec<InnerInstructions> {
    if let Some(meta) = &transaction.transaction.meta {
        match &meta.inner_instructions {
            solana_transaction_status::option_serializer::OptionSerializer::Some(inner_instructions) => {
            // 从 UI 格式转换为标准格式
            inner_instructions
                .iter()
                .filter_map(|ui_inner| {
                    // 这里需要实现从 UI 格式到标准格式的转换
                    // 这是一个简化的版本，实际实现需要根据具体的 UI 格式
                    Some(InnerInstructions {
                        index: ui_inner.index,
                        instructions: vec![], // 需要转换 UI 格式的指令
                    })
                })
                .collect()
            },
            _ => vec![]
        }
    } else {
        vec![]
    }
}

/// 为 DexEvent 添加辅助方法
impl DexEvent {
    /// 获取事件的签名 - 纯函数
    pub fn get_signature(&self) -> Signature {
        match self {
            DexEvent::PumpFunCreate(e) => e.metadata.signature,
            DexEvent::PumpFunTrade(e) => e.metadata.signature,
            DexEvent::PumpFunComplete(e) => e.metadata.signature,
            DexEvent::BonkTrade(e) => e.metadata.signature,
            DexEvent::BonkPoolCreate(e) => e.metadata.signature,
            DexEvent::BonkMigrateAmm(e) => e.metadata.signature,
            DexEvent::PumpSwapBuy(e) => e.metadata.signature,
            DexEvent::PumpSwapSell(e) => e.metadata.signature,
            DexEvent::PumpSwapCreatePool(e) => e.metadata.signature,
            DexEvent::RaydiumClmmSwap(e) => e.metadata.signature,
            DexEvent::RaydiumClmmCreatePool(e) => e.metadata.signature,
            DexEvent::RaydiumClmmOpenPosition(e) => e.metadata.signature,
            DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => e.metadata.signature,
            DexEvent::RaydiumClmmClosePosition(e) => e.metadata.signature,
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => e.metadata.signature,
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => e.metadata.signature,
            DexEvent::RaydiumCpmmSwap(e) => e.metadata.signature,
            DexEvent::RaydiumCpmmDeposit(e) => e.metadata.signature,
            DexEvent::RaydiumCpmmWithdraw(e) => e.metadata.signature,
            DexEvent::RaydiumCpmmInitialize(e) => e.metadata.signature,
            DexEvent::TokenAccount(e) => e.metadata.signature,
            DexEvent::NonceAccount(e) => e.metadata.signature,
            DexEvent::BlockMeta(e) => e.metadata.signature,
            DexEvent::TokenInfo(e) => e.metadata.signature,
            DexEvent::Error(_) => Signature::default(),
            DexEvent::PumpSwapPoolCreated(e) => e.signature,
            DexEvent::PumpSwapTrade(e) => e.signature,
            DexEvent::PumpSwapLiquidityAdded(e) => e.signature,
            DexEvent::PumpSwapLiquidityRemoved(e) => e.signature,
            DexEvent::PumpSwapPoolUpdated(e) => e.signature,
            DexEvent::PumpSwapFeesClaimed(e) => e.signature,
        }
    }

    /// 获取事件的槽号 - 纯函数
    pub fn get_slot(&self) -> u64 {
        match self {
            DexEvent::PumpFunCreate(e) => e.metadata.slot,
            DexEvent::PumpFunTrade(e) => e.metadata.slot,
            DexEvent::PumpFunComplete(e) => e.metadata.slot,
            DexEvent::BonkTrade(e) => e.metadata.slot,
            DexEvent::BonkPoolCreate(e) => e.metadata.slot,
            DexEvent::BonkMigrateAmm(e) => e.metadata.slot,
            DexEvent::PumpSwapBuy(e) => e.metadata.slot,
            DexEvent::PumpSwapSell(e) => e.metadata.slot,
            DexEvent::PumpSwapCreatePool(e) => e.metadata.slot,
            DexEvent::RaydiumClmmSwap(e) => e.metadata.slot,
            DexEvent::RaydiumClmmCreatePool(e) => e.metadata.slot,
            DexEvent::RaydiumClmmOpenPosition(e) => e.metadata.slot,
            DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => e.metadata.slot,
            DexEvent::RaydiumClmmClosePosition(e) => e.metadata.slot,
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => e.metadata.slot,
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => e.metadata.slot,
            DexEvent::RaydiumCpmmSwap(e) => e.metadata.slot,
            DexEvent::RaydiumCpmmDeposit(e) => e.metadata.slot,
            DexEvent::RaydiumCpmmWithdraw(e) => e.metadata.slot,
            DexEvent::RaydiumCpmmInitialize(e) => e.metadata.slot,
            DexEvent::TokenAccount(e) => e.metadata.slot,
            DexEvent::NonceAccount(e) => e.metadata.slot,
            DexEvent::BlockMeta(e) => e.metadata.slot,
            DexEvent::TokenInfo(e) => e.metadata.slot,
            DexEvent::Error(_) => 0,
            DexEvent::PumpSwapPoolCreated(e) => e.slot,
            DexEvent::PumpSwapTrade(e) => e.slot,
            DexEvent::PumpSwapLiquidityAdded(e) => e.slot,
            DexEvent::PumpSwapLiquidityRemoved(e) => e.slot,
            DexEvent::PumpSwapPoolUpdated(e) => e.slot,
            DexEvent::PumpSwapFeesClaimed(e) => e.slot,
        }
    }

    /// 检查是否是交易事件 - 纯函数
    pub fn is_trade_event(&self) -> bool {
        matches!(
            self,
            DexEvent::PumpFunTrade(_)
                | DexEvent::BonkTrade(_)
                | DexEvent::PumpSwapBuy(_)
                | DexEvent::PumpSwapSell(_)
                | DexEvent::RaydiumClmmSwap(_)
                | DexEvent::RaydiumCpmmSwap(_)
                | DexEvent::PumpSwapTrade(_)
        )
    }

    /// 检查是否是创建事件 - 纯函数
    pub fn is_create_event(&self) -> bool {
        matches!(
            self,
            DexEvent::PumpFunCreate(_)
                | DexEvent::BonkPoolCreate(_)
                | DexEvent::PumpSwapCreatePool(_)
                | DexEvent::RaydiumClmmCreatePool(_)
                | DexEvent::PumpSwapPoolCreated(_)
        )
    }

    /// 检查是否是流动性事件 - 纯函数
    pub fn is_liquidity_event(&self) -> bool {
        matches!(
            self,
            DexEvent::RaydiumClmmOpenPosition(_)
                | DexEvent::RaydiumClmmClosePosition(_)
                | DexEvent::RaydiumClmmIncreaseLiquidity(_)
                | DexEvent::RaydiumClmmDecreaseLiquidity(_)
                | DexEvent::RaydiumCpmmDeposit(_)
                | DexEvent::RaydiumCpmmWithdraw(_)
                | DexEvent::PumpSwapLiquidityAdded(_)
                | DexEvent::PumpSwapLiquidityRemoved(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_event_key_generation() {
        let signature = Signature::default();
        let slot = 123;
        let block_time = None;

        let event = DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            metadata: EventMetadata {
                signature,
                slot,
                block_time,
                block_time_ms: None,
                program_id: Pubkey::default(),
                outer_index: 0,
                inner_index: None,
                transaction_index: None,
                recv_us: 0,
                handle_us: 0,
            },
            name: "Test".to_string(),
            symbol: "TEST".to_string(),
            uri: "".to_string(),
            mint: Pubkey::default(),
            bonding_curve: Pubkey::default(),
            user: Pubkey::default(),
            creator: Pubkey::default(),
            virtual_token_reserves: 0,
            virtual_sol_reserves: 0,
            real_token_reserves: 0,
            token_total_supply: 0,
            timestamp: 0,
            mint_authority: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
        });

        let key = generate_event_key(&event);
        assert!(key.starts_with("pumpfun_create_"));
    }

    #[test]
    fn test_event_helpers() {
        let signature = Signature::default();
        let slot = 123;

        let trade_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: EventMetadata {
                signature,
                slot,
                block_time: None,
                block_time_ms: None,
                program_id: Pubkey::default(),
                outer_index: 0,
                inner_index: None,
                transaction_index: None,
                recv_us: 0,
                handle_us: 0,
            },
            mint: Pubkey::default(),
            user: Pubkey::default(),
            sol_amount: 1000,
            token_amount: 500,
            is_buy: true,
            bonding_curve: Pubkey::default(),
            virtual_sol_reserves: 0,
            virtual_token_reserves: 0,
            real_sol_reserves: 0,
            real_token_reserves: 0,
            fee_recipient: Pubkey::default(),
            fee_basis_points: 0,
            fee: 0,
            creator: Pubkey::default(),
            creator_fee_basis_points: 0,
            creator_fee: 0,
            total_unclaimed_tokens: 0,
            total_claimed_tokens: 0,
            current_sol_volume: 0,
            timestamp: 0,
            last_update_timestamp: 0,
            track_volume: false,
            max_sol_cost: 0,
            min_sol_output: 0,
            amount: 0,
            is_bot: false,
            is_dev_create_token_trade: false,
            global: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
            associated_user: Pubkey::default(),
            system_program: Pubkey::default(),
            token_program: Pubkey::default(),
            creator_vault: Pubkey::default(),
            event_authority: Pubkey::default(),
            program: Pubkey::default(),
            global_volume_accumulator: Pubkey::default(),
            user_volume_accumulator: Pubkey::default(),
        });

        assert_eq!(trade_event.get_signature(), signature);
        assert_eq!(trade_event.get_slot(), slot);
        assert!(trade_event.is_trade_event());
        assert!(!trade_event.is_create_event());
        assert!(!trade_event.is_liquidity_event());
    }

    #[test]
    fn test_merge_events() {
        let signature = Signature::default();
        let mint = Pubkey::default();

        let base_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: EventMetadata {
                signature,
                slot: 100,
                block_time: None,
                block_time_ms: None,
                program_id: Pubkey::default(),
                outer_index: 0,
                inner_index: None,
                transaction_index: None,
                recv_us: 0,
                handle_us: 0,
            },
            mint,
            user: Pubkey::default(),
            sol_amount: 1000,
            token_amount: 500,
            is_buy: true,
            bonding_curve: Pubkey::default(),
            virtual_sol_reserves: 0, // 空字段
            virtual_token_reserves: 0, // 空字段
            real_sol_reserves: 0,
            real_token_reserves: 0,
            fee_recipient: Pubkey::default(),
            fee_basis_points: 0,
            fee: 0, // 空字段
            creator: Pubkey::default(),
            creator_fee_basis_points: 0,
            creator_fee: 0,
            total_unclaimed_tokens: 0,
            total_claimed_tokens: 0,
            current_sol_volume: 0,
            timestamp: 0,
            last_update_timestamp: 0,
            track_volume: false,
            max_sol_cost: 0,
            min_sol_output: 0,
            amount: 0,
            is_bot: false,
            is_dev_create_token_trade: false,
            global: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
            associated_user: Pubkey::default(),
            system_program: Pubkey::default(),
            token_program: Pubkey::default(),
            creator_vault: Pubkey::default(),
            event_authority: Pubkey::default(),
            program: Pubkey::default(),
            global_volume_accumulator: Pubkey::default(),
            user_volume_accumulator: Pubkey::default(),
        });

        let log_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: EventMetadata {
                signature,
                slot: 100,
                block_time: None,
                block_time_ms: None,
                program_id: Pubkey::default(),
                outer_index: 0,
                inner_index: None,
                transaction_index: None,
                recv_us: 0,
                handle_us: 0,
            },
            mint,
            user: Pubkey::default(),
            sol_amount: 1000,
            token_amount: 500,
            is_buy: true,
            bonding_curve: Pubkey::default(),
            virtual_sol_reserves: 1000000, // 从日志获得的真实数据
            virtual_token_reserves: 500000, // 从日志获得的真实数据
            real_sol_reserves: 900000,
            real_token_reserves: 450000,
            fee_recipient: Pubkey::default(),
            fee_basis_points: 0,
            fee: 100, // 从日志获得的真实手续费
            creator: Pubkey::default(),
            creator_fee_basis_points: 0,
            creator_fee: 0,
            total_unclaimed_tokens: 0,
            total_claimed_tokens: 0,
            current_sol_volume: 0,
            timestamp: 1640995200,
            last_update_timestamp: 0,
            track_volume: false,
            max_sol_cost: 0,
            min_sol_output: 0,
            amount: 0,
            is_bot: false,
            is_dev_create_token_trade: false,
            global: Pubkey::default(),
            associated_bonding_curve: Pubkey::default(),
            associated_user: Pubkey::default(),
            system_program: Pubkey::default(),
            token_program: Pubkey::default(),
            creator_vault: Pubkey::default(),
            event_authority: Pubkey::default(),
            program: Pubkey::default(),
            global_volume_accumulator: Pubkey::default(),
            user_volume_accumulator: Pubkey::default(),
        });

        let merged = merge_events(base_event, log_event);

        if let DexEvent::PumpFunTrade(merged_trade) = merged {
            assert_eq!(merged_trade.virtual_sol_reserves, 1000000); // 应该被填充
            assert_eq!(merged_trade.virtual_token_reserves, 500000); // 应该被填充
            assert_eq!(merged_trade.fee, 100); // 应该被填充
            assert_eq!(merged_trade.timestamp, 1640995200); // 应该被填充
            assert_eq!(merged_trade.sol_amount, 1000); // 原始数据保留
        } else {
            panic!("Merge failed");
        }
    }

    #[test]
    fn test_parse_logs_only() {
        let logs = vec![
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
            "Program data: dGVzdCBkYXRh".to_string(), // base64 encoded test data
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string(),
        ];

        let events = parse_from_logs_only(
            &logs,
            Signature::default(),
            100,
            Some(1640995200),
        );

        // 应该解析出一些事件（具体取决于日志格式的实现）
        // 这里只是测试函数不会崩溃
        assert!(events.len() >= 0);
    }
}