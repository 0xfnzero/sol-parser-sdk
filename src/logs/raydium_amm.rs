//! Raydium AMM V4 日志解析器
//!
//! 使用 match discriminator 模式解析 Raydium AMM V4 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Raydium AMM V4 日志事件 discriminator 常量
pub mod discriminators {
    // 事件鉴别器 - 基于参考代码，Raydium AMM V4 使用的可能的日志事件标识
    pub const SWAP_BASE_IN_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 9];
    pub const SWAP_BASE_OUT_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 11];
    pub const DEPOSIT_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 3];
    pub const WITHDRAW_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];
    pub const INITIALIZE2_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    pub const WITHDRAW_PNL_EVENT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 7];
}

/// Raydium AMM V4 程序 ID
pub const PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// 检查是否是 Raydium AMM 日志
pub fn is_raydium_amm_log(log: &str) -> bool {
    log.contains(PROGRAM_ID) ||
    log.contains("RaydiumAmmV4") ||
    log.contains("raydium_amm_v4") ||
    (log.contains("AMM") && log.contains("675kPX")) ||
    (log.contains("Program data:") && (
        log.contains("swap") ||
        log.contains("deposit") ||
        log.contains("withdraw") ||
        log.contains("initialize")
    ))
}

/// 解析 Raydium AMM V4 日志
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
        discriminators::SWAP_BASE_IN_EVENT => {
            parse_swap_base_in_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::SWAP_BASE_OUT_EVENT => {
            parse_swap_base_out_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::DEPOSIT_EVENT => {
            parse_deposit_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::WITHDRAW_EVENT => {
            parse_withdraw_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::INITIALIZE2_EVENT => {
            parse_initialize2_event(data, signature, slot, block_time, grpc_recv_us)
        },
        discriminators::WITHDRAW_PNL_EVENT => {
            parse_withdraw_pnl_event(data, signature, slot, block_time, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析 SwapBaseIn 事件
fn parse_swap_base_in_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        amount_in,
        minimum_amount_out,
        max_amount_in: 0,
        amount_out: 0,
        token_program: Pubkey::default(),
        amm,
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: None,
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_bids: Pubkey::default(),
        serum_asks: Pubkey::default(),
        serum_event_queue: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
        user_source_token_account: Pubkey::default(),
        user_destination_token_account: Pubkey::default(),
        user_source_owner: user,
    }))
}

/// 解析 SwapBaseOut 事件
fn parse_swap_base_out_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let max_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        amount_in: 0,
        minimum_amount_out: 0,
        max_amount_in,
        amount_out,
        token_program: Pubkey::default(),
        amm,
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: None,
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_bids: Pubkey::default(),
        serum_asks: Pubkey::default(),
        serum_event_queue: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
        user_source_token_account: Pubkey::default(),
        user_destination_token_account: Pubkey::default(),
        user_source_owner: user,
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

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let max_coin_amount = read_u64_le(data, offset)?;
    offset += 8;

    let max_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let base_side = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent {
        metadata,
        max_coin_amount,
        max_pc_amount,
        base_side,
        token_program: Pubkey::default(),
        amm,
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: Pubkey::default(),
        lp_mint_address: Pubkey::default(),
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        serum_market: Pubkey::default(),
        user_coin_token_account: Pubkey::default(),
        user_pc_token_account: Pubkey::default(),
        user_lp_token_account: Pubkey::default(),
        user_owner: user,
        serum_event_queue: Pubkey::default(),
    }))
}

/// 解析提取事件
fn parse_withdraw_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent {
        metadata,
        amount,
        token_program: Pubkey::default(),
        amm,
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: Pubkey::default(),
        lp_mint_address: Pubkey::default(),
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        pool_withdraw_queue: Pubkey::default(),
        pool_temp_lp_token_account: Pubkey::default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
        user_lp_token_account: Pubkey::default(),
        user_coin_token_account: Pubkey::default(),
        user_pc_token_account: Pubkey::default(),
        user_owner: user,
        serum_event_queue: Pubkey::default(),
        serum_bids: Pubkey::default(),
        serum_asks: Pubkey::default(),
    }))
}

/// 解析初始化事件
fn parse_initialize2_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let nonce = data.get(offset)?.clone();
    offset += 1;

    let open_time = read_u64_le(data, offset)?;
    offset += 8;

    let init_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let init_coin_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Initialize2(RaydiumAmmV4Initialize2Event {
        metadata,
        nonce,
        open_time,
        init_pc_amount,
        init_coin_amount,
        token_program: Pubkey::default(),
        spl_associated_token_account: Pubkey::default(),
        system_program: Pubkey::default(),
        rent: Pubkey::default(),
        amm,
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        lp_mint: Pubkey::default(),
        coin_mint: Pubkey::default(),
        pc_mint: Pubkey::default(),
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        pool_withdraw_queue: Pubkey::default(),
        amm_target_orders: Pubkey::default(),
        pool_temp_lp: Pubkey::default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        user_wallet: user,
        user_token_coin: Pubkey::default(),
        user_token_pc: Pubkey::default(),
        user_lp_token_account: Pubkey::default(),
    }))
}

