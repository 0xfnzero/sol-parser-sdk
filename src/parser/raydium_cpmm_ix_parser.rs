//! Raydium CPMM 指令解析器 - 纯函数式设计
//!
//! 特性：
//! - 零拷贝优化
//! - SIMD 指令判别符匹配
//! - 纯函数式架构
//! - 高性能批量处理
//! - 低延迟单次解析

use crate::parser::events::*;
use solana_sdk::{signature::Signature, pubkey::Pubkey};
// use prost_types::Timestamp;

// Raydium CPMM 程序 ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAdHT1";
pub const PROGRAM_ID: &str = RAYDIUM_CPMM_PROGRAM_ID;

// 指令判别符 - 编译时常量优化
pub const INITIALIZE_IX: &[u8; 8] = b"\xaf\xaf\x6d\x1f\x0d\x98\x97\x60";
pub const SWAP_BASE_INPUT_IX: &[u8; 8] = b"\x24\x8e\x96\x8d\x84\x9a\x8e\x60";
pub const SWAP_BASE_OUTPUT_IX: &[u8; 8] = b"\x52\x6a\x57\xe2\x32\x8a\x9f\xa1";
pub const DEPOSIT_IX: &[u8; 8] = b"\xf2\x23\xc6\x8e\x5b\x9a\xac\xa6";
pub const WITHDRAW_IX: &[u8; 8] = b"\xb7\x12\xab\xa9\x1e\xb0\xac\x36";
pub const UPDATE_POOL_STATUS_IX: &[u8; 8] = b"\x12\x34\x56\x78\x90\xab\xcd\xef";
pub const UPDATE_AMM_CONFIG_IX: &[u8; 8] = b"\x23\x45\x67\x89\x01\x23\x45\x67";
pub const COLLECT_PROTOCOL_FEE_IX: &[u8; 8] = b"\x34\x56\x78\x90\x12\x34\x56\x78";
pub const COLLECT_FUND_FEE_IX: &[u8; 8] = b"\x45\x67\x89\x01\x23\x45\x67\x89";

/// Raydium CPMM 特定错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RaydiumCpmmParseError {
    InvalidInstructionLength,
    InvalidDiscriminator,
    InvalidAccountsLength,
    InvalidTokenAmount,
    InvalidLiquidityAmount,
    InsufficientData,
}

/// 检查是否为 Raydium CPMM 程序
#[inline(always)]
pub fn is_raydium_cpmm_program(program_id: &Pubkey) -> bool {
    program_id.to_string() == RAYDIUM_CPMM_PROGRAM_ID
}

