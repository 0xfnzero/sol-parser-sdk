//! Raydium AMM V4 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium AMM V4 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium AMM V4 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaydiumAmmV4Instruction {
    Initialize2 = 1,
    Deposit = 3,
    Withdraw = 4,
    WithdrawPnl = 7,
    SwapBaseIn = 9,
    SwapBaseOut = 11,
    /// Post 2026-07-22 OpenBook removal — 8 accounts, no serum market.
    SwapBaseInV2 = 16,
    SwapBaseOutV2 = 17,
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
            16 => Some(Self::SwapBaseInV2),
            17 => Some(Self::SwapBaseOutV2),
            _ => None,
        }
    }
}

/// Raydium AMM V4 discriminator 常量
pub mod discriminators {
    pub const SWAP_BASE_IN: u8 = 9;
    pub const SWAP_BASE_OUT: u8 = 11;
    pub const SWAP_BASE_IN_V2: u8 = 16;
    pub const SWAP_BASE_OUT_V2: u8 = 17;
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
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.is_empty() {
        return None;
    }

    let discriminator_byte = instruction_data[0];
    let instruction_type = RaydiumAmmV4Instruction::from_u8(discriminator_byte)?;
    let data = &instruction_data[1..];

    match instruction_type {
        RaydiumAmmV4Instruction::SwapBaseIn => {
            parse_swap_base_in_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        RaydiumAmmV4Instruction::SwapBaseOut => parse_swap_base_out_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        RaydiumAmmV4Instruction::SwapBaseInV2 => parse_swap_base_in_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        RaydiumAmmV4Instruction::SwapBaseOutV2 => parse_swap_base_out_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        RaydiumAmmV4Instruction::Deposit => {
            parse_deposit_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        RaydiumAmmV4Instruction::Withdraw => {
            parse_withdraw_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        RaydiumAmmV4Instruction::Initialize2 => {
            parse_initialize2_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        RaydiumAmmV4Instruction::WithdrawPnl => {
            parse_withdraw_pnl_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
    }
}

/// SwapBaseInV2 (tag 16): tokenProgram, amm, authority, coinVault, pcVault, userSrc, userDst, owner
fn parse_swap_base_in_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let amount_in = read_u64_le(data, 0)?;
    let minimum_amount_out = read_u64_le(data, 8)?;
    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);
    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        ix_name: "swap_base_in_v2".into(),
        instruction_amount_in: amount_in,
        amount_in: 0,
        minimum_amount_out,
        max_amount_in: 0,
        instruction_amount_out: 0,
        amount_out: 0,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: None,
        pool_coin_token_account: get_account(accounts, 3).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 4).unwrap_or_default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_bids: Pubkey::default(),
        serum_asks: Pubkey::default(),
        serum_event_queue: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
        user_source_token_account: get_account(accounts, 5).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 6).unwrap_or_default(),
        user_source_owner: get_account(accounts, 7).unwrap_or_default(),
    }))
}

/// SwapBaseOutV2 (tag 17): same 8-account layout as V2 in.
fn parse_swap_base_out_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let max_amount_in = read_u64_le(data, 0)?;
    let amount_out = read_u64_le(data, 8)?;
    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);
    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        ix_name: "swap_base_out_v2".into(),
        instruction_amount_in: 0,
        amount_in: 0,
        minimum_amount_out: 0,
        max_amount_in,
        instruction_amount_out: amount_out,
        amount_out: 0,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: Pubkey::default(),
        amm_target_orders: None,
        pool_coin_token_account: get_account(accounts, 3).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 4).unwrap_or_default(),
        serum_program: Pubkey::default(),
        serum_market: Pubkey::default(),
        serum_bids: Pubkey::default(),
        serum_asks: Pubkey::default(),
        serum_event_queue: Pubkey::default(),
        serum_coin_vault_account: Pubkey::default(),
        serum_pc_vault_account: Pubkey::default(),
        serum_vault_signer: Pubkey::default(),
        user_source_token_account: get_account(accounts, 5).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 6).unwrap_or_default(),
        user_source_owner: get_account(accounts, 7).unwrap_or_default(),
    }))
}

