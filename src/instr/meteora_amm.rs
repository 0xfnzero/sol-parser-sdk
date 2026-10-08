//! Meteora Pools 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora Pools 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Meteora Pools 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeteoraPoolsInstruction {
    Initialize = 0,
    Swap = 1,
    AddLiquidity = 2,
    AddImbalanceLiquidity = 20,
    RemoveLiquidity = 3,
    RemoveLiquiditySingleSide = 21,
    CreateConfig = 4,
    CloseConfig = 5,
    UpdateCurveInfo = 6,
    TransferAdmin = 7,
    SetPoolFees = 8,
    OverrideCurveParam = 9,
    SetNewFeeOwner = 10,
    PartnerClaimFees = 11,
    WithdrawProtocolFees = 12,
    CreateLockEscrow = 13,
    Lock = 14,
    ClaimFee = 15,
    CreatePool = 16,
    CreatePoolWithConfig2 = 22,
    InitializePermissionedPool = 23,
    InitializePermissionlessPool = 24,
    InitializePermissionlessPoolWithFeeTier = 25,
    InitializeCustomizablePermissionlessConstantProductPool = 26,
    EnableOrDisablePool = 17,
    BootstrapLiquidity = 18,
    MigrateFeeAccount = 19,
}

impl MeteoraPoolsInstruction {
    /// 从 discriminator 转换为指令类型
    pub fn from_discriminator(discriminator: &[u8; 8]) -> Option<Self> {
        match *discriminator {
            [175, 175, 109, 31, 13, 152, 155, 237] => Some(Self::Initialize),
            [248, 198, 158, 145, 225, 117, 135, 200] => Some(Self::Swap),
            [168, 227, 50, 62, 189, 171, 84, 176] => Some(Self::AddLiquidity),
            [79, 35, 122, 84, 173, 15, 93, 191] => Some(Self::AddImbalanceLiquidity),
            [133, 109, 44, 179, 56, 238, 114, 33] => Some(Self::RemoveLiquidity),
            [84, 84, 177, 66, 254, 185, 10, 251] => Some(Self::RemoveLiquiditySingleSide),
            [4, 228, 215, 71, 225, 253, 119, 206] => Some(Self::BootstrapLiquidity),
            [208, 127, 21, 1, 194, 190, 196, 70] => Some(Self::CreateConfig),
            [123, 134, 81, 0, 49, 68, 98, 98] => Some(Self::CloseConfig),
            [7, 166, 138, 171, 206, 171, 236, 244] => Some(Self::CreatePool),
            [48, 149, 220, 130, 61, 11, 9, 178] => Some(Self::CreatePoolWithConfig2),
            [102, 44, 158, 54, 205, 37, 126, 78] => Some(Self::SetPoolFees),
            [77, 85, 178, 157, 50, 48, 212, 126] => Some(Self::InitializePermissionedPool),
            [118, 173, 41, 157, 173, 72, 97, 103] => Some(Self::InitializePermissionlessPool),
            [6, 135, 68, 147, 229, 82, 169, 113] => {
                Some(Self::InitializePermissionlessPoolWithFeeTier)
            }
            [145, 24, 172, 194, 219, 125, 3, 190] => {
                Some(Self::InitializeCustomizablePermissionlessConstantProductPool)
            }
            _ => None,
        }
    }
}

/// Meteora Pools discriminator 常量
pub mod discriminators {
    pub const INITIALIZE_PERMISSIONED_POOL: [u8; 8] = [77, 85, 178, 157, 50, 48, 212, 126];
    pub const INITIALIZE_PERMISSIONLESS_POOL: [u8; 8] = [118, 173, 41, 157, 173, 72, 97, 103];
    pub const INITIALIZE_PERMISSIONLESS_POOL_WITH_FEE_TIER: [u8; 8] =
        [6, 135, 68, 147, 229, 82, 169, 113];
    pub const INITIALIZE_CUSTOMIZABLE_POOL: [u8; 8] = [145, 24, 172, 194, 219, 125, 3, 190];
    pub const REMOVE_LIQUIDITY_SINGLE_SIDE: [u8; 8] = [84, 84, 177, 66, 254, 185, 10, 251];
    pub const BOOTSTRAP_LIQUIDITY: [u8; 8] = [4, 228, 215, 71, 225, 253, 119, 206];
    pub const ADD_IMBALANCE_LIQUIDITY: [u8; 8] = [79, 35, 122, 84, 173, 15, 93, 191];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const ADD_LIQUIDITY: [u8; 8] = [168, 227, 50, 62, 189, 171, 84, 176];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [133, 109, 44, 179, 56, 238, 114, 33];
    pub const CREATE_CONFIG: [u8; 8] = [208, 127, 21, 1, 194, 190, 196, 70];
    pub const CLOSE_CONFIG: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    /// Constant-product pool initialization with config (current IDL).
    pub const CREATE_POOL: [u8; 8] = [7, 166, 138, 171, 206, 171, 236, 244];
    pub const CREATE_POOL_WITH_CONFIG2: [u8; 8] = [48, 149, 220, 130, 61, 11, 9, 178];
    pub const SET_POOL_FEES: [u8; 8] = [102, 44, 158, 54, 205, 37, 126, 78];
}