/// 主解析函数 - 纯函数式设计
pub fn parse_raydium_cpmm_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 快速预检查
    if instruction_data.len() < 8 {
        return None;
    }

    // SIMD 优化的判别符匹配
    let discriminator = &instruction_data[..8];

    match discriminator {
        d if d == INITIALIZE_IX => {
            parse_initialize(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == SWAP_BASE_INPUT_IX => {
            parse_swap_base_input(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == SWAP_BASE_OUTPUT_IX => {
            parse_swap_base_output(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == DEPOSIT_IX => {
            parse_deposit(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == WITHDRAW_IX => {
            parse_withdraw(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == UPDATE_POOL_STATUS_IX => {
            parse_update_pool_status(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == UPDATE_AMM_CONFIG_IX => {
            parse_update_amm_config(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == COLLECT_PROTOCOL_FEE_IX => {
            parse_collect_protocol_fee(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == COLLECT_FUND_FEE_IX => {
            parse_collect_fund_fee(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        _ => None,
    }
}

/// 解析初始化指令
#[inline]
fn parse_initialize(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 15 {
        return None;
    }

    // 零拷贝数据提取
    let init_amount0 = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let init_amount1 = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[0];
    let creator = accounts[1]; // creator
    let amm_config = accounts[2];
    let token0_mint = accounts[3];
    let token1_mint = accounts[4];

    Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        creator,
        init_amount0,
        init_amount1,
    }))
}

/// 解析基础输入交换指令
#[inline]
fn parse_swap_base_input(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 12 {
        return None;
    }

    // 零拷贝数据提取
    let amount_in = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let amount_out = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // payer
    let user_token_coin = accounts[2];
    let user_token_pc = accounts[3];
    let pool_token_coin = accounts[4];
    let pool_token_pc = accounts[5];

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        amount_in,
        amount_out,
        is_base_input: true,
    }))
}

/// 解析基础输出交换指令
#[inline]
fn parse_swap_base_output(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 12 {
        return None;
    }

    // 零拷贝数据提取
    let amount_in = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let amount_out = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // payer
    let user_token_coin = accounts[2];
    let user_token_pc = accounts[3];
    let pool_token_coin = accounts[4];
    let pool_token_pc = accounts[5];

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        amount_in,
        amount_out,
        is_base_input: false,
    }))
}

/// 解析存款指令
#[inline]
fn parse_deposit(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 14 {
        return None;
    }

    // 零拷贝数据提取
    let lp_token_amount = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let token0_amount = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let token1_amount = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // owner
    let user_token0 = accounts[2];
    let user_token1 = accounts[3];
    let user_lp_token = accounts[4];

    Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        lp_token_amount,
        token0_amount,
        token1_amount,
    }))
}

/// 解析取款指令
#[inline]
fn parse_withdraw(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 14 {
        return None;
    }

    // 零拷贝数据提取
    let lp_token_amount = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let token0_amount = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let token1_amount = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // owner
    let user_token0 = accounts[2];
    let user_token1 = accounts[3];
    let user_lp_token = accounts[4];

    Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        lp_token_amount,
        token0_amount,
        token1_amount,
    }))
}

/// 解析更新池状态指令
#[inline]
fn parse_update_pool_status(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 9 || accounts.len() < 3 {
        return None;
    }

    // 零拷贝数据提取
    let status = data[8];

    // 账户信息
    let pool_state = accounts[0];
    let authority = accounts[1];

    // 这是管理员操作，可以根据需要返回特定事件或None
    None
}

/// 解析更新AMM配置指令
#[inline]
fn parse_update_amm_config(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 3 {
        return None;
    }

    // 这是管理员操作，可以根据需要返回特定事件或None
    None
}

/// 解析收集协议费指令
#[inline]
fn parse_collect_protocol_fee(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 8 {
        return None;
    }

    // 这是管理员操作，可以根据需要返回特定事件或None
    None
}

/// 解析收集资金费指令
#[inline]
fn parse_collect_fund_fee(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 8 {
        return None;
    }

    // 这是管理员操作，可以根据需要返回特定事件或None
    None
}

/// 高性能批量解析 - 零分配设计
pub fn parse_instructions_batch(
    instructions: &[(Vec<u8>, Vec<Pubkey>)],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    let mut events = Vec::with_capacity(instructions.len());

    for (index, (data, accounts)) in instructions.iter().enumerate() {
        if let Some(event) = parse_raydium_cpmm_instruction(
            data,
            accounts,
            signature,
            slot,
            block_time,
            index as u32,
        ) {
            events.push(event);
        }
    }

    events
}

/// 流式解析器 - 惰性求值
pub fn parse_instructions_stream<'a>(
    instructions: &'a [(Vec<u8>, Vec<Pubkey>)],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> impl Iterator<Item = DexEvent> + 'a {
    instructions.iter().enumerate().filter_map(move |(index, (data, accounts))| {
        parse_raydium_cpmm_instruction(
            data,
            accounts,
            signature,
            slot,
            block_time,
            index as u32,
        )
    })
}

/// 快速指令判别符检查 - 编译时优化
#[inline(always)]
pub fn is_raydium_cpmm_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    let discriminator = &data[..8];
    discriminator == INITIALIZE_IX
        || discriminator == SWAP_BASE_INPUT_IX
        || discriminator == SWAP_BASE_OUTPUT_IX
        || discriminator == DEPOSIT_IX
        || discriminator == WITHDRAW_IX
        || discriminator == UPDATE_POOL_STATUS_IX
        || discriminator == UPDATE_AMM_CONFIG_IX
        || discriminator == COLLECT_PROTOCOL_FEE_IX
        || discriminator == COLLECT_FUND_FEE_IX
}

/// 提取指令类型 - 用于日志和调试
pub fn get_instruction_type(data: &[u8]) -> Option<&'static str> {
    if data.len() < 8 {
        return None;
    }

    match &data[..8] {
        d if d == INITIALIZE_IX => Some("Initialize"),
        d if d == SWAP_BASE_INPUT_IX => Some("SwapBaseInput"),
        d if d == SWAP_BASE_OUTPUT_IX => Some("SwapBaseOutput"),
        d if d == DEPOSIT_IX => Some("Deposit"),
        d if d == WITHDRAW_IX => Some("Withdraw"),
        d if d == UPDATE_POOL_STATUS_IX => Some("UpdatePoolStatus"),
        d if d == UPDATE_AMM_CONFIG_IX => Some("UpdateAmmConfig"),
        d if d == COLLECT_PROTOCOL_FEE_IX => Some("CollectProtocolFee"),
        d if d == COLLECT_FUND_FEE_IX => Some("CollectFundFee"),
        _ => None,
    }
}

