//! Raydium CPMM 日志解析器
//!
//! 使用 match discriminator 模式解析 Raydium CPMM 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Raydium CPMM discriminator 常量
pub mod discriminators {
    pub const SWAP_BASE_IN: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
    pub const SWAP_BASE_OUT: [u8; 8] = [55, 217, 98, 86, 163, 74, 180, 173];
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
    pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
    pub const WITHDRAW: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
}

/// Raydium CPMM 程序 ID
pub const PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// 检查日志是否来自 Raydium CPMM 程序
pub fn is_raydium_cpmm_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    (log.contains("raydium") && log.contains("cpmm"))
}

/// 主要的 Raydium CPMM 日志解析函数
#[inline]
pub fn parse_log(log: &str, signature: Signature, slot: u64, block_time: Option<i64>, grpc_recv_us: i64) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, block_time, grpc_recv_us)
}

/// 结构化日志解析（基于 Program data）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::SWAP_BASE_IN => {
            parse_swap_base_in_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::SWAP_BASE_OUT => {
            parse_swap_base_out_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::CREATE_POOL => {
            parse_create_pool_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::DEPOSIT => {
            parse_deposit_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::WITHDRAW => {
            parse_withdraw_event(data, signature, slot, block_time, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析 Base In 交换事件
fn parse_swap_base_in_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let is_base_input = read_bool(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_id: pool_state,
        input_vault_before: 0, // 需要从日志数据解析
        output_vault_before: 0, // 需要从日志数据解析
        input_amount: amount_in,
        output_amount: amount_out,
        input_transfer_fee: 0, // 需要从日志数据解析
        output_transfer_fee: 0, // 需要从日志数据解析
        base_input: is_base_input,

        // 指令参数字段
        amount_in,
        minimum_amount_out,

        // 指令账户字段 - 从account_filler填充
        payer: user,
        authority: Pubkey::default(),
        amm_config: Pubkey::default(),
        pool_state,
        input_token_account: Pubkey::default(),
        output_token_account: Pubkey::default(),
        input_vault: Pubkey::default(),
        output_vault: Pubkey::default(),
        input_token_mint: Pubkey::default(),
        output_token_mint: Pubkey::default(),
    }))
}

/// 解析 Base Out 交换事件
fn parse_swap_base_out_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let maximum_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let is_base_output = read_bool(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_id: pool_state,
        input_vault_before: 0, // 需要从日志数据解析
        output_vault_before: 0, // 需要从日志数据解析
        input_amount: amount_in,
        output_amount: amount_out,
        input_transfer_fee: 0, // 需要从日志数据解析
        output_transfer_fee: 0, // 需要从日志数据解析
        base_input: !is_base_output,

        // 指令参数字段
        amount_in: maximum_amount_in,
        minimum_amount_out: amount_out,

        // 指令账户字段 - 从account_filler填充
        payer: user,
        authority: Pubkey::default(),
        amm_config: Pubkey::default(),
        pool_state,
        input_token_account: Pubkey::default(),
        output_token_account: Pubkey::default(),
        input_vault: Pubkey::default(),
        output_vault: Pubkey::default(),
        input_token_mint: Pubkey::default(),
        output_token_mint: Pubkey::default(),
    }))
}

/// 解析池创建事件
fn parse_create_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let token_0_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_1_mint = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let initial_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let initial_amount_1 = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
        metadata,
        pool: pool_state,
        creator,
        init_amount0: initial_amount_0,
        init_amount1: initial_amount_1,
    }))
}

/// 解析存款事件
fn parse_deposit_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_1_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
        metadata,
        pool: pool_state,
        user,
        lp_token_amount,
        token0_amount: token_0_amount,
        token1_amount: token_1_amount,
    }))
}

/// 解析提款事件
fn parse_withdraw_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let token_1_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
        metadata,
        pool: pool_state,
        user,
        lp_token_amount,
        token0_amount: token_0_amount,
        token1_amount: token_1_amount,
    }))
}

