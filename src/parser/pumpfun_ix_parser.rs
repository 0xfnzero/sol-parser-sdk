//! PumpFun 指令解析器 - 主解析器
//!
//! 设计原则：
//! - 解析指令数据并调用日志解析器
//! - 整合指令意图和日志执行结果
//! - 返回统一的 DexEvent 类型
//! - 函数式编程范式

use crate::parser::events::*;
use crate::parser::pumpfun_logs_parser;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;

/// PumpFun 程序 ID
pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// 指令判别符常量
pub const CREATE_TOKEN_DISCRIMINATOR: &[u8] = &[24, 30, 200, 40, 5, 28, 7, 119];
pub const BUY_DISCRIMINATOR: &[u8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const SELL_DISCRIMINATOR: &[u8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const MIGRATE_DISCRIMINATOR: &[u8] = &[155, 234, 231, 146, 236, 158, 162, 30];

/// 检查程序 ID 是否匹配 - 纯函数
#[inline(always)]
pub fn is_program_id(pubkey: &Pubkey) -> bool {
    match Pubkey::from_str(PROGRAM_ID) {
        Ok(program_id) => *pubkey == program_id,
        Err(_) => false,
    }
}

/// 匹配指令判别符 - 纯函数
#[inline(always)]
pub fn match_discriminator(data: &[u8]) -> Option<&'static [u8]> {
    if data.len() < 8 {
        return None;
    }

    let disc = &data[..8];
    match disc {
        CREATE_TOKEN_DISCRIMINATOR => Some(CREATE_TOKEN_DISCRIMINATOR),
        BUY_DISCRIMINATOR => Some(BUY_DISCRIMINATOR),
        SELL_DISCRIMINATOR => Some(SELL_DISCRIMINATOR),
        MIGRATE_DISCRIMINATOR => Some(MIGRATE_DISCRIMINATOR),
        _ => None,
    }
}

/// 解析创建代币指令 - 纯函数
pub fn parse_create_token(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    // 验证最小数据长度
    if data.len() < 24 || accounts.len() < 11 {
        return Err(ParseError::InsufficientData);
    }

    // 跳过判别符
    let mut offset = 8;

    // 解析名称
    let name = parse_string(data, &mut offset)?;
    let symbol = parse_string(data, &mut offset)?;
    let uri = parse_string(data, &mut offset)?;

    // 解析创建者
    let creator = if offset + 32 <= data.len() {
        parse_pubkey(&data[offset..offset + 32])?
    } else {
        Pubkey::default()
    };

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    let event = PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint: accounts[0],
        bonding_curve: accounts[2],
        user: accounts[7],
        creator,
        virtual_token_reserves: 0,
        virtual_sol_reserves: 0,
        real_token_reserves: 0,
        token_total_supply: 0,
        timestamp: 0,
        mint_authority: accounts[1],
        associated_bonding_curve: accounts[3],
    };

    Ok(DexEvent::PumpFunCreate(event))
}

/// 解析买入指令 - 纯函数
pub fn parse_buy(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 24 || accounts.len() < 13 {
        return Err(ParseError::InsufficientData);
    }

    // 跳过判别符，解析参数
    let amount = parse_u64(&data[8..16])?;
    let max_sol_cost = parse_u64(&data[16..24])?;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    let event = PumpFunTradeEvent {
        metadata,
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
        fee_recipient: accounts[1],
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
        max_sol_cost,
        min_sol_output: 0,
        amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: accounts[0],
        associated_bonding_curve: accounts[4],
        associated_user: accounts[5],
        system_program: accounts[7],
        token_program: accounts[8],
        creator_vault: accounts[9],
        event_authority: accounts[10],
        program: accounts[11],
        global_volume_accumulator: accounts[12],
        user_volume_accumulator: get_account_or_default(accounts, 13),
    };

    Ok(DexEvent::PumpFunTrade(event))
}

/// 解析卖出指令 - 纯函数
pub fn parse_sell(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 24 || accounts.len() < 11 {
        return Err(ParseError::InsufficientData);
    }

    let amount = parse_u64(&data[8..16])?;
    let min_sol_output = parse_u64(&data[16..24])?;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    let event = PumpFunTradeEvent {
        metadata,
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
        fee_recipient: accounts[1],
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
        min_sol_output,
        amount,
        is_bot: false,
        is_dev_create_token_trade: false,
        global: accounts[0],
        associated_bonding_curve: accounts[4],
        associated_user: accounts[5],
        system_program: accounts[7],
        token_program: accounts[9],
        creator_vault: accounts[8],
        event_authority: accounts[10],
        program: accounts[11],
        global_volume_accumulator: get_account_or_default(accounts, 12),
        user_volume_accumulator: get_account_or_default(accounts, 13),
    };

    Ok(DexEvent::PumpFunTrade(event))
}

