//! Bonk 指令解析器 - 纯函数式设计
//!
//! 设计原则：
//! - 纯函数，无副作用
//! - 零拷贝，高性能
//! - 职责单一，只解析 Bonk 指令
//! - 函数式编程范式

use crate::parser::events::*;
// use prost_types::Timestamp;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;

/// Bonk 程序 ID
pub const PROGRAM_ID: &str = "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1";

/// 指令判别符常量
pub const CREATE_POOL_DISCRIMINATOR: &[u8] = &[233, 146, 209, 142, 207, 104, 64, 188];
pub const SWAP_DISCRIMINATOR: &[u8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const ADD_LIQUIDITY_DISCRIMINATOR: &[u8] = &[181, 157, 89, 67, 143, 182, 52, 72];
pub const REMOVE_LIQUIDITY_DISCRIMINATOR: &[u8] = &[80, 85, 209, 72, 24, 206, 177, 108];

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
        CREATE_POOL_DISCRIMINATOR => Some(CREATE_POOL_DISCRIMINATOR),
        SWAP_DISCRIMINATOR => Some(SWAP_DISCRIMINATOR),
        ADD_LIQUIDITY_DISCRIMINATOR => Some(ADD_LIQUIDITY_DISCRIMINATOR),
        REMOVE_LIQUIDITY_DISCRIMINATOR => Some(REMOVE_LIQUIDITY_DISCRIMINATOR),
        _ => None,
    }
}

/// 解析创建池指令 - 纯函数
pub fn parse_create_pool(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 200 || accounts.len() < 20 {
        return Err(ParseError::InsufficientData);
    }

    let mut offset = 8; // 跳过判别符

    // 解析基础代币参数
    let symbol = parse_string(data, &mut offset)?;
    let name = parse_string(data, &mut offset)?;
    let uri = parse_string(data, &mut offset)?;
    let decimals = parse_u8(&data[offset..offset + 1])?;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    let event = BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol,
            name,
            uri,
            decimals,
        },
        pool_state: accounts[1], // pool state account
        creator: accounts[0],    // creator/payer account
    };

    Ok(DexEvent::BonkPoolCreate(event))
}

/// 解析交换指令 - 纯函数
pub fn parse_swap(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 40 || accounts.len() < 10 {
        return Err(ParseError::InsufficientData);
    }

    let mut offset = 8; // 跳过判别符

    // 解析交换参数
    let amount_in = parse_u64(&data[offset..offset + 8])?;
    offset += 8;
    let minimum_amount_out = parse_u64(&data[offset..offset + 8])?;
    offset += 8;
    let is_exact_input = parse_bool(&data[offset..offset + 1])?;
    offset += 1;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    // 确定交易方向
    let is_buy = determine_trade_direction(accounts)?;

    let event = BonkTradeEvent {
        metadata,
        pool_state: accounts[1],
        user: accounts[0],
        amount_in,
        amount_out: minimum_amount_out, // 实际输出量需要从事件日志获取
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in: is_exact_input,
    };

    Ok(DexEvent::BonkTrade(event))
}

/// 解析添加流动性指令 - 纯函数
pub fn parse_add_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 24 || accounts.len() < 15 {
        return Err(ParseError::InsufficientData);
    }

    let mut offset = 8;

    let max_token_a_amount = parse_u64(&data[offset..offset + 8])?;
    offset += 8;
    let max_token_b_amount = parse_u64(&data[offset..offset + 8])?;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    // 为添加流动性创建一个交易事件
    let event = BonkTradeEvent {
        metadata,
        pool_state: accounts[1],
        user: accounts[0],
        amount_in: max_token_a_amount,
        amount_out: max_token_b_amount,
        is_buy: true, // 添加流动性视为买入
        trade_direction: TradeDirection::Buy,
        exact_in: true,
    };

    Ok(DexEvent::BonkTrade(event))
}

