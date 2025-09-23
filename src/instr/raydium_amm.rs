//! Raydium AMM V4 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium AMM V4 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// Raydium AMM V4 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaydiumAmmV4Instruction {
    Initialize2 = 1,
    Deposit = 3,
    Withdraw = 4,
    WithdrawPnl = 7,
    SwapBaseIn = 9,
    SwapBaseOut = 11,
}

impl RaydiumAmmV4Instruction {
    /// 从字节转换为指令类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Initialize2),
            3 => Some(Self::Deposit),
            4 => Some(Self::Withdraw),
            7 => Some(Self::WithdrawPnl),
            9 => Some(Self::SwapBaseIn),
            11 => Some(Self::SwapBaseOut),
            _ => None,
        }
    }
}

/// Raydium AMM V4 discriminator 常量
pub mod discriminators {
    pub const SWAP_BASE_IN: u8 = 9;
    pub const SWAP_BASE_OUT: u8 = 11;
    pub const DEPOSIT: u8 = 3;
    pub const WITHDRAW: u8 = 4;
    pub const INITIALIZE2: u8 = 1;
    pub const WITHDRAW_PNL: u8 = 7;
}

/// Raydium AMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_AMM_V4_PROGRAM_ID;

/// 主要的 Raydium AMM V4 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.is_empty() {
        return None;
    }

    let discriminator_byte = instruction_data[0];
    let instruction_type = RaydiumAmmV4Instruction::from_u8(discriminator_byte)?;
    let data = &instruction_data[1..];

    match instruction_type {
        RaydiumAmmV4Instruction::SwapBaseIn => {
            parse_swap_base_in_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        RaydiumAmmV4Instruction::SwapBaseOut => {
            parse_swap_base_out_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        RaydiumAmmV4Instruction::Deposit => {
            parse_deposit_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        RaydiumAmmV4Instruction::Withdraw => {
            parse_withdraw_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        RaydiumAmmV4Instruction::Initialize2 => {
            parse_initialize2_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
        RaydiumAmmV4Instruction::WithdrawPnl => {
            parse_withdraw_pnl_instruction(data, accounts, signature, slot, tx_index, block_time)
        },
    }
}

/// 解析 SwapBaseIn 指令
fn parse_swap_base_in_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        amount_in,
        minimum_amount_out,
        max_amount_in: 0,
        amount_out: 0,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 4),
        pool_coin_token_account: get_account(accounts, 5).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 6).unwrap_or_default(),
        serum_program: get_account(accounts, 7).unwrap_or_default(),
        serum_market: get_account(accounts, 8).unwrap_or_default(),
        serum_bids: get_account(accounts, 9).unwrap_or_default(),
        serum_asks: get_account(accounts, 10).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 11).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 12).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 13).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 14).unwrap_or_default(),
        user_source_token_account: get_account(accounts, 15).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 16).unwrap_or_default(),
        user_source_owner: get_account(accounts, 17).unwrap_or_default(),
    }))
}

/// 解析 SwapBaseOut 指令
fn parse_swap_base_out_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let max_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        amount_in: 0,
        minimum_amount_out: 0,
        max_amount_in,
        amount_out,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 4),
        pool_coin_token_account: get_account(accounts, 5).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 6).unwrap_or_default(),
        serum_program: get_account(accounts, 7).unwrap_or_default(),
        serum_market: get_account(accounts, 8).unwrap_or_default(),
        serum_bids: get_account(accounts, 9).unwrap_or_default(),
        serum_asks: get_account(accounts, 10).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 11).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 12).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 13).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 14).unwrap_or_default(),
        user_source_token_account: get_account(accounts, 15).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 16).unwrap_or_default(),
        user_source_owner: get_account(accounts, 17).unwrap_or_default(),
    }))
}

/// 解析存款指令
fn parse_deposit_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let max_coin_amount = read_u64_le(data, offset)?;
    offset += 8;

    let max_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let base_side = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent {
        metadata,
        max_coin_amount,
        max_pc_amount,
        base_side,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 4).unwrap_or_default(),
        lp_mint_address: get_account(accounts, 5).unwrap_or_default(),
        pool_coin_token_account: get_account(accounts, 6).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 7).unwrap_or_default(),
        serum_market: get_account(accounts, 8).unwrap_or_default(),
        user_coin_token_account: get_account(accounts, 9).unwrap_or_default(),
        user_pc_token_account: get_account(accounts, 10).unwrap_or_default(),
        user_lp_token_account: get_account(accounts, 11).unwrap_or_default(),
        user_owner: get_account(accounts, 12).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 13).unwrap_or_default(),
    }))
}