/// Meteora AMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::METEORA_POOLS_PROGRAM_ID;

/// 主要的 Meteora Pools 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let instruction_type = MeteoraPoolsInstruction::from_discriminator(&discriminator)?;
    let data = &instruction_data[8..];

    match instruction_type {
        MeteoraPoolsInstruction::Swap => {
            parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        MeteoraPoolsInstruction::AddLiquidity | MeteoraPoolsInstruction::AddImbalanceLiquidity => {
            parse_add_liquidity_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                instruction_type == MeteoraPoolsInstruction::AddImbalanceLiquidity,
            )
        }
        MeteoraPoolsInstruction::RemoveLiquidity => parse_remove_liquidity_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        MeteoraPoolsInstruction::RemoveLiquiditySingleSide => {
            parse_single_side_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        MeteoraPoolsInstruction::BootstrapLiquidity => {
            parse_bootstrap_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        MeteoraPoolsInstruction::CreatePool | MeteoraPoolsInstruction::CreatePoolWithConfig2 => {
            parse_create_pool_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                instruction_type == MeteoraPoolsInstruction::CreatePoolWithConfig2,
            )
        }
        MeteoraPoolsInstruction::SetPoolFees => parse_set_pool_fees_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        MeteoraPoolsInstruction::InitializePermissionedPool
        | MeteoraPoolsInstruction::InitializePermissionlessPool
        | MeteoraPoolsInstruction::InitializePermissionlessPoolWithFeeTier => {
            parse_curve_pool_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                instruction_type,
            )
        }
        MeteoraPoolsInstruction::InitializeCustomizablePermissionlessConstantProductPool => {
            parse_customizable_pool_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        _ => None, // 其他指令暂不解析
    }
}

/// 解析 Swap 指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let in_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_out_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    let mut event = MeteoraPoolsSwapEvent {
        metadata,
        ix_name: "swap".to_string(),
        amount_in: in_amount,
        minimum_out_amount,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_swap_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsSwap(event))
}

/// 解析 Add Liquidity 指令
fn parse_add_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    imbalance: bool,
) -> Option<DexEvent> {
    let lp_amount = read_u64_le(data, 0)?;
    let a_amount = read_u64_le(data, 8)?;
    let b_amount = read_u64_le(data, 16)?;
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);
    let mut event = MeteoraPoolsAddLiquidityEvent {
        metadata,
        ix_name: if imbalance { "add_imbalance_liquidity" } else { "add_balance_liquidity" }.into(),
        pool_token_amount: if imbalance { 0 } else { lp_amount },
        maximum_token_a_amount: if imbalance { 0 } else { a_amount },
        maximum_token_b_amount: if imbalance { 0 } else { b_amount },
        minimum_pool_token_amount: if imbalance { lp_amount } else { 0 },
        token_a_in_amount: if imbalance { a_amount } else { 0 },
        token_b_in_amount: if imbalance { b_amount } else { 0 },
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_add_liquidity_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsAddLiquidity(event))
}

/// 解析 Remove Liquidity 指令
fn parse_remove_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_b_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    let mut event = MeteoraPoolsRemoveLiquidityEvent {
        metadata,
        ix_name: "remove_balance_liquidity".into(),
        pool_token_amount,
        minimum_a_token_out: minimum_token_a_amount,
        minimum_b_token_out: minimum_token_b_amount,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_remove_liquidity_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsRemoveLiquidity(event))
}

fn parse_single_side_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool_token_amount = read_u64_le(data, 0)?;
    let minimum_out_amount = read_u64_le(data, 8)?;
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsRemoveLiquidityEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: "remove_liquidity_single_side".into(),
        pool_token_amount,
        minimum_out_amount,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_remove_liquidity_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsRemoveLiquidity(event))
}

fn parse_bootstrap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let token_a_in_amount = read_u64_le(data, 0)?;
    let token_b_in_amount = read_u64_le(data, 8)?;
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsBootstrapLiquidityEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: "bootstrap_liquidity".into(),
        token_a_in_amount,
        token_b_in_amount,
        pool,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_bootstrap_liquidity_accounts(
        &mut event,
        &|i| accounts.get(i).copied().unwrap_or_default(),
    );
    Some(DexEvent::MeteoraPoolsBootstrapLiquidity(event))
}

/// 解析 Create Pool 指令
fn parse_curve_type(data: &[u8]) -> Option<(u8, Option<MeteoraPoolsStableCurveParams>, usize)> {
    match read_u8(data, 0)? {
        0 => Some((0, None, 1)),
        1 => {
            let depeg_type = read_u8(data, 42)?;
            if depeg_type > 3 {
                return None;
            }
            Some((
                1,
                Some(MeteoraPoolsStableCurveParams {
                    amp: read_u64_le(data, 1)?,
                    token_a_multiplier: read_u64_le(data, 9)?,
                    token_b_multiplier: read_u64_le(data, 17)?,
                    precision_factor: read_u8(data, 25)?,
                    base_virtual_price: read_u64_le(data, 26)?,
                    base_cache_updated: read_u64_le(data, 34)?,
                    depeg_type,
                    last_amp_updated_timestamp: read_u64_le(data, 43)?,
                }),
                51,
            ))
        }
        _ => None,
    }
}