/// 解析迁移指令 - 纯函数
pub fn parse_migrate(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if accounts.len() < 6 {
        return Err(ParseError::InsufficientData);
    }

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    let event = PumpFunCompleteTokenEvent {
        metadata,
        user: accounts[5],
        mint: accounts[2],
        bonding_curve: accounts[3],
    };

    Ok(DexEvent::PumpFunComplete(event))
}

/// 主解析函数 - 纯函数
pub fn parse_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<Option<DexEvent>, ParseError> {
    let discriminator = match match_discriminator(data) {
        Some(disc) => disc,
        None => return Ok(None), // 不是 PumpFun 指令
    };

    let result = match discriminator {
        CREATE_TOKEN_DISCRIMINATOR => {
            parse_create_token(data, accounts, signature, slot, block_time, instruction_index)
        }
        BUY_DISCRIMINATOR => {
            parse_buy(data, accounts, signature, slot, block_time, instruction_index)
        }
        SELL_DISCRIMINATOR => {
            parse_sell(data, accounts, signature, slot, block_time, instruction_index)
        }
        MIGRATE_DISCRIMINATOR => {
            parse_migrate(data, accounts, signature, slot, block_time, instruction_index)
        }
        _ => return Ok(None),
    };

    result.map(Some)
}

/// 批量解析指令 - 纯函数
pub fn parse_instructions_batch(
    instructions: &[(Vec<u8>, Vec<Pubkey>)],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<Result<DexEvent, ParseError>> {
    instructions
        .iter()
        .enumerate()
        .filter_map(|(index, (data, accounts))| {
            match parse_instruction(data, accounts, signature, slot, block_time, index as i64) {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None, // 不是 PumpFun 指令
                Err(e) => Some(Err(e)),
            }
        })
        .collect()
}

/// 流式解析指令 - 惰性求值
pub fn parse_instructions_stream<'a>(
    instructions: &'a [(Vec<u8>, Vec<Pubkey>)],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> impl Iterator<Item = Result<DexEvent, ParseError>> + 'a {
    instructions
        .iter()
        .enumerate()
        .filter_map(move |(index, (data, accounts))| {
            match parse_instruction(data, accounts, signature, slot, block_time, index as i64) {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
}

// ============= 辅助函数 =============

/// 解析错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InsufficientData,
    InvalidData,
    InvalidPubkey,
    ProgramIdParseError,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InsufficientData => write!(f, "Insufficient data"),
            ParseError::InvalidData => write!(f, "Invalid data"),
            ParseError::InvalidPubkey => write!(f, "Invalid pubkey"),
            ParseError::ProgramIdParseError => write!(f, "Program ID parse error"),
        }
    }
}

impl std::error::Error for ParseError {}

/// 创建事件元数据 - 纯函数
fn create_metadata(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
    inner_instruction_index: Option<i64>,
) -> Result<EventMetadata, ParseError> {
    let program_id = Pubkey::from_str(PROGRAM_ID)
        .map_err(|_| ParseError::ProgramIdParseError)?;

    Ok(EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time,
        program_id,
        outer_index: instruction_index,
        inner_index: inner_instruction_index,
        transaction_index: None,
        recv_us: 0,
        handle_us: 0,
    })
}

/// 解析字符串 - 纯函数
fn parse_string(data: &[u8], offset: &mut usize) -> Result<String, ParseError> {
    if *offset + 4 > data.len() {
        return Err(ParseError::InsufficientData);
    }

    let len = u32::from_le_bytes(
        data[*offset..*offset + 4]
            .try_into()
            .map_err(|_| ParseError::InvalidData)?
    ) as usize;
    *offset += 4;

    if *offset + len > data.len() {
        return Err(ParseError::InsufficientData);
    }

    let string = String::from_utf8_lossy(&data[*offset..*offset + len]).to_string();
    *offset += len;

    Ok(string)
}

/// 解析 u64 - 纯函数
fn parse_u64(data: &[u8]) -> Result<u64, ParseError> {
    if data.len() < 8 {
        return Err(ParseError::InsufficientData);
    }

    data[..8]
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| ParseError::InvalidData)
}

/// 解析公钥 - 纯函数
fn parse_pubkey(data: &[u8]) -> Result<Pubkey, ParseError> {
    if data.len() < 32 {
        return Err(ParseError::InsufficientData);
    }

    data[..32]
        .try_into()
        .map(Pubkey::new_from_array)
        .map_err(|_| ParseError::InvalidPubkey)
}

/// 安全获取账户或默认值 - 纯函数
fn get_account_or_default(accounts: &[Pubkey], index: usize) -> Pubkey {
    accounts.get(index).copied().unwrap_or_default()
}