/// 解析提取指令
fn parse_withdraw_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let amount = read_u64_le(data, 0)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent {
        metadata,
        amount,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 4).unwrap_or_default(),
        lp_mint_address: get_account(accounts, 5).unwrap_or_default(),
        pool_coin_token_account: get_account(accounts, 6).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 7).unwrap_or_default(),
        pool_withdraw_queue: get_account(accounts, 8).unwrap_or_default(),
        pool_temp_lp_token_account: get_account(accounts, 9).unwrap_or_default(),
        serum_program: get_account(accounts, 10).unwrap_or_default(),
        serum_market: get_account(accounts, 11).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 12).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 13).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 14).unwrap_or_default(),
        user_lp_token_account: get_account(accounts, 15).unwrap_or_default(),
        user_coin_token_account: get_account(accounts, 16).unwrap_or_default(),
        user_pc_token_account: get_account(accounts, 17).unwrap_or_default(),
        user_owner: get_account(accounts, 18).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 19).unwrap_or_default(),
        serum_bids: get_account(accounts, 20).unwrap_or_default(),
        serum_asks: get_account(accounts, 21).unwrap_or_default(),
    }))
}

/// 解析初始化指令
fn parse_initialize2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let nonce = data.get(offset)?.clone();
    offset += 1;

    let open_time = read_u64_le(data, offset)?;
    offset += 8;

    let init_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let init_coin_amount = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 4)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4Initialize2(RaydiumAmmV4Initialize2Event {
        metadata,
        nonce,
        open_time,
        init_pc_amount,
        init_coin_amount,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        spl_associated_token_account: get_account(accounts, 1).unwrap_or_default(),
        system_program: get_account(accounts, 2).unwrap_or_default(),
        rent: get_account(accounts, 3).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 5).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 6).unwrap_or_default(),
        lp_mint: get_account(accounts, 7).unwrap_or_default(),
        coin_mint: get_account(accounts, 8).unwrap_or_default(),
        pc_mint: get_account(accounts, 9).unwrap_or_default(),
        pool_coin_token_account: get_account(accounts, 10).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 11).unwrap_or_default(),
        pool_withdraw_queue: get_account(accounts, 12).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 13).unwrap_or_default(),
        pool_temp_lp: get_account(accounts, 14).unwrap_or_default(),
        serum_program: get_account(accounts, 15).unwrap_or_default(),
        serum_market: get_account(accounts, 16).unwrap_or_default(),
        user_wallet: get_account(accounts, 17).unwrap_or_default(),
        user_token_coin: get_account(accounts, 18).unwrap_or_default(),
        user_token_pc: get_account(accounts, 19).unwrap_or_default(),
        user_lp_token_account: get_account(accounts, 20).unwrap_or_default(),
    }))
}

/// 解析提取PnL指令
fn parse_withdraw_pnl_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: Option<u64>,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time, amm);

    Some(DexEvent::RaydiumAmmV4WithdrawPnl(RaydiumAmmV4WithdrawPnlEvent {
        metadata,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_config: get_account(accounts, 2).unwrap_or_default(),
        amm_authority: get_account(accounts, 3).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 4).unwrap_or_default(),
        pool_coin_token_account: get_account(accounts, 5).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 6).unwrap_or_default(),
        coin_pnl_token_account: get_account(accounts, 7).unwrap_or_default(),
        pc_pnl_token_account: get_account(accounts, 8).unwrap_or_default(),
        pnl_owner: get_account(accounts, 9).unwrap_or_default(),
        amm_target_orders: get_account(accounts, 10).unwrap_or_default(),
        serum_program: get_account(accounts, 11).unwrap_or_default(),
        serum_market: get_account(accounts, 12).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 13).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 14).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 15).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 16).unwrap_or_default(),
    }))
}