fn parse_curve_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    kind: MeteoraPoolsInstruction,
) -> Option<DexEvent> {
    let (_, stable_curve, mut offset) = parse_curve_type(data)?;
    let permissioned = kind == MeteoraPoolsInstruction::InitializePermissionedPool;
    let fee_tier = kind == MeteoraPoolsInstruction::InitializePermissionlessPoolWithFeeTier;
    let trade_fee_bps = if fee_tier {
        let v = read_u64_le(data, offset)?;
        offset += 8;
        Some(v)
    } else {
        None
    };
    let (token_a_in_amount, token_b_in_amount) = if permissioned {
        (0, 0)
    } else {
        (read_u64_le(data, offset)?, read_u64_le(data, offset + 8)?)
    };
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsPoolCreatedEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: if permissioned {
            "initialize_permissioned_pool"
        } else if fee_tier {
            "initialize_permissionless_pool_with_fee_tier"
        } else {
            "initialize_permissionless_pool"
        }
        .into(),
        // PoolType is permission status; CurveType is represented by stable_curve.
        pool_type: if permissioned { 0 } else { 1 },
        stable_curve,
        trade_fee_bps,
        token_a_in_amount,
        token_b_in_amount,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_pool_created_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsPoolCreated(event))
}

fn parse_customizable_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let token_a_in_amount = read_u64_le(data, 0)?;
    let token_b_in_amount = read_u64_le(data, 8)?;
    let trade_fee_numerator = read_u32_le(data, 16)?;
    let (activation_point, offset) = match read_u8(data, 20)? {
        0 => (None, 21),
        1 => (Some(read_u64_le(data, 21)?), 29),
        _ => return None,
    };
    let has_alpha_vault = match read_u8(data, offset)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let activation_type = read_u8(data, offset + 1)?;
    let padding = data.get(offset + 2..offset + 92)?.to_vec();
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsPoolCreatedEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: "initialize_customizable_permissionless_constant_product_pool".into(),
        token_a_in_amount,
        token_b_in_amount,
        activation_point,
        customizable_params: Some(MeteoraPoolsCustomizableParams {
            trade_fee_numerator,
            activation_point,
            has_alpha_vault,
            activation_type,
            padding,
        }),
        pool_type: 1,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_pool_created_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsPoolCreated(event))
}

fn parse_create_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    config2: bool,
) -> Option<DexEvent> {
    let token_a_in_amount = read_u64_le(data, 0)?;
    let token_b_in_amount = read_u64_le(data, 8)?;
    let activation_point = if config2 {
        match read_u8(data, 16)? {
            0 => None,
            1 => Some(read_u64_le(data, 17)?),
            _ => return None,
        }
    } else {
        None
    };
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsPoolCreatedEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: if config2 {
            "initialize_permissionless_constant_product_pool_with_config2"
        } else {
            "initialize_permissionless_constant_product_pool_with_config"
        }
        .into(),
        token_a_in_amount,
        token_b_in_amount,
        activation_point,
        // PoolType::Permissionless; independent of the ConstantProduct curve.
        pool_type: 1,
        ..Default::default()
    };
    crate::core::account_fillers::meteora::fill_pools_pool_created_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsPoolCreated(event))
}

fn parse_set_pool_fees_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 0)?;
    let mut event = MeteoraPoolsSetPoolFeesEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool),
        ix_name: "set_pool_fees".into(),
        trade_fee_numerator: read_u64_le(data, 0)?,
        trade_fee_denominator: read_u64_le(data, 8)?,
        protocol_trade_fee_numerator: read_u64_le(data, 16)?,
        protocol_trade_fee_denominator: read_u64_le(data, 24)?,
        new_partner_fee_numerator: read_u64_le(data, 32)?,
        ..Default::default()
    };
    // Preserve the historical aliases for protocol fees.
    event.owner_trade_fee_numerator = event.protocol_trade_fee_numerator;
    event.owner_trade_fee_denominator = event.protocol_trade_fee_denominator;
    crate::core::account_fillers::meteora::fill_pools_set_pool_fees_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::MeteoraPoolsSetPoolFees(event))
}

#[cfg(test)]
mod pools_swap_wire_tests {
    use super::*;

    #[test]
    fn swap_accounts_and_extreme_parameters_match_idl_without_execution_guesses() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        for (input, minimum) in [(0u64, u64::MAX), (u64::MAX, 0)] {
            let mut data = discriminators::SWAP.to_vec();
            data.extend_from_slice(&input.to_le_bytes());
            data.extend_from_slice(&minimum.to_le_bytes());
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsSwap(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("swap")
                };
                assert_eq!(e.ix_name, "swap");
                assert_eq!((e.amount_in, e.minimum_out_amount), (input, minimum));
                assert_eq!(
                    (e.in_amount, e.out_amount, e.trade_fee, e.admin_fee, e.host_fee),
                    (0, 0, 0, 0, 0)
                );
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(
                    e.user_source_token,
                    accounts[..count].get(1).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.user_destination_token,
                    accounts[..count].get(2).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_token_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault_lp, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(
                    e.protocol_token_fee,
                    accounts[..count].get(11).copied().unwrap_or_default()
                );
                assert_eq!(e.user, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(14).copied().unwrap_or_default());
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
        }
    }
}

