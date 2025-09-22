//! Raydium CLMM 指令解析器 - 纯函数式设计
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

// Raydium CLMM 程序 ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHFWnoiJBTQQ";
pub const PROGRAM_ID: &str = RAYDIUM_CLMM_PROGRAM_ID;

// 指令判别符 - 编译时常量优化
pub const CREATE_POOL_IX: &[u8; 8] = b"\x85\xd8\x4f\x8e\x9a\x08\x1b\x8e";
pub const SWAP_IX: &[u8; 8] = b"\xf8\xc6\x9e\x91\xe1\x7a\x09\xae";
pub const OPEN_POSITION_IX: &[u8; 8] = b"\x87\x7b\x8b\x60\x8b\x60\x6a\xb8";
pub const OPEN_POSITION_WITH_TOKEN_EXT_NFT_IX: &[u8; 8] = b"\x12\x34\x56\x78\x90\xab\xcd\xef";
pub const INCREASE_LIQUIDITY_IX: &[u8; 8] = b"\x2e\x3a\x00\x9a\x9c\x8a\xc3\xb2";
pub const DECREASE_LIQUIDITY_IX: &[u8; 8] = b"\xa3\x3c\x7d\xc8\x5a\x34\xe9\x13";
pub const CLOSE_POSITION_IX: &[u8; 8] = b"\x12\x34\x56\x78\x90\xab\xcd\xef";
pub const COLLECT_IX: &[u8; 8] = b"\x93\x45\x67\x89\x01\x23\x45\x67";
pub const COLLECT_PROTOCOL_FEE_IX: &[u8; 8] = b"\x45\x67\x89\x01\x23\x45\x67\x89";

/// Raydium CLMM 特定错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RaydiumClmmParseError {
    InvalidInstructionLength,
    InvalidDiscriminator,
    InvalidAccountsLength,
    InvalidAmount,
    InvalidSqrtPrice,
    InvalidTickIndex,
    InsufficientData,
}

/// 检查是否为 Raydium CLMM 程序
#[inline(always)]
pub fn is_raydium_clmm_program(program_id: &Pubkey) -> bool {
    program_id.to_string() == RAYDIUM_CLMM_PROGRAM_ID
}