/// 解析 SwapBaseIn 指令
fn parse_swap_base_in_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

    let shift = usize::from(accounts.len() == 17);
    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        ix_name: "swap_base_in".into(),
        instruction_amount_in: amount_in,
        amount_in: 0,
        minimum_amount_out,
        max_amount_in: 0,
        instruction_amount_out: 0,
        amount_out: 0,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: if shift == 0 { get_account(accounts, 4) } else { None },
        pool_coin_token_account: get_account(accounts, 5 - shift).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 6 - shift).unwrap_or_default(),
        serum_program: get_account(accounts, 7 - shift).unwrap_or_default(),
        serum_market: get_account(accounts, 8 - shift).unwrap_or_default(),
        serum_bids: get_account(accounts, 9 - shift).unwrap_or_default(),
        serum_asks: get_account(accounts, 10 - shift).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 11 - shift).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 12 - shift).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 13 - shift).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 14 - shift).unwrap_or_default(),
        user_source_token_account: get_account(accounts, 15 - shift).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 16 - shift).unwrap_or_default(),
        user_source_owner: get_account(accounts, 17 - shift).unwrap_or_default(),
    }))
}

/// 解析 SwapBaseOut 指令
fn parse_swap_base_out_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let max_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

    let shift = usize::from(accounts.len() == 17);
    Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
        metadata,
        ix_name: "swap_base_out".into(),
        instruction_amount_in: 0,
        amount_in: 0,
        minimum_amount_out: 0,
        max_amount_in,
        instruction_amount_out: amount_out,
        amount_out: 0,
        token_program: get_account(accounts, 0).unwrap_or_default(),
        amm,
        amm_authority: get_account(accounts, 2).unwrap_or_default(),
        amm_open_orders: get_account(accounts, 3).unwrap_or_default(),
        amm_target_orders: if shift == 0 { get_account(accounts, 4) } else { None },
        pool_coin_token_account: get_account(accounts, 5 - shift).unwrap_or_default(),
        pool_pc_token_account: get_account(accounts, 6 - shift).unwrap_or_default(),
        serum_program: get_account(accounts, 7 - shift).unwrap_or_default(),
        serum_market: get_account(accounts, 8 - shift).unwrap_or_default(),
        serum_bids: get_account(accounts, 9 - shift).unwrap_or_default(),
        serum_asks: get_account(accounts, 10 - shift).unwrap_or_default(),
        serum_event_queue: get_account(accounts, 11 - shift).unwrap_or_default(),
        serum_coin_vault_account: get_account(accounts, 12 - shift).unwrap_or_default(),
        serum_pc_vault_account: get_account(accounts, 13 - shift).unwrap_or_default(),
        serum_vault_signer: get_account(accounts, 14 - shift).unwrap_or_default(),
        user_source_token_account: get_account(accounts, 15 - shift).unwrap_or_default(),
        user_destination_token_account: get_account(accounts, 16 - shift).unwrap_or_default(),
        user_source_owner: get_account(accounts, 17 - shift).unwrap_or_default(),
    }))
}