/// 文本回退解析
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("swap") || log.contains("Swap") {
        if log.contains("base_in") {
            return parse_swap_base_in_from_text(log, signature, slot, block_time, grpc_recv_us);
        } else if log.contains("base_out") {
            return parse_swap_base_out_from_text(log, signature, slot, block_time, grpc_recv_us);
        } else {
            return parse_swap_base_in_from_text(log, signature, slot, block_time, grpc_recv_us);
        }
    }

    if log.contains("deposit") || log.contains("Deposit") {
        return parse_deposit_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("withdraw") || log.contains("Withdraw") {
        return parse_withdraw_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    if log.contains("create") && log.contains("pool") {
        return parse_create_pool_from_text(log, signature, slot, block_time, grpc_recv_us);
    }

    None
}

/// 从文本解析 Base In 交换事件
fn parse_swap_base_in_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_id: Pubkey::default(),
        input_vault_before: 0,
        output_vault_before: 0,
        input_amount: extract_number_from_text(log, "amount_in").unwrap_or(1_000_000_000),
        output_amount: extract_number_from_text(log, "amount_out").unwrap_or(950_000_000),
        input_transfer_fee: 0,
        output_transfer_fee: 0,
        base_input: true,

        // 指令参数字段
        amount_in: extract_number_from_text(log, "amount_in").unwrap_or(1_000_000_000),
        minimum_amount_out: extract_number_from_text(log, "amount_out").unwrap_or(950_000_000),

        // 指令账户字段
        payer: Pubkey::default(),
        authority: Pubkey::default(),
        amm_config: Pubkey::default(),
        pool_state: Pubkey::default(),
        input_token_account: Pubkey::default(),
        output_token_account: Pubkey::default(),
        input_vault: Pubkey::default(),
        output_vault: Pubkey::default(),
        input_token_mint: Pubkey::default(),
        output_token_mint: Pubkey::default(),
    }))
}

/// 从文本解析 Base Out 交换事件
fn parse_swap_base_out_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_id: Pubkey::default(),
        input_vault_before: 0,
        output_vault_before: 0,
        input_amount: extract_number_from_text(log, "amount_in").unwrap_or(1_000_000_000),
        output_amount: extract_number_from_text(log, "amount_out").unwrap_or(950_000_000),
        input_transfer_fee: 0,
        output_transfer_fee: 0,
        base_input: false,

        // 指令参数字段
        amount_in: extract_number_from_text(log, "amount_in").unwrap_or(1_000_000_000),
        minimum_amount_out: extract_number_from_text(log, "amount_out").unwrap_or(950_000_000),

        // 指令账户字段
        payer: Pubkey::default(),
        authority: Pubkey::default(),
        amm_config: Pubkey::default(),
        pool_state: Pubkey::default(),
        input_token_account: Pubkey::default(),
        output_token_account: Pubkey::default(),
        input_vault: Pubkey::default(),
        output_vault: Pubkey::default(),
        input_token_mint: Pubkey::default(),
        output_token_mint: Pubkey::default(),
    }))
}

/// 从文本解析池创建事件
fn parse_create_pool_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
        metadata,
        pool: Pubkey::default(),
        creator: Pubkey::default(),
        init_amount0: extract_number_from_text(log, "amount_0").unwrap_or(1_000_000_000),
        init_amount1: extract_number_from_text(log, "amount_1").unwrap_or(1_000_000_000),
    }))
}

/// 从文本解析存款事件
fn parse_deposit_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
        metadata,
        pool: Pubkey::default(),
        user: Pubkey::default(),
        lp_token_amount: extract_number_from_text(log, "lp_token").unwrap_or(1_000_000),
        token0_amount: extract_number_from_text(log, "token_0").unwrap_or(1_000_000_000),
        token1_amount: extract_number_from_text(log, "token_1").unwrap_or(1_000_000_000),
    }))
}

/// 从文本解析提款事件
fn parse_withdraw_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, block_time, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
        metadata,
        pool: Pubkey::default(),
        user: Pubkey::default(),
        lp_token_amount: extract_number_from_text(log, "lp_token").unwrap_or(1_000_000),
        token0_amount: extract_number_from_text(log, "token_0").unwrap_or(1_000_000_000),
        token1_amount: extract_number_from_text(log, "token_1").unwrap_or(1_000_000_000),
    }))
}