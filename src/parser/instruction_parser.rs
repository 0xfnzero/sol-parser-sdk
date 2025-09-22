//! 指令解析器 - 从旧版本集成高性能指令解析功能
//!
//! 这个模块整合了旧版本 solana-streamer 中的指令解析能力：
//! - 支持从原始指令数据解析事件，而不仅仅是日志
//! - 统一的配置驱动架构
//! - SIMD 优化的性能
//! - 缓存机制减少内存分配

use crate::parser::events::*;
// use crate::common::StreamClientConfig;
use prost_types::Timestamp;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    instruction::CompiledInstruction,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{InnerInstructions};
use std::collections::HashMap;
use std::sync::Arc;
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransactionInfo;

/// 指令事件解析器类型
pub type InstructionEventParser =
    fn(data: &[u8], accounts: &[Pubkey], metadata: InstructionMetadata) -> Option<DexEvent>;

/// 内联指令事件解析器类型
pub type InnerInstructionEventParser =
    fn(data: &[u8], metadata: InstructionMetadata) -> Option<DexEvent>;

/// 指令元数据
#[derive(Debug, Clone)]
pub struct InstructionMetadata {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<Timestamp>,
    pub program_id: Pubkey,
    pub instruction_index: i64,
    pub inner_instruction_index: Option<i64>,
    pub transaction_index: Option<u64>,
}

impl InstructionMetadata {
    pub fn new(
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
        program_id: Pubkey,
        instruction_index: i64,
        inner_instruction_index: Option<i64>,
        transaction_index: Option<u64>,
    ) -> Self {
        Self {
            signature,
            slot,
            block_time,
            program_id,
            instruction_index,
            inner_instruction_index,
            transaction_index,
        }
    }
}

/// 通用事件解析配置
#[derive(Debug, Clone)]
pub struct InstructionParseConfig {
    pub program_id: Pubkey,
    pub instruction_discriminator: &'static [u8],
    pub inner_instruction_discriminator: &'static [u8],
    pub instruction_parser: Option<InstructionEventParser>,
    pub inner_instruction_parser: Option<InnerInstructionEventParser>,
    pub requires_inner_instruction: bool,
}

/// 账户公钥缓存，避免重复分配
#[derive(Debug)]
pub struct AccountPubkeyCache {
    cache: Vec<Pubkey>,
}

impl AccountPubkeyCache {
    pub fn new() -> Self {
        Self {
            cache: Vec::with_capacity(32),
        }
    }

    /// 从指令账户索引构建账户公钥向量
    pub fn build_account_pubkeys(
        &mut self,
        instruction_accounts: &[u8],
        all_accounts: &[Pubkey],
    ) -> &[Pubkey] {
        self.cache.clear();

        if self.cache.capacity() < instruction_accounts.len() {
            self.cache.reserve(instruction_accounts.len() - self.cache.capacity());
        }

        for &idx in instruction_accounts.iter() {
            if (idx as usize) < all_accounts.len() {
                self.cache.push(all_accounts[idx as usize]);
            }
        }

        &self.cache
    }
}

impl Default for AccountPubkeyCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 高性能指令解析器
pub struct InstructionParser {
    program_ids: Vec<Pubkey>,
    instruction_configs: HashMap<Vec<u8>, Vec<InstructionParseConfig>>,
    account_cache: parking_lot::Mutex<AccountPubkeyCache>,
}

impl InstructionParser {
    pub fn new() -> Self {
        let mut parser = Self {
            program_ids: Vec::new(),
            instruction_configs: HashMap::new(),
            account_cache: parking_lot::Mutex::new(AccountPubkeyCache::new()),
        };

        // 注册所有DEX的解析配置
        parser.register_pumpfun_configs();
        parser.register_bonk_configs();
        parser.register_pumpswap_configs();
        // TODO: 添加 Raydium 配置
        // parser.register_raydium_clmm_configs();
        // parser.register_raydium_cpmm_configs();

        parser
    }

