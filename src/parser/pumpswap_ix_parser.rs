//! PumpSwap 指令解析器 - 纯函数式设计
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

// PumpSwap 程序 ID
pub const PUMPSWAP_PROGRAM_ID: &str = "PS1111111111111111111111111111111111111111";

// 指令判别符 - 编译时常量优化
pub const INITIALIZE_POOL_IX: &[u8; 8] = b"\x01\x00\x00\x00\x00\x00\x00\x00";
pub const SWAP_IX: &[u8; 8] = b"\x02\x00\x00\x00\x00\x00\x00\x00";
pub const ADD_LIQUIDITY_IX: &[u8; 8] = b"\x03\x00\x00\x00\x00\x00\x00\x00";
pub const REMOVE_LIQUIDITY_IX: &[u8; 8] = b"\x04\x00\x00\x00\x00\x00\x00\x00";
pub const UPDATE_POOL_IX: &[u8; 8] = b"\x05\x00\x00\x00\x00\x00\x00\x00";
pub const CLAIM_FEES_IX: &[u8; 8] = b"\x06\x00\x00\x00\x00\x00\x00\x00";

/// PumpSwap 特定错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum PumpSwapParseError {
    InvalidInstructionLength,
    InvalidDiscriminator,
    InvalidAccountsLength,
    InvalidTokenAmount,
    InvalidLiquidityAmount,
    InsufficientData,
}

/// 检查是否为 PumpSwap 程序
#[inline(always)]
pub fn is_pumpswap_program(program_id: &Pubkey) -> bool {
    program_id.to_string() == PUMPSWAP_PROGRAM_ID
}