/// 解析存款指令
fn parse_deposit_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let max_coin_amount = read_u64_le(data, offset)?;
    offset += 8;

    let max_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let base_side = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

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
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let amount = read_u64_le(data, 0)?;

    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

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
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let nonce = *data.get(offset)?;
    offset += 1;

    let open_time = read_u64_le(data, offset)?;
    offset += 8;

    let init_pc_amount = read_u64_le(data, offset)?;
    offset += 8;

    let init_coin_amount = read_u64_le(data, offset)?;

    let amm = get_account(accounts, 4)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

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
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let amm = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, amm);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn swap(accounts: &[Pubkey]) -> RaydiumAmmV4SwapEvent {
        let mut data = vec![discriminators::SWAP_BASE_IN];
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&90u64.to_le_bytes());
        let event =
            parse_instruction(&data, accounts, Signature::default(), 1, 0, None).expect("swap");
        let DexEvent::RaydiumAmmV4Swap(event) = event else { panic!("unexpected event") };
        event
    }

    #[test]
    fn swap_supports_legacy_and_target_orders_free_layouts() {
        let legacy: Vec<_> = (0..18).map(|_| Pubkey::new_unique()).collect();
        let current: Vec<_> = (0..17).map(|_| Pubkey::new_unique()).collect();

        let legacy_event = swap(&legacy);
        assert_eq!(legacy_event.amm_target_orders, Some(legacy[4]));
        assert_eq!(legacy_event.pool_coin_token_account, legacy[5]);
        assert_eq!(legacy_event.user_source_owner, legacy[17]);

        let current_event = swap(&current);
        assert_eq!(current_event.amm_target_orders, None);
        assert_eq!(current_event.pool_coin_token_account, current[4]);
        assert_eq!(current_event.user_source_owner, current[16]);
    }

    #[test]
    fn swap_v2_maps_eight_accounts_without_serum() {
        let accounts: Vec<_> = (0..8).map(|i| Pubkey::new_from_array([i + 1; 32])).collect();
        let mut data = vec![discriminators::SWAP_BASE_IN_V2];
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&90u64.to_le_bytes());
        let event =
            parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).expect("v2 swap");
        let DexEvent::RaydiumAmmV4Swap(event) = event else { panic!("unexpected event") };
        assert_eq!(event.instruction_amount_in, 100);
        assert_eq!(event.amount_in, 0);
        assert_eq!(event.minimum_amount_out, 90);
        assert_eq!(event.token_program, accounts[0]);
        assert_eq!(event.amm, accounts[1]);
        assert_eq!(event.amm_authority, accounts[2]);
        assert_eq!(event.pool_coin_token_account, accounts[3]);
        assert_eq!(event.pool_pc_token_account, accounts[4]);
        assert_eq!(event.user_source_token_account, accounts[5]);
        assert_eq!(event.user_destination_token_account, accounts[6]);
        assert_eq!(event.user_source_owner, accounts[7]);
        assert_eq!(event.amm_open_orders, Pubkey::default());
        assert!(event.amm_target_orders.is_none());
        assert_eq!(event.serum_market, Pubkey::default());
    }
}

#[cfg(test)]
mod swap_parameter_semantics_tests {
    use super::*;
    #[test]
    fn swap_wire_quantities_and_limits_are_distinct_from_execution() {
        for (tag, name, count, exact_input) in [
            (9, "swap_base_in", 18),
            (11, "swap_base_out", 18),
            (16, "swap_base_in_v2", 8),
            (17, "swap_base_out_v2", 8),
        ]
        .map(|(t, n, c)| (t, n, c, t == 9 || t == 16))
        {
            let accounts: Vec<_> = (0..count).map(|_| Pubkey::new_unique()).collect();
            for (specified, limit) in [(0u64, u64::MAX), (u64::MAX, 0)] {
                let mut data = vec![tag];
                let values = if exact_input { [specified, limit] } else { [limit, specified] };
                for v in values {
                    data.extend_from_slice(&v.to_le_bytes());
                }
                let DexEvent::RaydiumAmmV4Swap(e) =
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap()
                else {
                    panic!("swap")
                };
                assert_eq!(e.ix_name, name);
                assert_eq!((e.amount_in, e.amount_out), (0, 0));
                assert_eq!(
                    (e.instruction_amount_in, e.instruction_amount_out),
                    if exact_input { (specified, 0) } else { (0, specified) }
                );
                assert_eq!(
                    (e.minimum_amount_out, e.max_amount_in),
                    if exact_input { (limit, 0) } else { (0, limit) }
                );
                assert_eq!(e.user_source_owner, accounts[count - 1]);
            }
        }
    }
}