    /// 注册PumpFun解析配置
    fn register_pumpfun_configs(&mut self) {
        let program_id = crate::parser::pumpfun::PUMPFUN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        self.program_ids.push(program_id);

        // 创建代币指令配置
        let create_config = InstructionParseConfig {
            program_id,
            instruction_discriminator: &[0x18, 0x1e, 0xc8, 0x28, 0x05, 0x11, 0x05, 0x1b], // create discriminator
            inner_instruction_discriminator: &[0x55, 0x28, 0x6e, 0x69, 0x4a, 0x4e, 0x56, 0xfe], // create event discriminator
            instruction_parser: Some(parse_pumpfun_create_instruction),
            inner_instruction_parser: Some(parse_pumpfun_create_inner_instruction),
            requires_inner_instruction: false,
        };

        // 买入指令配置
        let buy_config = InstructionParseConfig {
            program_id,
            instruction_discriminator: &[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea], // buy discriminator
            inner_instruction_discriminator: &[0x2e, 0x05, 0x27, 0x9b, 0xe7, 0x37, 0x7b, 0x04], // trade event discriminator
            instruction_parser: Some(parse_pumpfun_buy_instruction),
            inner_instruction_parser: Some(parse_pumpfun_trade_inner_instruction),
            requires_inner_instruction: false,
        };

        // 卖出指令配置
        let sell_config = InstructionParseConfig {
            program_id,
            instruction_discriminator: &[0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad], // sell discriminator
            inner_instruction_discriminator: &[0x2e, 0x05, 0x27, 0x9b, 0xe7, 0x37, 0x7b, 0x04], // trade event discriminator
            instruction_parser: Some(parse_pumpfun_sell_instruction),
            inner_instruction_parser: Some(parse_pumpfun_trade_inner_instruction),
            requires_inner_instruction: false,
        };

        // 注册配置
        self.instruction_configs
            .entry(create_config.instruction_discriminator.to_vec())
            .or_insert_with(Vec::new)
            .push(create_config);

        self.instruction_configs
            .entry(buy_config.instruction_discriminator.to_vec())
            .or_insert_with(Vec::new)
            .push(buy_config);

        self.instruction_configs
            .entry(sell_config.instruction_discriminator.to_vec())
            .or_insert_with(Vec::new)
            .push(sell_config);
    }

    /// 注册Bonk解析配置
    fn register_bonk_configs(&mut self) {
        // TODO: 实现Bonk指令解析配置
    }

    /// 注册PumpSwap解析配置
    fn register_pumpswap_configs(&mut self) {
        // TODO: 实现PumpSwap指令解析配置
    }