/// 解析提取 PnL 事件
fn parse_withdraw_pnl_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amm = read_pubkey(data, offset)?;
    offset += 32;

    let pnl_owner = read_pubkey(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, block_time, amm, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4WithdrawPnl(RaydiumAmmV4WithdrawPnlEvent {
        metadata,
        token_program: Pubkey::default(),
        amm,
        amm_config: Pubkey::default(),
        amm_authority: Pubkey::default(),
        amm_open_orders: Pubkey::default(),
        pool_coin_token_account: Pubkey::default(),
        pool_pc_token_account: Pubkey::default(),
        coin_pnl_token_account: Pubkey::default(),
        pc_pnl_token_account: Pubkey::default(),
        pnl_owner,
        amm_target_orders: Pubkey::default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_event_queue: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
    }))
}

/// 文本日志解析（回退方案）
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // 检查是否是交换相关的日志
    if log.contains("swap") || log.contains("Swap") {
        return parse_swap_log_fallback(log, signature, slot, block_time, grpc_recv_us);
    }

    // 检查是否是存款相关的日志
    if log.contains("deposit") || log.contains("Deposit") {
        return parse_deposit_log_fallback(log, signature, slot, block_time, grpc_recv_us);
    }

    // 检查是否是提取相关的日志
    if log.contains("withdraw") || log.contains("Withdraw") {
        return parse_withdraw_log_fallback(log, signature, slot, block_time, grpc_recv_us);
    }

    None
}

/// 文本回退解析交换事件
fn parse_swap_log_fallback(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // 尝试从日志文本中提取基本信息
    let amount_in = super::utils::text_parser::extract_number_from_text(log, "amount_in")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "amountIn"))
        .unwrap_or(0);

    let amount_out = super::utils::text_parser::extract_number_from_text(log, "amount_out")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "amountOut"))
        .unwrap_or(0);

    let minimum_amount_out = super::utils::text_parser::extract_number_from_text(log, "minimum_amount_out")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "minimumAmountOut"))
        .unwrap_or(0);

    let max_amount_in = super::utils::text_parser::extract_number_from_text(log, "max_amount_in")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "maxAmountIn"))
        .unwrap_or(0);

    let default_pubkey = Pubkey::default();
    let metadata = create_metadata_simple(signature, slot, block_time, default_pubkey, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        amount_in,
        minimum_amount_out,
        max_amount_in,
        amount_out,
        token_program: default_pubkey,
        amm: default_pubkey,
        amm_authority: default_pubkey,
        amm_open_orders: default_pubkey,
        amm_target_orders: None,
        pool_coin_token_account: default_pubkey,
        pool_pc_token_account: default_pubkey,
        serum_program: default_pubkey,
        serum_market: default_pubkey,
        serum_bids: default_pubkey,
        serum_asks: default_pubkey,
        serum_event_queue: default_pubkey,
        serum_coin_vault_account: default_pubkey,
        serum_pc_vault_account: default_pubkey,
        serum_vault_signer: default_pubkey,
        user_source_token_account: default_pubkey,
        user_destination_token_account: default_pubkey,
        user_source_owner: default_pubkey,
    }))
}

/// 文本回退解析存款事件
fn parse_deposit_log_fallback(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let max_coin_amount = super::utils::text_parser::extract_number_from_text(log, "max_coin_amount")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "maxCoinAmount"))
        .unwrap_or(0);

    let max_pc_amount = super::utils::text_parser::extract_number_from_text(log, "max_pc_amount")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "maxPcAmount"))
        .unwrap_or(0);

    let base_side = super::utils::text_parser::extract_number_from_text(log, "base_side")
        .or_else(|| super::utils::text_parser::extract_number_from_text(log, "baseSide"))
        .unwrap_or(0);

    let default_pubkey = Pubkey::default();
    let metadata = create_metadata_simple(signature, slot, block_time, default_pubkey, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent {
        metadata,
        max_coin_amount,
        max_pc_amount,
        base_side,
        token_program: default_pubkey,
        amm: default_pubkey,
        amm_authority: default_pubkey,
        amm_open_orders: default_pubkey,
        amm_target_orders: default_pubkey,
        lp_mint_address: default_pubkey,
        pool_coin_token_account: default_pubkey,
        pool_pc_token_account: default_pubkey,
        serum_market: default_pubkey,
        user_coin_token_account: default_pubkey,
        user_pc_token_account: default_pubkey,
        user_lp_token_account: default_pubkey,
        user_owner: default_pubkey,
        serum_event_queue: default_pubkey,
    }))
}

/// 文本回退解析提取事件
fn parse_withdraw_log_fallback(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let amount = super::utils::text_parser::extract_number_from_text(log, "amount")
        .unwrap_or(0);

    let default_pubkey = Pubkey::default();
    let metadata = create_metadata_simple(signature, slot, block_time, default_pubkey, grpc_recv_us);

    Some(DexEvent::RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent {
        metadata,
        amount,
        token_program: default_pubkey,
        amm: default_pubkey,
        amm_authority: default_pubkey,
        amm_open_orders: default_pubkey,
        amm_target_orders: default_pubkey,
        lp_mint_address: default_pubkey,
        pool_coin_token_account: default_pubkey,
        pool_pc_token_account: default_pubkey,
        pool_withdraw_queue: default_pubkey,
        pool_temp_lp_token_account: default_pubkey,
        serum_program: default_pubkey,
        serum_market: default_pubkey,
        serum_coin_vault_account: default_pubkey,
        serum_pc_vault_account: default_pubkey,
        serum_vault_signer: default_pubkey,
        user_lp_token_account: default_pubkey,
        user_coin_token_account: default_pubkey,
        user_pc_token_account: default_pubkey,
        user_owner: default_pubkey,
        serum_event_queue: default_pubkey,
        serum_bids: default_pubkey,
        serum_asks: default_pubkey,
    }))
}