/// 主解析函数 - 智能合并指令和日志数据，返回最完整的 DexEvent
pub fn parse_pumpfun_transaction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 1. 解析指令数据（用户意图 + 账户信息）
    let instruction_event = match parse_instruction(
        instruction_data,
        accounts,
        signature,
        slot,
        block_time,
        instruction_index as i64,
    ) {
        Ok(Some(event)) => Some(event),
        _ => None,
    };

    // 2. 解析日志数据（实际执行结果）
    let mut log_event = None;
    for log in logs {
        if let Some(event) = pumpfun_logs_parser::parse_pumpfun_from_log_string(
            log,
            signature,
            slot,
            block_time,
        ) {
            log_event = Some(event);
            break;
        }
    }

    // 3. 智能合并：优先使用日志数据，用指令数据补充缺失字段
    merge_instruction_and_log_events(instruction_event, log_event)
}

/// 智能合并指令和日志事件数据
fn merge_instruction_and_log_events(
    instruction_event: Option<DexEvent>,
    log_event: Option<DexEvent>,
) -> Option<DexEvent> {
    match (instruction_event, log_event) {
        // 理想情况：同时有指令和日志数据，合并它们
        (Some(DexEvent::PumpFunTrade(ix_trade)), Some(DexEvent::PumpFunTrade(log_trade))) => {
            Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
                // 元数据：优先使用日志的
                metadata: log_trade.metadata.clone(),

                // 核心交易数据：优先使用日志的真实数据
                mint: log_trade.mint,
                user: log_trade.user,
                sol_amount: log_trade.sol_amount, // 日志中的实际金额
                token_amount: log_trade.token_amount, // 日志中的实际代币数量
                is_buy: log_trade.is_buy,

                // 账户信息：使用指令中的完整账户信息
                bonding_curve: ix_trade.bonding_curve,
                global: ix_trade.global,
                associated_bonding_curve: ix_trade.associated_bonding_curve,
                associated_user: ix_trade.associated_user,
                system_program: ix_trade.system_program,
                token_program: ix_trade.token_program,
                creator_vault: ix_trade.creator_vault,
                event_authority: ix_trade.event_authority,
                program: ix_trade.program,
                global_volume_accumulator: ix_trade.global_volume_accumulator,
                user_volume_accumulator: ix_trade.user_volume_accumulator,

                // 储备和费用：优先使用日志中的真实数据
                virtual_sol_reserves: log_trade.virtual_sol_reserves,
                virtual_token_reserves: log_trade.virtual_token_reserves,
                real_sol_reserves: log_trade.real_sol_reserves,
                real_token_reserves: log_trade.real_token_reserves,
                fee_recipient: log_trade.fee_recipient,
                fee_basis_points: log_trade.fee_basis_points,
                fee: log_trade.fee, // 实际费用
                creator: log_trade.creator,
                creator_fee_basis_points: log_trade.creator_fee_basis_points,
                creator_fee: log_trade.creator_fee, // 实际创作者费用

                // 交易量和统计：优先使用日志数据
                total_unclaimed_tokens: log_trade.total_unclaimed_tokens,
                total_claimed_tokens: log_trade.total_claimed_tokens,
                current_sol_volume: log_trade.current_sol_volume,
                timestamp: log_trade.timestamp, // 真实时间戳
                last_update_timestamp: log_trade.last_update_timestamp,
                track_volume: log_trade.track_volume,

                // 用户意图参数：使用指令中的数据
                max_sol_cost: ix_trade.max_sol_cost,
                min_sol_output: ix_trade.min_sol_output,
                amount: ix_trade.amount, // 用户期望的数量

                // 其他分析字段：可以基于合并后的数据推断
                is_bot: detect_bot_behavior(&ix_trade, &log_trade),
                is_dev_create_token_trade: ix_trade.is_dev_create_token_trade,
            }))
        }

        // 创建代币事件的合并
        (Some(DexEvent::PumpFunCreate(ix_create)), Some(DexEvent::PumpFunCreate(log_create))) => {
            Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: log_create.metadata,

                // 代币信息：优先使用日志中的数据
                name: log_create.name,
                symbol: log_create.symbol,
                uri: log_create.uri,
                mint: log_create.mint,
                bonding_curve: log_create.bonding_curve,
                user: log_create.user,
                creator: log_create.creator,

                // 储备和供应：使用日志中的真实数据
                virtual_token_reserves: log_create.virtual_token_reserves,
                virtual_sol_reserves: log_create.virtual_sol_reserves,
                real_token_reserves: log_create.real_token_reserves,
                token_total_supply: log_create.token_total_supply,
                timestamp: log_create.timestamp,

                // 账户信息：补充指令中的信息
                mint_authority: ix_create.mint_authority,
                associated_bonding_curve: ix_create.associated_bonding_curve,
            }))
        }

        // 只有日志数据：直接返回（已经很完整了）
        (None, Some(log_event)) => Some(log_event),

        // 只有指令数据：返回但数据不够完整
        (Some(ix_event), None) => Some(ix_event),

        // 都没有：返回None
        (None, None) => None,

        // 类型不匹配：优先返回日志数据
        (_, Some(log_event)) => Some(log_event),
    }
}