/// 解析移除流动性指令 - 纯函数
pub fn parse_remove_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: i64,
) -> Result<DexEvent, ParseError> {
    if data.len() < 24 || accounts.len() < 12 {
        return Err(ParseError::InsufficientData);
    }

    let mut offset = 8;

    let lp_token_amount = parse_u64(&data[offset..offset + 8])?;
    offset += 8;
    let min_token_a_amount = parse_u64(&data[offset..offset + 8])?;

    let metadata = create_metadata(signature, slot, block_time, instruction_index, None)?;

    // 为移除流动性创建一个交易事件
    let event = BonkTradeEvent {
        metadata,
        pool_state: accounts[1],
        user: accounts[0],
        amount_in: lp_token_amount,
        amount_out: min_token_a_amount,
        is_buy: false, // 移除流动性视为卖出
        trade_direction: TradeDirection::Sell,
        exact_in: true,
    };

    Ok(DexEvent::BonkTrade(event))
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
        None => return Ok(None), // 不是 Bonk 指令
    };

    let result = match discriminator {
        CREATE_POOL_DISCRIMINATOR => {
            parse_create_pool(data, accounts, signature, slot, block_time, instruction_index)
        }
        SWAP_DISCRIMINATOR => {
            parse_swap(data, accounts, signature, slot, block_time, instruction_index)
        }
        ADD_LIQUIDITY_DISCRIMINATOR => {
            parse_add_liquidity(data, accounts, signature, slot, block_time, instruction_index)
        }
        REMOVE_LIQUIDITY_DISCRIMINATOR => {
            parse_remove_liquidity(data, accounts, signature, slot, block_time, instruction_index)
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
                Ok(None) => None,
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
        block_time: block_time,
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

/// 解析 u8 - 纯函数
fn parse_u8(data: &[u8]) -> Result<u8, ParseError> {
    if data.is_empty() {
        return Err(ParseError::InsufficientData);
    }
    Ok(data[0])
}

/// 解析布尔值 - 纯函数
fn parse_bool(data: &[u8]) -> Result<bool, ParseError> {
    if data.is_empty() {
        return Err(ParseError::InsufficientData);
    }
    Ok(data[0] != 0)
}

/// 确定交易方向 - 纯函数
fn determine_trade_direction(accounts: &[Pubkey]) -> Result<bool, ParseError> {
    // 这里简化处理，实际需要根据具体的账户结构判断
    // 通常通过输入/输出代币账户的类型来判断买卖方向
    Ok(true) // 默认为买入
}

/// 主解析函数 - 智能合并指令和日志数据，返回最完整的 DexEvent
pub fn parse_bonk_transaction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    use crate::parser::bonk_logs_parser;

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
        if bonk_logs_parser::is_bonk_log(log) {
            // TODO: 实现 Bonk 日志解析
            // if let Some(event) = bonk_logs_parser::parse_bonk_from_log_string(...) {
            //     log_event = Some(event);
            //     break;
            // }
        }
    }

    // 3. 智能合并：优先使用日志数据，用指令数据补充缺失字段
    match (instruction_event, log_event) {
        // 理想情况：同时有指令和日志数据，合并它们
        (Some(DexEvent::BonkTrade(ix_trade)), Some(DexEvent::BonkTrade(log_trade))) => {
            // TODO: 实现 Bonk 特定的合并逻辑
            Some(DexEvent::BonkTrade(ix_trade)) // 临时返回指令数据
        }

        // 只有日志数据：直接返回
        (None, Some(log_event)) => Some(log_event),

        // 只有指令数据：返回但数据不够完整
        (Some(ix_event), None) => Some(ix_event),

        // 都没有：返回None
        (None, None) => None,

        // 类型不匹配：优先返回日志数据
        (_, Some(log_event)) => Some(log_event),
    }
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

        // 测试交换判别符
        data[..8].copy_from_slice(SWAP_DISCRIMINATOR);
        assert_eq!(match_discriminator(&data), Some(SWAP_DISCRIMINATOR));

        // 测试创建池判别符
        data[..8].copy_from_slice(CREATE_POOL_DISCRIMINATOR);
        assert_eq!(match_discriminator(&data), Some(CREATE_POOL_DISCRIMINATOR));

        // 测试无效判别符
        data[..8].fill(0);
        assert_eq!(match_discriminator(&data), None);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool(&[1]), Ok(true));
        assert_eq!(parse_bool(&[0]), Ok(false));
        assert_eq!(parse_bool(&[255]), Ok(true));
        assert_eq!(parse_bool(&[]), Err(ParseError::InsufficientData));
    }

    #[test]
    fn test_parse_u8() {
        assert_eq!(parse_u8(&[42]), Ok(42));
        assert_eq!(parse_u8(&[]), Err(ParseError::InsufficientData));
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
        let mut instruction1 = vec![0u8; 50];
        instruction1[..8].copy_from_slice(SWAP_DISCRIMINATOR);
        instruction1[8..16].copy_from_slice(&1000u64.to_le_bytes());
        instruction1[16..24].copy_from_slice(&2000u64.to_le_bytes());
        instruction1[24] = 1; // is_exact_input = true

        let accounts = vec![Pubkey::default(); 15];
        let instructions = vec![(instruction1, accounts)];

        let results = parse_instructions_batch(&instructions, Signature::default(), 0, None);
        assert_eq!(results.len(), 1);

        // 验证是成功的交易事件
        match &results[0] {
            Ok(DexEvent::BonkTrade(_)) => (),
            _ => panic!("Expected BonkTrade event"),
        }
    }

    #[test]
    fn test_stream_parsing() {
        let mut instruction = vec![0u8; 50];
        instruction[..8].copy_from_slice(SWAP_DISCRIMINATOR);
        instruction[8..16].copy_from_slice(&1000u64.to_le_bytes());
        instruction[16..24].copy_from_slice(&2000u64.to_le_bytes());
        instruction[24] = 1;

        let accounts = vec![Pubkey::default(); 15];
        let instructions = vec![(instruction, accounts)];

        let events: Vec<_> = parse_instructions_stream(
            &instructions,
            Signature::default(),
            0,
            None,
        ).collect();

        assert_eq!(events.len(), 1);
        assert!(events[0].is_ok());
    }
}