#[cfg(test)]
mod liquidity_idl_tests {
    use super::*;
    #[test]
    fn add_balance_liquidity_matches_idl_and_keeps_limits_separate() {
        let accounts: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        for values in [[0u64, u64::MAX, 0], [u64::MAX, 0, u64::MAX]] {
            let mut data = vec![168, 227, 50, 62, 189, 171, 84, 176];
            for value in values {
                data.extend_from_slice(&value.to_le_bytes());
            }
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsAddLiquidity(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("liquidity")
                };
                assert_eq!(e.ix_name, "add_balance_liquidity");
                assert_eq!(e.pool_token_amount, values[0]);
                assert_eq!(e.maximum_token_a_amount, values[1]);
                assert_eq!(e.maximum_token_b_amount, values[2]);
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.user_pool_lp, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.a_vault_lp, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_token_vault, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.user_a_token, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.user_b_token, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.user, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(15).copied().unwrap_or_default());
                assert_eq!(e.lp_mint_amount, 0);
                assert_eq!(e.token_a_amount, 0);
                assert_eq!(e.token_b_amount, 0);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
        }
    }
    #[test]
    fn add_imbalance_liquidity_matches_idl_and_keeps_limits_separate() {
        let accounts: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        for values in [[0u64, u64::MAX, 0], [u64::MAX, 0, u64::MAX]] {
            let mut data = vec![79, 35, 122, 84, 173, 15, 93, 191];
            for value in values {
                data.extend_from_slice(&value.to_le_bytes());
            }
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsAddLiquidity(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("liquidity")
                };
                assert_eq!(e.ix_name, "add_imbalance_liquidity");
                assert_eq!(e.minimum_pool_token_amount, values[0]);
                assert_eq!(e.token_a_in_amount, values[1]);
                assert_eq!(e.token_b_in_amount, values[2]);
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.user_pool_lp, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.a_vault_lp, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_token_vault, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.user_a_token, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.user_b_token, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.user, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(15).copied().unwrap_or_default());
                assert_eq!(e.lp_mint_amount, 0);
                assert_eq!(e.token_a_amount, 0);
                assert_eq!(e.token_b_amount, 0);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
        }
    }
    #[test]
    fn remove_balance_liquidity_matches_idl_and_keeps_limits_separate() {
        let accounts: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        for values in [[0u64, u64::MAX, 0], [u64::MAX, 0, u64::MAX]] {
            let mut data = vec![133, 109, 44, 179, 56, 238, 114, 33];
            for value in values {
                data.extend_from_slice(&value.to_le_bytes());
            }
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsRemoveLiquidity(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("liquidity")
                };
                assert_eq!(e.ix_name, "remove_balance_liquidity");
                assert_eq!(e.pool_token_amount, values[0]);
                assert_eq!(e.minimum_a_token_out, values[1]);
                assert_eq!(e.minimum_b_token_out, values[2]);
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.user_pool_lp, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.a_vault_lp, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_token_vault, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.user_a_token, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.user_b_token, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.user, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(15).copied().unwrap_or_default());
                assert_eq!(e.lp_unmint_amount, 0);
                assert_eq!(e.token_a_out_amount, 0);
                assert_eq!(e.token_b_out_amount, 0);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
        }
    }
    #[test]
    fn incorrect_legacy_discriminators_are_not_accepted() {
        for disc in [[181, 157, 89, 67, 143, 182, 52, 72], [80, 85, 209, 72, 24, 206, 177, 108]] {
            let mut data = disc.to_vec();
            data.extend_from_slice(&[0; 24]);
            assert!(!crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            assert!(parse_instruction(
                &data,
                &[Pubkey::new_unique(); 16],
                Signature::default(),
                1,
                0,
                None
            )
            .is_none());
        }
    }
}