/// 检测是否为机器人行为（基于指令意图和实际执行结果的差异）
fn detect_bot_behavior(ix_trade: &PumpFunTradeEvent, log_trade: &PumpFunTradeEvent) -> bool {
    // 如果用户设置的最大成本和实际花费差异很大，可能是机器人
    if ix_trade.max_sol_cost > 0 && log_trade.sol_amount > 0 {
        let cost_ratio = log_trade.sol_amount as f64 / ix_trade.max_sol_cost as f64;
        // 如果实际成本是预期成本的95%以上，可能是精确的机器人交易
        return cost_ratio > 0.95 && cost_ratio <= 1.0;
    }
    false
}

/// 批量解析 PumpFun 交易
pub fn parse_pumpfun_transactions_batch(
    transactions: &[(Vec<u8>, Vec<Pubkey>, Vec<String>)],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    transactions
        .iter()
        .enumerate()
        .filter_map(|(index, (instruction_data, accounts, logs))| {
            parse_pumpfun_transaction(
                instruction_data,
                accounts,
                logs,
                signature,
                slot,
                block_time,
                index as u32,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_id_check() {
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        assert!(is_program_id(&program_id));
        assert!(!is_program_id(&Pubkey::default()));
    }

    #[test]
    fn test_discriminator_matching() {
        let mut data = vec![0u8; 16];

        // 测试买入判别符
        data[..8].copy_from_slice(BUY_DISCRIMINATOR);
        assert_eq!(match_discriminator(&data), Some(BUY_DISCRIMINATOR));

        // 测试卖出判别符
        data[..8].copy_from_slice(SELL_DISCRIMINATOR);
        assert_eq!(match_discriminator(&data), Some(SELL_DISCRIMINATOR));

        // 测试无效判别符
        data[..8].fill(0);
        assert_eq!(match_discriminator(&data), None);
    }

    #[test]
    fn test_parse_error_display() {
        assert_eq!(format!("{}", ParseError::InsufficientData), "Insufficient data");
        assert_eq!(format!("{}", ParseError::InvalidData), "Invalid data");
    }

    #[test]
    fn test_parse_u64() {
        let data = 1000u64.to_le_bytes();
        assert_eq!(parse_u64(&data), Ok(1000));

        let insufficient_data = [1, 2, 3];
        assert_eq!(parse_u64(&insufficient_data), Err(ParseError::InsufficientData));
    }

    #[test]
    fn test_get_account_or_default() {
        let accounts = vec![Pubkey::default(), Pubkey::new_unique()];

        assert_eq!(get_account_or_default(&accounts, 0), accounts[0]);
        assert_eq!(get_account_or_default(&accounts, 1), accounts[1]);
        assert_eq!(get_account_or_default(&accounts, 5), Pubkey::default());
    }

    #[test]
    fn test_instruction_parsing_insufficient_data() {
        let data = vec![0u8; 4]; // 太短
        let accounts = vec![Pubkey::default(); 5]; // 太少
        let signature = Signature::default();

        let result = parse_instruction(&data, &accounts, signature, 0, None, 0);
        assert_eq!(result, Ok(None)); // 判别符不匹配，返回 None
    }

    #[test]
    fn test_batch_parsing() {
        let mut instruction1 = vec![0u8; 32];
        instruction1[..8].copy_from_slice(BUY_DISCRIMINATOR);
        instruction1[8..16].copy_from_slice(&1000u64.to_le_bytes());
        instruction1[16..24].copy_from_slice(&2000u64.to_le_bytes());

        let mut instruction2 = vec![0u8; 32];
        instruction2[..8].copy_from_slice(SELL_DISCRIMINATOR);
        instruction2[8..16].copy_from_slice(&500u64.to_le_bytes());
        instruction2[16..24].copy_from_slice(&1000u64.to_le_bytes());

        let accounts = vec![Pubkey::default(); 14];
        let instructions = vec![
            (instruction1, accounts.clone()),
            (instruction2, accounts),
        ];

        let results = parse_instructions_batch(&instructions, Signature::default(), 0, None);
        assert_eq!(results.len(), 2);

        // 验证都是成功的交易事件
        for result in results {
            assert!(result.is_ok());
            match result.unwrap() {
                DexEvent::PumpFunTrade(_) => (),
                _ => panic!("Expected PumpFunTrade event"),
            }
        }
    }
}