    /// 解析 gRPC 交易中的指令事件
    pub async fn parse_grpc_transaction(
        &self,
        grpc_tx: SubscribeUpdateTransactionInfo,
        signature: Signature,
        slot: Option<u64>,
        block_time: Option<Timestamp>,
        callback: Arc<dyn Fn(DexEvent) + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(transaction) = grpc_tx.transaction {
            if let Some(message) = &transaction.message {
                let mut accounts: Vec<Pubkey> = message.account_keys
                    .iter()
                    .filter_map(|account| {
                        if account.len() == 32 {
                            Some(Pubkey::try_from(account.as_slice()).unwrap_or_default())
                        } else {
                            None
                        }
                    })
                    .collect();

                // 添加地址表查找账户
                if let Some(meta) = &grpc_tx.meta {
                    accounts.extend(
                        meta.loaded_writable_addresses
                            .iter()
                            .chain(meta.loaded_readonly_addresses.iter())
                            .filter_map(|account| {
                                if account.len() == 32 {
                                    Some(Pubkey::try_from(account.as_slice()).unwrap_or_default())
                                } else {
                                    None
                                }
                            })
                    );
                }

                // 解析每个指令
                for (index, instruction) in message.instructions.iter().enumerate() {
                    if let Some(program_id) = accounts.get(instruction.program_id_index as usize) {
                        if self.should_handle(program_id) {
                            self.parse_grpc_instruction(
                                instruction,
                                &accounts,
                                signature,
                                slot.unwrap_or(0),
                                block_time.clone(),
                                index as i64,
                                None,
                                callback.clone(),
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 解析版本化交易中的指令事件
    pub async fn parse_versioned_transaction(
        &self,
        versioned_tx: &VersionedTransaction,
        signature: Signature,
        slot: Option<u64>,
        block_time: Option<Timestamp>,
        inner_instructions: &[InnerInstructions],
        callback: Arc<dyn Fn(DexEvent) + Send + Sync>,
    ) -> anyhow::Result<()> {
        let accounts: Vec<Pubkey> = versioned_tx.message.static_account_keys().to_vec();
        let compiled_instructions = versioned_tx.message.instructions();

        for (index, instruction) in compiled_instructions.iter().enumerate() {
            if let Some(program_id) = accounts.get(instruction.program_id_index as usize) {
                if self.should_handle(program_id) {
                    self.parse_instruction(
                        instruction,
                        &accounts,
                        signature,
                        slot.unwrap_or(0),
                        block_time.clone(),
                        index as i64,
                        None,
                        callback.clone(),
                    )?;
                }
            }

            // 处理内联指令
            if let Some(inner_instructions) = inner_instructions
                .iter()
                .find(|inner| inner.index == index as u8)
            {
                for (inner_index, inner_instruction) in inner_instructions.instructions.iter().enumerate() {
                    self.parse_instruction(
                        &inner_instruction.instruction,
                        &accounts,
                        signature,
                        slot.unwrap_or(0),
                        block_time.clone(),
                        index as i64,
                        Some(inner_index as i64),
                        callback.clone(),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// 解析单个 gRPC 指令
    fn parse_grpc_instruction(
        &self,
        instruction: &yellowstone_grpc_proto::prelude::CompiledInstruction,
        accounts: &[Pubkey],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
        instruction_index: i64,
        inner_instruction_index: Option<i64>,
        callback: Arc<dyn Fn(DexEvent) + Send + Sync>,
    ) -> anyhow::Result<()> {
        let program_id = accounts[instruction.program_id_index as usize];

        // 检查指令判别符
        for (discriminator, configs) in &self.instruction_configs {
            if instruction.data.len() >= discriminator.len()
                && &instruction.data[..discriminator.len()] == discriminator {

                for config in configs {
                    if config.program_id == program_id {
                        // 构建账户公钥列表
                        let account_pubkeys = {
                            let mut cache_guard = self.account_cache.lock();
                            cache_guard.build_account_pubkeys(&instruction.accounts, accounts).to_vec()
                        };

                        let metadata = InstructionMetadata::new(
                            signature,
                            slot,
                            block_time.clone(),
                            program_id,
                            instruction_index,
                            inner_instruction_index,
                            None,
                        );

                        if let Some(parser) = config.instruction_parser {
                            let data = &instruction.data[discriminator.len()..];
                            if let Some(event) = parser(data, &account_pubkeys, metadata) {
                                callback(event);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 解析单个指令
    fn parse_instruction(
        &self,
        instruction: &CompiledInstruction,
        accounts: &[Pubkey],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
        instruction_index: i64,
        inner_instruction_index: Option<i64>,
        callback: Arc<dyn Fn(DexEvent) + Send + Sync>,
    ) -> anyhow::Result<()> {
        let program_id = accounts[instruction.program_id_index as usize];

        // 检查指令判别符
        for (discriminator, configs) in &self.instruction_configs {
            if instruction.data.len() >= discriminator.len()
                && &instruction.data[..discriminator.len()] == discriminator {

                for config in configs {
                    if config.program_id == program_id {
                        // 构建账户公钥列表
                        let account_pubkeys = {
                            let mut cache_guard = self.account_cache.lock();
                            cache_guard.build_account_pubkeys(&instruction.accounts, accounts).to_vec()
                        };

                        let metadata = InstructionMetadata::new(
                            signature,
                            slot,
                            block_time.clone(),
                            program_id,
                            instruction_index,
                            inner_instruction_index,
                            None,
                        );

                        if let Some(parser) = config.instruction_parser {
                            let data = &instruction.data[discriminator.len()..];
                            if let Some(event) = parser(data, &account_pubkeys, metadata) {
                                callback(event);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn should_handle(&self, program_id: &Pubkey) -> bool {
        self.program_ids.contains(program_id)
    }
}

impl Default for InstructionParser {
    fn default() -> Self {
        Self::new()
    }
}

// PumpFun 指令解析函数

/// 解析PumpFun创建代币指令
fn parse_pumpfun_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    metadata: InstructionMetadata,
) -> Option<DexEvent> {
    if data.len() < 16 || accounts.len() < 11 {
        return None;
    }

    let mut offset = 0;

    // 解析名称
    if offset + 4 > data.len() {
        return None;
    }
    let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
    offset += 4;
    if offset + name_len > data.len() {
        return None;
    }
    let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
    offset += name_len;

    // 解析符号
    if offset + 4 > data.len() {
        return None;
    }
    let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
    offset += 4;
    if offset + symbol_len > data.len() {
        return None;
    }
    let symbol = String::from_utf8_lossy(&data[offset..offset + symbol_len]).to_string();
    offset += symbol_len;

    // 解析URI
    if offset + 4 > data.len() {
        return None;
    }
    let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
    offset += 4;
    if offset + uri_len > data.len() {
        return None;
    }
    let uri = String::from_utf8_lossy(&data[offset..offset + uri_len]).to_string();
    offset += uri_len;

    // 解析创建者
    let creator = if offset + 32 <= data.len() {
        Pubkey::new_from_array(data[offset..offset + 32].try_into().ok()?)
    } else {
        Pubkey::default()
    };

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    let event_metadata = EventMetadata {
        signature: metadata.signature,
        slot: metadata.slot,
        block_time: metadata.block_time.map(|ts| ts.seconds),
        block_time_ms: metadata.block_time.map(|ts| ts.seconds * 1000),
        program_id: metadata.program_id,
        outer_index: metadata.instruction_index,
        inner_index: metadata.inner_instruction_index,
        transaction_index: metadata.transaction_index,
        recv_us: current_time,
        handle_us: current_time,
    };

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata: event_metadata,
        name,
        symbol,
        uri,
        mint: accounts[0],
        bonding_curve: accounts.get(2).copied().unwrap_or_default(),
        user: accounts.get(7).copied().unwrap_or_default(),
        creator,
        virtual_token_reserves: 0,
        virtual_sol_reserves: 0,
        real_token_reserves: 0,
        token_total_supply: 0,
        timestamp: current_time / 1_000_000, // Convert microseconds to seconds
        mint_authority: accounts.get(1).copied().unwrap_or_default(),
        associated_bonding_curve: accounts.get(3).copied().unwrap_or_default(),
    }))
}

/// 解析PumpFun创建代币内联指令
fn parse_pumpfun_create_inner_instruction(
    data: &[u8],
    metadata: InstructionMetadata,
) -> Option<DexEvent> {
    // TODO: 实现从日志数据解析创建事件
    None
}

/// 解析PumpFun买入指令
fn parse_pumpfun_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    metadata: InstructionMetadata,
) -> Option<DexEvent> {
    if data.len() < 16 || accounts.len() < 13 {
        return None;
    }

    let amount = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let max_sol_cost = u64::from_le_bytes(data[8..16].try_into().ok()?);

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    let event_metadata = EventMetadata {
        signature: metadata.signature,
        slot: metadata.slot,
        block_time: metadata.block_time.map(|ts| ts.seconds),
        block_time_ms: metadata.block_time.map(|ts| ts.seconds * 1000),
        program_id: metadata.program_id,
        outer_index: metadata.instruction_index,
        inner_index: metadata.inner_instruction_index,
        transaction_index: metadata.transaction_index,
        recv_us: current_time,
        handle_us: current_time,
    };

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata: event_metadata,
        mint: accounts[2],
        user: accounts[6],
        sol_amount: max_sol_cost,
        token_amount: amount,
        is_buy: true,
        bonding_curve: accounts[3],
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
        timestamp: current_time / 1_000_000,
        last_update_timestamp: current_time / 1_000_000,
        track_volume: false,
        max_sol_cost,
        min_sol_output: 0,
        amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: accounts.get(0).copied().unwrap_or_default(),
        associated_bonding_curve: accounts.get(4).copied().unwrap_or_default(),
        associated_user: accounts.get(5).copied().unwrap_or_default(),
        system_program: accounts.get(7).copied().unwrap_or_default(),
        token_program: accounts.get(8).copied().unwrap_or_default(),
        creator_vault: accounts.get(9).copied().unwrap_or_default(),
        event_authority: accounts.get(10).copied().unwrap_or_default(),
        program: accounts.get(11).copied().unwrap_or_default(),
        global_volume_accumulator: accounts.get(12).copied().unwrap_or_default(),
        user_volume_accumulator: accounts.get(13).copied().unwrap_or_default(),
    }))
}

/// 解析PumpFun卖出指令
fn parse_pumpfun_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    metadata: InstructionMetadata,
) -> Option<DexEvent> {
    if data.len() < 16 || accounts.len() < 11 {
        return None;
    }

    let amount = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let min_sol_output = u64::from_le_bytes(data[8..16].try_into().ok()?);

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    let event_metadata = EventMetadata {
        signature: metadata.signature,
        slot: metadata.slot,
        block_time: metadata.block_time.map(|ts| ts.seconds),
        block_time_ms: metadata.block_time.map(|ts| ts.seconds * 1000),
        program_id: metadata.program_id,
        outer_index: metadata.instruction_index,
        inner_index: metadata.inner_instruction_index,
        transaction_index: metadata.transaction_index,
        recv_us: current_time,
        handle_us: current_time,
    };

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata: event_metadata,
        mint: accounts[2],
        user: accounts[6],
        sol_amount: min_sol_output,
        token_amount: amount,
        is_buy: false,
        bonding_curve: accounts[3],
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
        timestamp: current_time / 1_000_000,
        last_update_timestamp: current_time / 1_000_000,
        track_volume: false,
        max_sol_cost: 0,
        min_sol_output,
        amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: accounts.get(0).copied().unwrap_or_default(),
        associated_bonding_curve: accounts.get(4).copied().unwrap_or_default(),
        associated_user: accounts.get(5).copied().unwrap_or_default(),
        system_program: accounts.get(7).copied().unwrap_or_default(),
        token_program: accounts.get(8).copied().unwrap_or_default(),
        creator_vault: accounts.get(9).copied().unwrap_or_default(),
        event_authority: accounts.get(10).copied().unwrap_or_default(),
        program: accounts.get(11).copied().unwrap_or_default(),
        global_volume_accumulator: accounts.get(12).copied().unwrap_or_default(),
        user_volume_accumulator: accounts.get(13).copied().unwrap_or_default(),
    }))
}

/// 解析PumpFun交易内联指令
fn parse_pumpfun_trade_inner_instruction(
    data: &[u8],
    metadata: InstructionMetadata,
) -> Option<DexEvent> {
    // TODO: 实现从日志数据解析交易事件
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_instruction_parser_creation() {
        let parser = InstructionParser::new();
        assert!(!parser.program_ids.is_empty());
        assert!(!parser.instruction_configs.is_empty());
    }

    #[test]
    fn test_account_cache() {
        let mut cache = AccountPubkeyCache::new();
        let accounts = vec![
            Pubkey::default(),
            Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        ];
        let instruction_accounts = vec![0u8, 1u8];

        let result = cache.build_account_pubkeys(&instruction_accounts, &accounts);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], accounts[0]);
        assert_eq!(result[1], accounts[1]);
    }
}