#[cfg(test)]
mod remaining_liquidity_idl_tests {
    use super::*;
    #[test]
    fn remove_liquidity_single_side_parameters_and_account_prefixes_match_idl() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        for values in [[0u64, u64::MAX], [u64::MAX, 0]] {
            let mut data = vec![84, 84, 177, 66, 254, 185, 10, 251];
            for v in values {
                data.extend_from_slice(&v.to_le_bytes());
            }
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsRemoveLiquidity(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("liquidity")
                };
                assert_eq!(e.ix_name, "remove_liquidity_single_side");
                assert_eq!(e.pool_token_amount, values[0]);
                assert_eq!(e.minimum_out_amount, values[1]);
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.user_pool_lp, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.a_vault_lp, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_token_vault, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(
                    e.user_destination_token,
                    accounts[..count].get(11).copied().unwrap_or_default()
                );
                assert_eq!(e.user, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(e.lp_unmint_amount, 0);
                assert_eq!(e.token_a_out_amount, 0);
                assert_eq!(e.token_b_out_amount, 0);
                assert_eq!(e.user_a_token, Pubkey::default());
                assert_eq!(e.user_b_token, Pubkey::default());
                assert_eq!((e.minimum_a_token_out, e.minimum_b_token_out), (0, 0));
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            assert!(parse_instruction(&data, &[], Signature::default(), 1, 0, None).is_none());
        }
    }
    #[test]
    fn bootstrap_liquidity_parameters_and_account_prefixes_match_idl() {
        let accounts: Vec<_> = (0..16).map(|_| Pubkey::new_unique()).collect();
        for values in [[0u64, u64::MAX], [u64::MAX, 0]] {
            let mut data = vec![4, 228, 215, 71, 225, 253, 119, 206];
            for v in values {
                data.extend_from_slice(&v.to_le_bytes());
            }
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsBootstrapLiquidity(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("liquidity")
                };
                assert_eq!(e.ix_name, "bootstrap_liquidity");
                assert_eq!(e.token_a_in_amount, values[0]);
                assert_eq!(e.token_b_in_amount, values[1]);
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.user_pool_lp, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.a_vault_lp, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(e.a_token_vault, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.user_a_token, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.user_b_token, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.user, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.vault_program, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(15).copied().unwrap_or_default());
                assert_eq!(e.lp_mint_amount, 0);
                assert_eq!(e.token_a_amount, 0);
                assert_eq!(e.token_b_amount, 0);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            assert!(parse_instruction(&data, &[], Signature::default(), 1, 0, None).is_none());
        }
    }
}

#[cfg(test)]
mod pools_management_idl_tests {
    use super::*;
    #[test]
    fn config_creation_accounts_inputs_and_activation_follow_idl() {
        let accounts: Vec<_> = (0..26).map(|_| Pubkey::new_unique()).collect();
        for config2 in [false, true] {
            for activation in [None, Some(0u64), Some(u64::MAX)] {
                let mut data = if config2 {
                    vec![48, 149, 220, 130, 61, 11, 9, 178]
                } else {
                    vec![7, 166, 138, 171, 206, 171, 236, 244]
                };
                data.extend_from_slice(&0u64.to_le_bytes());
                data.extend_from_slice(&u64::MAX.to_le_bytes());
                if config2 {
                    data.push(activation.is_some() as u8);
                    if let Some(v) = activation {
                        data.extend_from_slice(&v.to_le_bytes());
                    }
                }
                assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
                for count in 1..=accounts.len() {
                    let DexEvent::MeteoraPoolsPoolCreated(e) = parse_instruction(
                        &data,
                        &accounts[..count],
                        Signature::default(),
                        1,
                        0,
                        None,
                    )
                    .unwrap() else {
                        panic!("create")
                    };
                    assert_eq!(
                        e.ix_name,
                        if config2 {
                            "initialize_permissionless_constant_product_pool_with_config2"
                        } else {
                            "initialize_permissionless_constant_product_pool_with_config"
                        }
                    );
                    assert_eq!((e.token_a_in_amount, e.token_b_in_amount), (0, u64::MAX));
                    assert_eq!(e.activation_point, if config2 { activation } else { None });
                    assert_eq!(e.pool_type, 1);
                    assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                    assert_eq!(e.config, accounts[..count].get(1).copied().unwrap_or_default());
                    assert_eq!(e.lp_mint, accounts[..count].get(2).copied().unwrap_or_default());
                    assert_eq!(
                        e.token_a_mint,
                        accounts[..count].get(3).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.token_b_mint,
                        accounts[..count].get(4).copied().unwrap_or_default()
                    );
                    assert_eq!(e.a_vault, accounts[..count].get(5).copied().unwrap_or_default());
                    assert_eq!(e.b_vault, accounts[..count].get(6).copied().unwrap_or_default());
                    assert_eq!(
                        e.a_token_vault,
                        accounts[..count].get(7).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.b_token_vault,
                        accounts[..count].get(8).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.a_vault_lp_mint,
                        accounts[..count].get(9).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.b_vault_lp_mint,
                        accounts[..count].get(10).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.a_vault_lp,
                        accounts[..count].get(11).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.b_vault_lp,
                        accounts[..count].get(12).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.payer_token_a,
                        accounts[..count].get(13).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.payer_token_b,
                        accounts[..count].get(14).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.payer_pool_lp,
                        accounts[..count].get(15).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.protocol_token_a_fee,
                        accounts[..count].get(16).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.protocol_token_b_fee,
                        accounts[..count].get(17).copied().unwrap_or_default()
                    );
                    assert_eq!(e.payer, accounts[..count].get(18).copied().unwrap_or_default());
                    assert_eq!(e.rent, accounts[..count].get(19).copied().unwrap_or_default());
                    assert_eq!(
                        e.mint_metadata,
                        accounts[..count].get(20).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.metadata_program,
                        accounts[..count].get(21).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.vault_program,
                        accounts[..count].get(22).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.token_program,
                        accounts[..count].get(23).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.associated_token_program,
                        accounts[..count].get(24).copied().unwrap_or_default()
                    );
                    assert_eq!(
                        e.system_program,
                        accounts[..count].get(25).copied().unwrap_or_default()
                    );
                }
                for end in 0..data.len() {
                    assert!(parse_instruction(
                        &data[..end],
                        &accounts,
                        Signature::default(),
                        1,
                        0,
                        None
                    )
                    .is_none());
                }
                if config2 {
                    data[24] = 2;
                    assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None)
                        .is_none());
                }
            }
        }
        let mut obsolete = vec![95, 180, 10, 172, 84, 174, 232, 40];
        obsolete.extend_from_slice(&[0; 49]);
        assert!(!crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &obsolete));
        assert!(parse_instruction(&obsolete, &accounts, Signature::default(), 1, 0, None).is_none());
    }
    #[test]
    fn set_pool_fees_uses_five_u64_arguments_and_two_accounts() {
        let accounts = [Pubkey::new_unique(), Pubkey::new_unique()];
        let values = [0u64, u64::MAX, u64::MAX, 0, u64::MAX];
        let mut data = vec![102, 44, 158, 54, 205, 37, 126, 78];
        for v in values {
            data.extend_from_slice(&v.to_le_bytes());
        }
        assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
        let DexEvent::MeteoraPoolsSetPoolFees(e) =
            parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap()
        else {
            panic!("fees")
        };
        assert_eq!(e.trade_fee_numerator, values[0]);
        assert_eq!(e.trade_fee_denominator, values[1]);
        assert_eq!(e.protocol_trade_fee_numerator, values[2]);
        assert_eq!(e.protocol_trade_fee_denominator, values[3]);
        assert_eq!(e.new_partner_fee_numerator, values[4]);
        assert_eq!(
            (e.owner_trade_fee_numerator, e.owner_trade_fee_denominator),
            (values[2], values[3])
        );
        assert_eq!((e.pool, e.fee_operator), (accounts[0], accounts[1]));
        for end in 0..data.len() {
            assert!(parse_instruction(&data[..end], &accounts, Signature::default(), 1, 0, None)
                .is_none());
        }
    }
}