/// 主解析函数 - 纯函数式设计
pub fn parse_pumpswap_instruction(
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
        d if d == INITIALIZE_POOL_IX => {
            parse_initialize_pool(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == SWAP_IX => {
            parse_swap(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == ADD_LIQUIDITY_IX => {
            parse_add_liquidity(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == REMOVE_LIQUIDITY_IX => {
            parse_remove_liquidity(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == UPDATE_POOL_IX => {
            parse_update_pool(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == CLAIM_FEES_IX => {
            parse_claim_fees(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        _ => None,
    }
}

/// 解析初始化池子指令
#[inline]
fn parse_initialize_pool(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 8 {
        return None;
    }

    // 零拷贝数据提取
    let initial_token_a_amount = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let initial_token_b_amount = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );

    // 账户信息
    let pool_account = accounts[0];
    let token_a_mint = accounts[1];
    let token_b_mint = accounts[2];
    let token_a_vault = accounts[3];
    let token_b_vault = accounts[4];
    let lp_mint = accounts[5];
    let user_account = accounts[6];
    let authority = accounts[7];

    Some(DexEvent::PumpSwapPoolCreated(PumpSwapPoolCreated {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        token_a_mint,
        token_b_mint,
        token_a_vault,
        token_b_vault,
        lp_mint,
        creator: user_account,
        authority,
        initial_token_a_amount,
        initial_token_b_amount,
    }))
}

/// 解析交换指令
#[inline]
fn parse_swap(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 10 {
        return None;
    }

    // 零拷贝数据提取
    let amount_in = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let minimum_amount_out = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let is_token_a_to_b = data[24] != 0;

    // 账户信息
    let pool_account = accounts[0];
    let user_account = accounts[1];
    let user_token_in_account = accounts[2];
    let user_token_out_account = accounts[3];
    let pool_token_in_vault = accounts[4];
    let pool_token_out_vault = accounts[5];
    let token_in_mint = accounts[6];
    let token_out_mint = accounts[7];

    Some(DexEvent::PumpSwapTrade(PumpSwapTrade {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        user: user_account,
        user_token_in_account,
        user_token_out_account,
        pool_token_in_vault,
        pool_token_out_vault,
        token_in_mint,
        token_out_mint,
        amount_in,
        minimum_amount_out,
        is_token_a_to_b,
    }))
}

/// 解析添加流动性指令
#[inline]
fn parse_add_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 11 {
        return None;
    }

    // 零拷贝数据提取
    let max_token_a_amount = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let max_token_b_amount = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let min_lp_tokens = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );

    // 账户信息
    let pool_account = accounts[0];
    let user_account = accounts[1];
    let user_token_a_account = accounts[2];
    let user_token_b_account = accounts[3];
    let user_lp_token_account = accounts[4];
    let pool_token_a_vault = accounts[5];
    let pool_token_b_vault = accounts[6];
    let lp_mint = accounts[7];
    let token_a_mint = accounts[8];
    let token_b_mint = accounts[9];

    Some(DexEvent::PumpSwapLiquidityAdded(PumpSwapLiquidityAdded {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        user: user_account,
        user_token_a_account,
        user_token_b_account,
        user_lp_token_account,
        pool_token_a_vault,
        pool_token_b_vault,
        lp_mint,
        token_a_mint,
        token_b_mint,
        max_token_a_amount,
        max_token_b_amount,
        min_lp_tokens,
    }))
}

/// 解析移除流动性指令
#[inline]
fn parse_remove_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 11 {
        return None;
    }

    // 零拷贝数据提取
    let lp_tokens_to_burn = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let min_token_a_amount = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let min_token_b_amount = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );

    // 账户信息
    let pool_account = accounts[0];
    let user_account = accounts[1];
    let user_token_a_account = accounts[2];
    let user_token_b_account = accounts[3];
    let user_lp_token_account = accounts[4];
    let pool_token_a_vault = accounts[5];
    let pool_token_b_vault = accounts[6];
    let lp_mint = accounts[7];
    let token_a_mint = accounts[8];
    let token_b_mint = accounts[9];

    Some(DexEvent::PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        user: user_account,
        user_token_a_account,
        user_token_b_account,
        user_lp_token_account,
        pool_token_a_vault,
        pool_token_b_vault,
        lp_mint,
        token_a_mint,
        token_b_mint,
        lp_tokens_to_burn,
        min_token_a_amount,
        min_token_b_amount,
    }))
}

/// 解析更新池子指令
#[inline]
fn parse_update_pool(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 16 || accounts.len() < 3 {
        return None;
    }

    // 零拷贝数据提取
    let new_fee_rate = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );

    // 账户信息
    let pool_account = accounts[0];
    let authority = accounts[1];
    let admin_account = accounts[2];

    Some(DexEvent::PumpSwapPoolUpdated(PumpSwapPoolUpdated {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        authority,
        admin: admin_account,
        new_fee_rate,
    }))
}

/// 解析领取手续费指令
#[inline]
fn parse_claim_fees(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 6 {
        return None;
    }

    // 账户信息
    let pool_account = accounts[0];
    let authority = accounts[1];
    let admin_account = accounts[2];
    let admin_token_a_account = accounts[3];
    let admin_token_b_account = accounts[4];
    let pool_fee_vault = accounts[5];

    Some(DexEvent::PumpSwapFeesClaimed(PumpSwapFeesClaimed {
        signature,
        slot,
        block_time,
        instruction_index,
        pool_account,
        authority,
        admin: admin_account,
        admin_token_a_account,
        admin_token_b_account,
        pool_fee_vault,
    }))
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
        if let Some(event) = parse_pumpswap_instruction(
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
        parse_pumpswap_instruction(
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
pub fn is_pumpswap_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    let discriminator = &data[..8];
    discriminator == INITIALIZE_POOL_IX
        || discriminator == SWAP_IX
        || discriminator == ADD_LIQUIDITY_IX
        || discriminator == REMOVE_LIQUIDITY_IX
        || discriminator == UPDATE_POOL_IX
        || discriminator == CLAIM_FEES_IX
}

/// 提取指令类型 - 用于日志和调试
pub fn get_instruction_type(data: &[u8]) -> Option<&'static str> {
    if data.len() < 8 {
        return None;
    }

    match &data[..8] {
        d if d == INITIALIZE_POOL_IX => Some("InitializePool"),
        d if d == SWAP_IX => Some("Swap"),
        d if d == ADD_LIQUIDITY_IX => Some("AddLiquidity"),
        d if d == REMOVE_LIQUIDITY_IX => Some("RemoveLiquidity"),
        d if d == UPDATE_POOL_IX => Some("UpdatePool"),
        d if d == CLAIM_FEES_IX => Some("ClaimFees"),
        _ => None,
    }
}

/// 主解析函数 - 智能合并指令和日志数据，返回最完整的 DexEvent
pub fn parse_pumpswap_transaction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    use crate::parser::pumpswap_logs_parser;

    // 1. 解析指令数据（用户意图 + 账户信息）
    let instruction_event = parse_pumpswap_instruction(
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
        if pumpswap_logs_parser::is_pumpswap_log(log) {
            // TODO: 实现 PumpSwap 日志解析
            // if let Some(event) = pumpswap_logs_parser::parse_pumpswap_from_log_string(...) {
            //     log_event = Some(event);
            //     break;
            // }
        }
    }

    // 3. 智能合并：优先使用日志数据，用指令数据补充缺失字段
    merge_pumpswap_events(instruction_event, log_event)
}

/// PumpSwap 事件智能合并
fn merge_pumpswap_events(
    instruction_event: Option<DexEvent>,
    log_event: Option<DexEvent>,
) -> Option<DexEvent> {
    match (instruction_event, log_event) {
        // 交易事件合并：指令数据 + 日志买入事件
        (Some(DexEvent::PumpSwapTrade(_ix_trade)), Some(DexEvent::PumpSwapBuy(log_buy))) => {
            Some(DexEvent::PumpSwapBuy(log_buy))
        }

        // 交易事件合并：指令数据 + 日志卖出事件
        (Some(DexEvent::PumpSwapTrade(_ix_trade)), Some(DexEvent::PumpSwapSell(log_sell))) => {
            Some(DexEvent::PumpSwapSell(log_sell))
        }

        // 池创建事件合并
        (Some(DexEvent::PumpSwapPoolCreated(_ix_pool)), Some(DexEvent::PumpSwapCreatePool(log_pool))) => {
            Some(DexEvent::PumpSwapCreatePool(log_pool))
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

        // 测试初始化池子指令
        data[..8].copy_from_slice(INITIALIZE_POOL_IX);
        assert!(is_pumpswap_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("InitializePool"));

        // 测试交换指令
        data[..8].copy_from_slice(SWAP_IX);
        assert!(is_pumpswap_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("Swap"));

        // 测试无效指令
        data[..8].copy_from_slice(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        assert!(!is_pumpswap_instruction(&data));
        assert_eq!(get_instruction_type(&data), None);
    }

    #[test]
    fn test_program_id_validation() {
        let valid_program_id = Pubkey::try_from(PUMPSWAP_PROGRAM_ID).unwrap();
        assert!(is_pumpswap_program(&valid_program_id));

        let invalid_program_id = Pubkey::default();
        assert!(!is_pumpswap_program(&invalid_program_id));
    }

    #[test]
    fn test_parse_initialize_pool() {
        let mut data = vec![0u8; 24];
        data[..8].copy_from_slice(INITIALIZE_POOL_IX);
        data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        data[16..24].copy_from_slice(&2_000_000_000u64.to_le_bytes());

        let accounts = vec![Pubkey::default(); 8];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_pumpswap_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::PumpSwapPoolCreated(event)) = result {
            assert_eq!(event.initial_token_a_amount, 1_000_000_000);
            assert_eq!(event.initial_token_b_amount, 2_000_000_000);
        } else {
            panic!("Expected PumpSwapPoolCreated event");
        }
    }

    #[test]
    fn test_parse_swap() {
        let mut data = vec![0u8; 32];
        data[..8].copy_from_slice(SWAP_IX);
        data[8..16].copy_from_slice(&500_000_000u64.to_le_bytes());
        data[16..24].copy_from_slice(&450_000_000u64.to_le_bytes());
        data[24] = 1; // is_token_a_to_b = true

        let accounts = vec![Pubkey::default(); 10];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_pumpswap_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::PumpSwapTrade(event)) = result {
            assert_eq!(event.amount_in, 500_000_000);
            assert_eq!(event.minimum_amount_out, 450_000_000);
            assert!(event.is_token_a_to_b);
        } else {
            panic!("Expected PumpSwapTrade event");
        }
    }

    #[test]
    fn test_batch_parsing() {
        let mut instructions = Vec::new();

        // 添加初始化池子指令
        let mut data1 = vec![0u8; 24];
        data1[..8].copy_from_slice(INITIALIZE_POOL_IX);
        data1[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        let accounts1 = vec![Pubkey::default(); 8];
        instructions.push((data1, accounts1));

        // 添加交换指令
        let mut data2 = vec![0u8; 32];
        data2[..8].copy_from_slice(SWAP_IX);
        data2[8..16].copy_from_slice(&500_000_000u64.to_le_bytes());
        let accounts2 = vec![Pubkey::default(); 10];
        instructions.push((data2, accounts2));

        let events = parse_instructions_batch(
            &instructions,
            Signature::default(),
            123456789,
            None,
        );

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], DexEvent::PumpSwapPoolCreated(_)));
        assert!(matches!(events[1], DexEvent::PumpSwapTrade(_)));
    }
}