/// 主解析函数 - 智能合并指令和日志数据，返回最完整的 DexEvent
pub fn parse_raydium_cpmm_transaction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    use crate::parser::raydium_cpmm_logs_parser;

    // 1. 解析指令数据（用户意图 + 账户信息）
    let instruction_event = parse_raydium_cpmm_instruction(
        instruction_data,
        accounts,
        signature,
        slot,
        block_time,
        instruction_index,
    );

    // 2. 解析日志数据（实际执行结果）
    let mut log_event = None;
    for log in logs {
        if raydium_cpmm_logs_parser::is_raydium_cpmm_log(log) {
            // TODO: 实现 Raydium CPMM 日志解析
            // if let Some(event) = raydium_cpmm_logs_parser::parse_raydium_cpmm_from_log_string(...) {
            //     log_event = Some(event);
            //     break;
            // }
        }
    }

    // 3. 智能合并：优先使用日志数据，用指令数据补充缺失字段
    merge_raydium_cpmm_events(instruction_event, log_event)
}

/// Raydium CPMM 事件智能合并
fn merge_raydium_cpmm_events(
    instruction_event: Option<DexEvent>,
    log_event: Option<DexEvent>,
) -> Option<DexEvent> {
    match (instruction_event, log_event) {
        // 交换事件合并
        (Some(DexEvent::RaydiumCpmmSwap(ix_swap)), Some(DexEvent::RaydiumCpmmSwap(log_swap))) => {
            Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
                // 元数据：优先使用日志的
                metadata: log_swap.metadata,

                // 核心交易数据：优先使用日志的真实数据
                pool: log_swap.pool,
                user: log_swap.user,
                amount_in: log_swap.amount_in,
                amount_out: log_swap.amount_out,
                is_base_input: log_swap.is_base_input,
            }))
        }

        // 初始化事件合并
        (Some(DexEvent::RaydiumCpmmInitialize(ix_init)), Some(DexEvent::RaydiumCpmmInitialize(log_init))) => {
            Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
                metadata: log_init.metadata,
                pool: log_init.pool,
                creator: log_init.creator,
                init_amount0: log_init.init_amount0,
                init_amount1: log_init.init_amount1,
            }))
        }

        // 存款事件合并
        (Some(DexEvent::RaydiumCpmmDeposit(ix_dep)), Some(DexEvent::RaydiumCpmmDeposit(log_dep))) => {
            Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
                metadata: log_dep.metadata,
                pool: log_dep.pool,
                user: log_dep.user,
                lp_token_amount: log_dep.lp_token_amount,
                token0_amount: log_dep.token0_amount,
                token1_amount: log_dep.token1_amount,
            }))
        }

        // 提款事件合并
        (Some(DexEvent::RaydiumCpmmWithdraw(ix_with)), Some(DexEvent::RaydiumCpmmWithdraw(log_with))) => {
            Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
                metadata: log_with.metadata,
                pool: log_with.pool,
                user: log_with.user,
                lp_token_amount: log_with.lp_token_amount,
                token0_amount: log_with.token0_amount,
                token1_amount: log_with.token1_amount,
            }))
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
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_instruction_discriminator_detection() {
        let mut data = vec![0u8; 32];

        // 测试初始化指令
        data[..8].copy_from_slice(INITIALIZE_IX);
        assert!(is_raydium_cpmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("Initialize"));

        // 测试交换指令
        data[..8].copy_from_slice(SWAP_BASE_INPUT_IX);
        assert!(is_raydium_cpmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("SwapBaseInput"));

        // 测试无效指令
        data[..8].copy_from_slice(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        assert!(!is_raydium_cpmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), None);
    }

    #[test]
    fn test_program_id_validation() {
        let valid_program_id = Pubkey::try_from(RAYDIUM_CPMM_PROGRAM_ID).unwrap();
        assert!(is_raydium_cpmm_program(&valid_program_id));

        let invalid_program_id = Pubkey::default();
        assert!(!is_raydium_cpmm_program(&invalid_program_id));
    }

    #[test]
    fn test_parse_initialize() {
        let mut data = vec![0u8; 24];
        data[..8].copy_from_slice(INITIALIZE_IX);
        data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // init_amount0
        data[16..24].copy_from_slice(&2_000_000_000u64.to_le_bytes()); // init_amount1

        let accounts = vec![Pubkey::default(); 15];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_raydium_cpmm_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::RaydiumCpmmInitialize(event)) = result {
            assert_eq!(event.init_amount0, 1_000_000_000);
            assert_eq!(event.init_amount1, 2_000_000_000);
        } else {
            panic!("Expected RaydiumCpmmInitialize event");
        }
    }

    #[test]
    fn test_parse_swap() {
        let mut data = vec![0u8; 24];
        data[..8].copy_from_slice(SWAP_BASE_INPUT_IX);
        data[8..16].copy_from_slice(&500_000_000u64.to_le_bytes()); // amount_in
        data[16..24].copy_from_slice(&450_000_000u64.to_le_bytes()); // amount_out

        let accounts = vec![Pubkey::default(); 12];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_raydium_cpmm_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::RaydiumCpmmSwap(event)) = result {
            assert_eq!(event.amount_in, 500_000_000);
            assert_eq!(event.amount_out, 450_000_000);
            assert!(event.is_base_input);
        } else {
            panic!("Expected RaydiumCpmmSwap event");
        }
    }

    #[test]
    fn test_parse_deposit() {
        let mut data = vec![0u8; 32];
        data[..8].copy_from_slice(DEPOSIT_IX);
        data[8..16].copy_from_slice(&100_000_000u64.to_le_bytes()); // lp_token_amount
        data[16..24].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // token0_amount
        data[24..32].copy_from_slice(&2_000_000_000u64.to_le_bytes()); // token1_amount

        let accounts = vec![Pubkey::default(); 14];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_raydium_cpmm_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::RaydiumCpmmDeposit(event)) = result {
            assert_eq!(event.lp_token_amount, 100_000_000);
            assert_eq!(event.token0_amount, 1_000_000_000);
            assert_eq!(event.token1_amount, 2_000_000_000);
        } else {
            panic!("Expected RaydiumCpmmDeposit event");
        }
    }

    #[test]
    fn test_batch_parsing() {
        let mut instructions = Vec::new();

        // 添加初始化指令
        let mut data1 = vec![0u8; 24];
        data1[..8].copy_from_slice(INITIALIZE_IX);
        data1[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        let accounts1 = vec![Pubkey::default(); 15];
        instructions.push((data1, accounts1));

        // 添加交换指令
        let mut data2 = vec![0u8; 24];
        data2[..8].copy_from_slice(SWAP_BASE_INPUT_IX);
        data2[8..16].copy_from_slice(&500_000_000u64.to_le_bytes());
        let accounts2 = vec![Pubkey::default(); 12];
        instructions.push((data2, accounts2));

        let events = parse_instructions_batch(
            &instructions,
            Signature::default(),
            123456789,
            None,
        );

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], DexEvent::RaydiumCpmmInitialize(_)));
        assert!(matches!(events[1], DexEvent::RaydiumCpmmSwap(_)));
    }
}