#[cfg(test)]
mod remaining_creation_idl_tests {
    use super::*;
    fn stable_bytes(depeg: u8) -> Vec<u8> {
        let mut d = vec![1];
        for v in [0u64, u64::MAX, 123] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.push(9);
        for v in [456u64, 789] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.push(depeg);
        d.extend_from_slice(&987u64.to_le_bytes());
        d
    }
    #[test]
    fn initialize_permissioned_pool_matches_parameters_accounts_and_rejects_truncation() {
        let accounts: Vec<_> = (0..24).map(|_| Pubkey::new_unique()).collect();
        for curve in [vec![0], stable_bytes(0), stable_bytes(1), stable_bytes(2), stable_bytes(3)] {
            let mut data = vec![77, 85, 178, 157, 50, 48, 212, 126];
            data.extend_from_slice(&curve);
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsPoolCreated(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("create")
                };
                assert_eq!(e.ix_name, "initialize_permissioned_pool");
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.token_a_mint, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.token_b_mint, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(6).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(7).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault_lp, accounts[..count].get(8).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(9).copied().unwrap_or_default());
                assert_eq!(e.admin_token_a, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.admin_token_b, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.admin_pool_lp, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(
                    e.protocol_token_a_fee,
                    accounts[..count].get(13).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.protocol_token_b_fee,
                    accounts[..count].get(14).copied().unwrap_or_default()
                );
                assert_eq!(e.admin, accounts[..count].get(15).copied().unwrap_or_default());
                assert_eq!(e.fee_owner, accounts[..count].get(16).copied().unwrap_or_default());
                assert_eq!(e.rent, accounts[..count].get(17).copied().unwrap_or_default());
                assert_eq!(e.mint_metadata, accounts[..count].get(18).copied().unwrap_or_default());
                assert_eq!(
                    e.metadata_program,
                    accounts[..count].get(19).copied().unwrap_or_default()
                );
                assert_eq!(e.vault_program, accounts[..count].get(20).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(21).copied().unwrap_or_default());
                assert_eq!(
                    e.associated_token_program,
                    accounts[..count].get(22).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.system_program,
                    accounts[..count].get(23).copied().unwrap_or_default()
                );
                assert_eq!(e.pool_type, 0);
                if curve[0] == 0 {
                    assert!(e.stable_curve.is_none());
                } else {
                    let p = e.stable_curve.unwrap();
                    assert_eq!(
                        (p.amp, p.token_a_multiplier, p.token_b_multiplier),
                        (0, u64::MAX, 123)
                    );
                    assert_eq!(
                        (
                            p.precision_factor,
                            p.base_virtual_price,
                            p.base_cache_updated,
                            p.depeg_type,
                            p.last_amp_updated_timestamp
                        ),
                        (9, 456, 789, curve[42], 987)
                    );
                }
                assert_eq!((e.token_a_in_amount, e.token_b_in_amount), (0, 0));
                assert_eq!(e.trade_fee_bps, None);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            data[8] = 2;
            assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none());
            if curve[0] == 1 {
                data[8] = 1;
                data[50] = 4;
                assert!(
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none()
                );
            }
        }
    }
    #[test]
    fn initialize_permissionless_pool_matches_parameters_accounts_and_rejects_truncation() {
        let accounts: Vec<_> = (0..26).map(|_| Pubkey::new_unique()).collect();
        for curve in [vec![0], stable_bytes(0), stable_bytes(1), stable_bytes(2), stable_bytes(3)] {
            let mut data = vec![118, 173, 41, 157, 173, 72, 97, 103];
            data.extend_from_slice(&curve);
            data.extend_from_slice(&0u64.to_le_bytes());
            data.extend_from_slice(&u64::MAX.to_le_bytes());
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsPoolCreated(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("create")
                };
                assert_eq!(e.ix_name, "initialize_permissionless_pool");
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.token_a_mint, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.token_b_mint, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.a_token_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(7).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(9).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault_lp, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.payer_token_a, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.payer_token_b, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.payer_pool_lp, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(
                    e.protocol_token_a_fee,
                    accounts[..count].get(15).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.protocol_token_b_fee,
                    accounts[..count].get(16).copied().unwrap_or_default()
                );
                assert_eq!(e.payer, accounts[..count].get(17).copied().unwrap_or_default());
                assert_eq!(e.fee_owner, accounts[..count].get(18).copied().unwrap_or_default());
                assert_eq!(e.rent, accounts[..count].get(19).copied().unwrap_or_default());
                assert_eq!(e.mint_metadata, accounts[..count].get(20).copied().unwrap_or_default());
                assert_eq!(
                    e.metadata_program,
                    accounts[..count].get(21).copied().unwrap_or_default()
                );
                assert_eq!(e.vault_program, accounts[..count].get(22).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(23).copied().unwrap_or_default());
                assert_eq!(
                    e.associated_token_program,
                    accounts[..count].get(24).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.system_program,
                    accounts[..count].get(25).copied().unwrap_or_default()
                );
                assert_eq!(e.pool_type, 1);
                if curve[0] == 0 {
                    assert!(e.stable_curve.is_none());
                } else {
                    let p = e.stable_curve.unwrap();
                    assert_eq!(
                        (p.amp, p.token_a_multiplier, p.token_b_multiplier),
                        (0, u64::MAX, 123)
                    );
                    assert_eq!(
                        (
                            p.precision_factor,
                            p.base_virtual_price,
                            p.base_cache_updated,
                            p.depeg_type,
                            p.last_amp_updated_timestamp
                        ),
                        (9, 456, 789, curve[42], 987)
                    );
                }
                assert_eq!((e.token_a_in_amount, e.token_b_in_amount), (0, u64::MAX));
                assert_eq!(e.trade_fee_bps, None);
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            data[8] = 2;
            assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none());
            if curve[0] == 1 {
                data[8] = 1;
                data[50] = 4;
                assert!(
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none()
                );
            }
        }
    }
    #[test]
    fn initialize_permissionless_pool_with_fee_tier_matches_parameters_accounts_and_rejects_truncation(
    ) {
        let accounts: Vec<_> = (0..26).map(|_| Pubkey::new_unique()).collect();
        for curve in [vec![0], stable_bytes(0), stable_bytes(1), stable_bytes(2), stable_bytes(3)] {
            let mut data = vec![6, 135, 68, 147, 229, 82, 169, 113];
            data.extend_from_slice(&curve);
            data.extend_from_slice(&0u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes());
            data.extend_from_slice(&u64::MAX.to_le_bytes());
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsPoolCreated(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("create")
                };
                assert_eq!(e.ix_name, "initialize_permissionless_pool_with_fee_tier");
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.token_a_mint, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.token_b_mint, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.a_token_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(7).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(9).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault_lp, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.payer_token_a, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.payer_token_b, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.payer_pool_lp, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(
                    e.protocol_token_a_fee,
                    accounts[..count].get(15).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.protocol_token_b_fee,
                    accounts[..count].get(16).copied().unwrap_or_default()
                );
                assert_eq!(e.payer, accounts[..count].get(17).copied().unwrap_or_default());
                assert_eq!(e.fee_owner, accounts[..count].get(18).copied().unwrap_or_default());
                assert_eq!(e.rent, accounts[..count].get(19).copied().unwrap_or_default());
                assert_eq!(e.mint_metadata, accounts[..count].get(20).copied().unwrap_or_default());
                assert_eq!(
                    e.metadata_program,
                    accounts[..count].get(21).copied().unwrap_or_default()
                );
                assert_eq!(e.vault_program, accounts[..count].get(22).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(23).copied().unwrap_or_default());
                assert_eq!(
                    e.associated_token_program,
                    accounts[..count].get(24).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.system_program,
                    accounts[..count].get(25).copied().unwrap_or_default()
                );
                assert_eq!(e.pool_type, 1);
                if curve[0] == 0 {
                    assert!(e.stable_curve.is_none());
                } else {
                    let p = e.stable_curve.unwrap();
                    assert_eq!(
                        (p.amp, p.token_a_multiplier, p.token_b_multiplier),
                        (0, u64::MAX, 123)
                    );
                    assert_eq!(
                        (
                            p.precision_factor,
                            p.base_virtual_price,
                            p.base_cache_updated,
                            p.depeg_type,
                            p.last_amp_updated_timestamp
                        ),
                        (9, 456, 789, curve[42], 987)
                    );
                }
                assert_eq!((e.token_a_in_amount, e.token_b_in_amount), (0, u64::MAX));
                assert_eq!(e.trade_fee_bps, Some(0));
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            data[8] = 2;
            assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none());
            if curve[0] == 1 {
                data[8] = 1;
                data[50] = 4;
                assert!(
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none()
                );
            }
        }
    }
    #[test]
    fn initialize_customizable_permissionless_constant_product_pool_matches_parameters_accounts_and_rejects_truncation(
    ) {
        let accounts: Vec<_> = (0..25).map(|_| Pubkey::new_unique()).collect();
        for activation in [None, Some(0u64), Some(u64::MAX)] {
            let mut data = vec![145, 24, 172, 194, 219, 125, 3, 190];
            data.extend_from_slice(&0u64.to_le_bytes());
            data.extend_from_slice(&u64::MAX.to_le_bytes());
            data.extend_from_slice(&u32::MAX.to_le_bytes());
            data.push(activation.is_some() as u8);
            if let Some(v) = activation {
                data.extend_from_slice(&v.to_le_bytes());
            }
            data.extend_from_slice(&[1, 255]);
            data.extend(0..90u8);
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &data));
            for count in 1..=accounts.len() {
                let DexEvent::MeteoraPoolsPoolCreated(e) =
                    parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None)
                        .unwrap()
                else {
                    panic!("create")
                };
                assert_eq!(
                    e.ix_name,
                    "initialize_customizable_permissionless_constant_product_pool"
                );
                assert_eq!(e.pool, accounts[..count].get(0).copied().unwrap_or_default());
                assert_eq!(e.lp_mint, accounts[..count].get(1).copied().unwrap_or_default());
                assert_eq!(e.token_a_mint, accounts[..count].get(2).copied().unwrap_or_default());
                assert_eq!(e.token_b_mint, accounts[..count].get(3).copied().unwrap_or_default());
                assert_eq!(e.a_vault, accounts[..count].get(4).copied().unwrap_or_default());
                assert_eq!(e.b_vault, accounts[..count].get(5).copied().unwrap_or_default());
                assert_eq!(e.a_token_vault, accounts[..count].get(6).copied().unwrap_or_default());
                assert_eq!(e.b_token_vault, accounts[..count].get(7).copied().unwrap_or_default());
                assert_eq!(
                    e.a_vault_lp_mint,
                    accounts[..count].get(8).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.b_vault_lp_mint,
                    accounts[..count].get(9).copied().unwrap_or_default()
                );
                assert_eq!(e.a_vault_lp, accounts[..count].get(10).copied().unwrap_or_default());
                assert_eq!(e.b_vault_lp, accounts[..count].get(11).copied().unwrap_or_default());
                assert_eq!(e.payer_token_a, accounts[..count].get(12).copied().unwrap_or_default());
                assert_eq!(e.payer_token_b, accounts[..count].get(13).copied().unwrap_or_default());
                assert_eq!(e.payer_pool_lp, accounts[..count].get(14).copied().unwrap_or_default());
                assert_eq!(
                    e.protocol_token_a_fee,
                    accounts[..count].get(15).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.protocol_token_b_fee,
                    accounts[..count].get(16).copied().unwrap_or_default()
                );
                assert_eq!(e.payer, accounts[..count].get(17).copied().unwrap_or_default());
                assert_eq!(e.rent, accounts[..count].get(18).copied().unwrap_or_default());
                assert_eq!(e.mint_metadata, accounts[..count].get(19).copied().unwrap_or_default());
                assert_eq!(
                    e.metadata_program,
                    accounts[..count].get(20).copied().unwrap_or_default()
                );
                assert_eq!(e.vault_program, accounts[..count].get(21).copied().unwrap_or_default());
                assert_eq!(e.token_program, accounts[..count].get(22).copied().unwrap_or_default());
                assert_eq!(
                    e.associated_token_program,
                    accounts[..count].get(23).copied().unwrap_or_default()
                );
                assert_eq!(
                    e.system_program,
                    accounts[..count].get(24).copied().unwrap_or_default()
                );
                assert_eq!(e.pool_type, 1);
                assert_eq!(e.activation_point, activation);
                assert_eq!((e.token_a_in_amount, e.token_b_in_amount), (0, u64::MAX));
                let p = e.customizable_params.unwrap();
                assert_eq!(p.trade_fee_numerator, u32::MAX);
                assert_eq!(p.activation_point, activation);
                assert!(p.has_alpha_vault);
                assert_eq!(p.activation_type, 255);
                assert_eq!(p.padding, (0..90u8).collect::<Vec<_>>());
                assert!(e.stable_curve.is_none());
                assert!(e.trade_fee_bps.is_none());
            }
            for end in 0..data.len() {
                assert!(parse_instruction(
                    &data[..end],
                    &accounts,
                    Signature::default(),
                    1,
                    0,
                    None
                )
                .is_none());
            }
            let bool_index = if activation.is_some() { 37 } else { 29 };
            data[bool_index] = 2;
            assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none());
            data[28] = 2;
            assert!(parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).is_none());
        }
    }
}