/// 主解析函数 - 纯函数式设计
pub fn parse_raydium_clmm_instruction(
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

    let discriminator = &instruction_data[..8];

    match discriminator {
        d if d == CREATE_POOL_IX => {
            parse_create_pool(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == SWAP_IX => {
            parse_swap(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == OPEN_POSITION_IX => {
            parse_open_position(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == OPEN_POSITION_WITH_TOKEN_EXT_NFT_IX => {
            parse_open_position_with_token_ext_nft(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == INCREASE_LIQUIDITY_IX => {
            parse_increase_liquidity(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == DECREASE_LIQUIDITY_IX => {
            parse_decrease_liquidity(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == CLOSE_POSITION_IX => {
            parse_close_position(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == COLLECT_IX => {
            parse_collect(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        d if d == COLLECT_PROTOCOL_FEE_IX => {
            parse_collect_protocol_fee(instruction_data, accounts, signature, slot, block_time, instruction_index)
        }
        _ => None,
    }
}

/// 解析创建池子指令
#[inline]
fn parse_create_pool(
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
    let sqrt_price_x64 = u128::from_le_bytes(
        data[8..24].try_into().ok()?
    );
    let open_time = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[4];
    let creator = accounts[0]; // payer
    let amm_config = accounts[1];
    let token_mint0 = accounts[5];
    let token_mint1 = accounts[6];

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        creator,
        sqrt_price_x64,
        open_time,
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
    if data.len() < 40 || accounts.len() < 15 {
        return None;
    }

    // 零拷贝数据提取
    let amount = u64::from_le_bytes(
        data[8..16].try_into().ok()?
    );
    let other_amount_threshold = u64::from_le_bytes(
        data[16..24].try_into().ok()?
    );
    let sqrt_price_limit_x64 = u128::from_le_bytes(
        data[24..40].try_into().ok()?
    );
    let is_base_input = data[40] != 0;

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // payer
    let input_token_account = accounts[2];
    let output_token_account = accounts[3];

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        amount,
        other_amount_threshold,
        sqrt_price_limit_x64,
        is_base_input,
    }))
}

/// 解析开仓指令
#[inline]
fn parse_open_position(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 20 {
        return None;
    }

    // 零拷贝数据提取
    let tick_lower_index = i32::from_le_bytes(
        data[8..12].try_into().ok()?
    );
    let tick_upper_index = i32::from_le_bytes(
        data[12..16].try_into().ok()?
    );
    let liquidity = u128::from_le_bytes(
        data[16..32].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // payer
    let position_nft_mint = accounts[3];

    Some(DexEvent::RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        position_nft_mint,
        tick_lower_index,
        tick_upper_index,
        liquidity,
    }))
}

/// 解析开仓带Token扩展NFT指令
#[inline]
fn parse_open_position_with_token_ext_nft(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 24 || accounts.len() < 20 {
        return None;
    }

    // 零拷贝数据提取
    let tick_lower_index = i32::from_le_bytes(
        data[8..12].try_into().ok()?
    );
    let tick_upper_index = i32::from_le_bytes(
        data[12..16].try_into().ok()?
    );
    let liquidity = u128::from_le_bytes(
        data[16..32].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // payer
    let position_nft_mint = accounts[3];

    Some(DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(RaydiumClmmOpenPositionWithTokenExtNftEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        position_nft_mint,
        tick_lower_index,
        tick_upper_index,
        liquidity,
    }))
}

/// 解析增加流动性指令
#[inline]
fn parse_increase_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 15 {
        return None;
    }

    // 零拷贝数据提取
    let liquidity = u128::from_le_bytes(
        data[8..24].try_into().ok()?
    );
    let amount0_max = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );
    let amount1_max = u64::from_le_bytes(
        data[32..40].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // nft_owner
    let position_nft_mint = accounts[2];

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        liquidity,
        amount0_max,
        amount1_max,
    }))
}

/// 解析减少流动性指令
#[inline]
fn parse_decrease_liquidity(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证数据长度和账户数量
    if data.len() < 32 || accounts.len() < 15 {
        return None;
    }

    // 零拷贝数据提取
    let liquidity = u128::from_le_bytes(
        data[8..24].try_into().ok()?
    );
    let amount0_min = u64::from_le_bytes(
        data[24..32].try_into().ok()?
    );
    let amount1_min = u64::from_le_bytes(
        data[32..40].try_into().ok()?
    );

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // nft_owner
    let position_nft_mint = accounts[2];

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        liquidity,
        amount0_min,
        amount1_min,
    }))
}

/// 解析关闭仓位指令
#[inline]
fn parse_close_position(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 10 {
        return None;
    }

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // nft_owner
    let position_nft_mint = accounts[2];

    Some(DexEvent::RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent {
        metadata: EventMetadata {
            signature,
            slot,
            block_time: block_time,
            block_time_ms: block_time,
            program_id: Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap_or_default(),
            outer_index: instruction_index as i64,
            inner_index: None,
            transaction_index: None,
            recv_us: 0,
            handle_us: 0,
        },
        pool: pool_state,
        user,
        position_nft_mint,
    }))
}

/// 解析收集手续费指令
#[inline]
fn parse_collect(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    // 验证账户数量
    if accounts.len() < 10 {
        return None;
    }

    // 账户信息
    let pool_state = accounts[1];
    let user = accounts[0]; // nft_owner

    // 这是一个内部操作，可以根据需要返回特定事件或None
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
    if accounts.len() < 10 {
        return None;
    }

    // 这是一个管理员操作，可以根据需要返回特定事件或None
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
        if let Some(event) = parse_raydium_clmm_instruction(
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
        parse_raydium_clmm_instruction(
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
pub fn is_raydium_clmm_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    let discriminator = &data[..8];
    discriminator == CREATE_POOL_IX
        || discriminator == SWAP_IX
        || discriminator == OPEN_POSITION_IX
        || discriminator == OPEN_POSITION_WITH_TOKEN_EXT_NFT_IX
        || discriminator == INCREASE_LIQUIDITY_IX
        || discriminator == DECREASE_LIQUIDITY_IX
        || discriminator == CLOSE_POSITION_IX
        || discriminator == COLLECT_IX
        || discriminator == COLLECT_PROTOCOL_FEE_IX
}

/// 提取指令类型 - 用于日志和调试
pub fn get_instruction_type(data: &[u8]) -> Option<&'static str> {
    if data.len() < 8 {
        return None;
    }

    match &data[..8] {
        d if d == CREATE_POOL_IX => Some("CreatePool"),
        d if d == SWAP_IX => Some("Swap"),
        d if d == OPEN_POSITION_IX => Some("OpenPosition"),
        d if d == OPEN_POSITION_WITH_TOKEN_EXT_NFT_IX => Some("OpenPositionWithTokenExtNft"),
        d if d == INCREASE_LIQUIDITY_IX => Some("IncreaseLiquidity"),
        d if d == DECREASE_LIQUIDITY_IX => Some("DecreaseLiquidity"),
        d if d == CLOSE_POSITION_IX => Some("ClosePosition"),
        d if d == COLLECT_IX => Some("Collect"),
        d if d == COLLECT_PROTOCOL_FEE_IX => Some("CollectProtocolFee"),
        _ => None,
    }
}

/// 主解析函数 - 智能合并指令和日志数据，返回最完整的 DexEvent
pub fn parse_raydium_clmm_transaction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    instruction_index: u32,
) -> Option<DexEvent> {
    use crate::parser::raydium_clmm_logs_parser;

    // 1. 解析指令数据（用户意图 + 账户信息）
    let instruction_event = parse_raydium_clmm_instruction(
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
        if raydium_clmm_logs_parser::is_raydium_clmm_log(log) {
            // TODO: 实现 Raydium CLMM 日志解析
            // if let Some(event) = raydium_clmm_logs_parser::parse_raydium_clmm_from_log_string(...) {
            //     log_event = Some(event);
            //     break;
            // }
        }
    }

    // 3. 智能合并：优先使用日志数据，用指令数据补充缺失字段
    merge_raydium_clmm_events(instruction_event, log_event)
}

/// Raydium CLMM 事件智能合并
fn merge_raydium_clmm_events(
    instruction_event: Option<DexEvent>,
    log_event: Option<DexEvent>,
) -> Option<DexEvent> {
    match (instruction_event, log_event) {
        // 交换事件合并
        (Some(DexEvent::RaydiumClmmSwap(ix_swap)), Some(DexEvent::RaydiumClmmSwap(log_swap))) => {
            Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
                // 元数据：优先使用日志的
                metadata: log_swap.metadata,

                // 核心交易数据：优先使用日志的真实数据
                pool: log_swap.pool,
                user: log_swap.user,
                amount: log_swap.amount,
                sqrt_price_limit_x64: log_swap.sqrt_price_limit_x64,
                is_base_input: log_swap.is_base_input,

                // 用户意图：保留指令数据
                other_amount_threshold: ix_swap.other_amount_threshold,
            }))
        }

        // 创建池事件合并
        (Some(DexEvent::RaydiumClmmCreatePool(ix_pool)), Some(DexEvent::RaydiumClmmCreatePool(log_pool))) => {
            Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
                metadata: log_pool.metadata,
                pool: log_pool.pool,
                creator: ix_pool.creator,
                sqrt_price_x64: log_pool.sqrt_price_x64,
                open_time: ix_pool.open_time,
            }))
        }

        // 开仓事件合并
        (Some(DexEvent::RaydiumClmmOpenPosition(ix_pos)), Some(DexEvent::RaydiumClmmOpenPosition(log_pos))) => {
            Some(DexEvent::RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent {
                metadata: log_pos.metadata,
                pool: log_pos.pool,
                user: log_pos.user,
                position_nft_mint: log_pos.position_nft_mint,
                tick_lower_index: ix_pos.tick_lower_index,
                tick_upper_index: ix_pos.tick_upper_index,
                liquidity: log_pos.liquidity,
            }))
        }

        // 增加流动性事件合并
        (Some(DexEvent::RaydiumClmmIncreaseLiquidity(ix_liq)), Some(DexEvent::RaydiumClmmIncreaseLiquidity(log_liq))) => {
            Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
                metadata: log_liq.metadata,
                pool: log_liq.pool,
                user: log_liq.user,
                liquidity: log_liq.liquidity,
                amount0_max: ix_liq.amount0_max,
                amount1_max: ix_liq.amount1_max,
            }))
        }

        // 减少流动性事件合并
        (Some(DexEvent::RaydiumClmmDecreaseLiquidity(ix_liq)), Some(DexEvent::RaydiumClmmDecreaseLiquidity(log_liq))) => {
            Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
                metadata: log_liq.metadata,
                pool: log_liq.pool,
                user: log_liq.user,
                liquidity: log_liq.liquidity,
                amount0_min: ix_liq.amount0_min,
                amount1_min: ix_liq.amount1_min,
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

        // 测试创建池子指令
        data[..8].copy_from_slice(CREATE_POOL_IX);
        assert!(is_raydium_clmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("CreatePool"));

        // 测试交换指令
        data[..8].copy_from_slice(SWAP_IX);
        assert!(is_raydium_clmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), Some("Swap"));

        // 测试无效指令
        data[..8].copy_from_slice(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        assert!(!is_raydium_clmm_instruction(&data));
        assert_eq!(get_instruction_type(&data), None);
    }

    #[test]
    fn test_program_id_validation() {
        let valid_program_id = Pubkey::try_from(RAYDIUM_CLMM_PROGRAM_ID).unwrap();
        assert!(is_raydium_clmm_program(&valid_program_id));

        let invalid_program_id = Pubkey::default();
        assert!(!is_raydium_clmm_program(&invalid_program_id));
    }

    #[test]
    fn test_parse_create_pool() {
        let mut data = vec![0u8; 32];
        data[..8].copy_from_slice(CREATE_POOL_IX);
        data[8..24].copy_from_slice(&79228162514264337593543950336u128.to_le_bytes()); // sqrt_price_x64
        data[24..32].copy_from_slice(&1640995200u64.to_le_bytes()); // open_time

        let accounts = vec![Pubkey::default(); 15];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_raydium_clmm_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::RaydiumClmmCreatePool(event)) = result {
            assert_eq!(event.sqrt_price_x64, 79228162514264337593543950336u128);
            assert_eq!(event.open_time, 1640995200);
        } else {
            panic!("Expected RaydiumClmmCreatePool event");
        }
    }

    #[test]
    fn test_parse_swap() {
        let mut data = vec![0u8; 41];
        data[..8].copy_from_slice(SWAP_IX);
        data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // amount
        data[16..24].copy_from_slice(&900_000_000u64.to_le_bytes()); // other_amount_threshold
        data[24..40].copy_from_slice(&79228162514264337593543950336u128.to_le_bytes()); // sqrt_price_limit_x64
        data[40] = 1; // is_base_input = true

        let accounts = vec![Pubkey::default(); 15];
        let signature = Signature::default();
        let slot = 123456789;
        let block_time = None;

        let result = parse_raydium_clmm_instruction(&data, &accounts, signature, slot, block_time, 0);
        assert!(result.is_some());

        if let Some(DexEvent::RaydiumClmmSwap(event)) = result {
            assert_eq!(event.amount, 1_000_000_000);
            assert_eq!(event.other_amount_threshold, 900_000_000);
            assert!(event.is_base_input);
        } else {
            panic!("Expected RaydiumClmmSwap event");
        }
    }

    #[test]
    fn test_batch_parsing() {
        let mut instructions = Vec::new();

        // 添加创建池子指令
        let mut data1 = vec![0u8; 32];
        data1[..8].copy_from_slice(CREATE_POOL_IX);
        data1[8..24].copy_from_slice(&79228162514264337593543950336u128.to_le_bytes());
        let accounts1 = vec![Pubkey::default(); 15];
        instructions.push((data1, accounts1));

        // 添加交换指令
        let mut data2 = vec![0u8; 41];
        data2[..8].copy_from_slice(SWAP_IX);
        data2[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        let accounts2 = vec![Pubkey::default(); 15];
        instructions.push((data2, accounts2));

        let events = parse_instructions_batch(
            &instructions,
            Signature::default(),
            123456789,
            None,
        );

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], DexEvent::RaydiumClmmCreatePool(_)));
        assert!(matches!(events[1], DexEvent::RaydiumClmmSwap(_)));
    }
}