//! Orca Whirlpool 指令解析器
//!
//! 使用 match discriminator 模式解析 Orca Whirlpool 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Orca Whirlpool 指令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrcaWhirlpoolInstruction {
    InitializeConfig = 0,
    InitializePool = 1,
    InitializeTickArray = 2,
    InitializeFeeTier = 3,
    InitializeReward = 4,
    SetRewardEmissions = 5,
    OpenPosition = 6,
    OpenPositionWithMetadata = 7,
    IncreaseLiquidity = 8,
    DecreaseLiquidity = 9,
    UpdateFeesAndRewards = 10,
    CollectFees = 11,
    CollectReward = 12,
    CollectProtocolFees = 13,
    Swap = 14,
    ClosePosition = 15,
    SetDefaultFeeRate = 16,
    SetDefaultProtocolFeeRate = 17,
    SetFeeRate = 18,
    SetProtocolFeeRate = 19,
    SetFeeAuthority = 20,
    SetCollectProtocolFeesAuthority = 21,
    SetRewardAuthority = 22,
    SetRewardAuthorityBySuperAuthority = 23,
    SetRewardEmissionsSuperAuthority = 24,
    TwoHopSwap = 25,
    InitializePositionBundle = 26,
    InitializePositionBundleWithMetadata = 27,
    DeletePositionBundle = 28,
    OpenBundledPosition = 29,
    CloseBundledPosition = 30,
    CollectFeesV2 = 31,
    CollectProtocolFeesV2 = 32,
    CollectRewardV2 = 33,
    DecreaseLiquidityV2 = 34,
    IncreaseLiquidityV2 = 35,
    InitializePoolV2 = 36,
    InitializeRewardV2 = 37,
    SetRewardEmissionsV2 = 38,
    SwapV2 = 39,
    TwoHopSwapV2 = 40,
}

impl OrcaWhirlpoolInstruction {
    /// 从 discriminator 转换为指令类型
    pub fn from_discriminator(discriminator: &[u8; 8]) -> Option<Self> {
        match *discriminator {
            [208, 127, 21, 1, 194, 190, 196, 70] => Some(Self::InitializeConfig),
            [95, 180, 10, 172, 84, 174, 232, 40] => Some(Self::InitializePool),
            [207, 45, 87, 242, 27, 63, 204, 67] => Some(Self::InitializePoolV2),
            [214, 27, 15, 109, 164, 252, 221, 253] => Some(Self::InitializeTickArray),
            [183, 74, 156, 160, 112, 2, 42, 30] => Some(Self::InitializeFeeTier),
            [95, 135, 192, 196, 242, 129, 230, 68] => Some(Self::InitializeReward),
            [13, 197, 86, 168, 109, 176, 27, 244] => Some(Self::SetRewardEmissions),
            [87, 190, 72, 189, 204, 203, 226, 66] => Some(Self::OpenPosition),
            [78, 217, 28, 185, 88, 104, 255, 231] => Some(Self::OpenPositionWithMetadata),
            [46, 156, 243, 118, 13, 205, 251, 178] => Some(Self::IncreaseLiquidity),
            [133, 29, 89, 223, 69, 238, 176, 10] => Some(Self::IncreaseLiquidityV2),
            [160, 38, 208, 111, 104, 91, 44, 1] => Some(Self::DecreaseLiquidity),
            [58, 127, 188, 62, 79, 82, 196, 96] => Some(Self::DecreaseLiquidityV2),
            [173, 178, 66, 24, 33, 156, 204, 31] => Some(Self::UpdateFeesAndRewards),
            [164, 152, 207, 99, 30, 186, 19, 182] => Some(Self::CollectFees),
            [206, 68, 114, 253, 168, 177, 245, 180] => Some(Self::CollectReward),
            [22, 67, 23, 98, 150, 178, 70, 220] => Some(Self::CollectProtocolFees),
            [248, 198, 158, 145, 225, 117, 135, 200] => Some(Self::Swap),
            [123, 134, 81, 0, 49, 68, 98, 98] => Some(Self::ClosePosition),
            [43, 4, 237, 11, 26, 201, 30, 98] => Some(Self::SwapV2),
            [195, 96, 237, 108, 68, 162, 219, 230] => Some(Self::TwoHopSwap),
            [186, 143, 209, 29, 254, 2, 194, 117] => Some(Self::TwoHopSwapV2),
            _ => None,
        }
    }
}

/// Orca Whirlpool discriminator 常量
pub mod discriminators {
    pub const INITIALIZE_CONFIG: [u8; 8] = [208, 127, 21, 1, 194, 190, 196, 70];
    pub const INITIALIZE_POOL: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];
    pub const INITIALIZE_POOL_V2: [u8; 8] = [207, 45, 87, 242, 27, 63, 204, 67];
    pub const INCREASE_LIQUIDITY_V2: [u8; 8] = [133, 29, 89, 223, 69, 238, 176, 10];
    pub const DECREASE_LIQUIDITY_V2: [u8; 8] = [58, 127, 188, 62, 79, 82, 196, 96];
    pub const INITIALIZE_TICK_ARRAY: [u8; 8] = [214, 27, 15, 109, 164, 252, 221, 253];
    pub const INITIALIZE_FEE_TIER: [u8; 8] = [183, 74, 156, 160, 112, 2, 42, 30];
    pub const INITIALIZE_REWARD: [u8; 8] = [95, 135, 192, 196, 242, 129, 230, 68];
    pub const SET_REWARD_EMISSIONS: [u8; 8] = [13, 197, 86, 168, 109, 176, 27, 244];
    pub const OPEN_POSITION: [u8; 8] = [87, 190, 72, 189, 204, 203, 226, 66];
    pub const OPEN_POSITION_WITH_METADATA: [u8; 8] = [78, 217, 28, 185, 88, 104, 255, 231];
    pub const INCREASE_LIQUIDITY: [u8; 8] = [46, 156, 243, 118, 13, 205, 251, 178];
    pub const DECREASE_LIQUIDITY: [u8; 8] = [160, 38, 208, 111, 104, 91, 44, 1];
    pub const UPDATE_FEES_AND_REWARDS: [u8; 8] = [173, 178, 66, 24, 33, 156, 204, 31];
    pub const COLLECT_FEES: [u8; 8] = [164, 152, 207, 99, 30, 186, 19, 182];
    pub const COLLECT_REWARD: [u8; 8] = [206, 68, 114, 253, 168, 177, 245, 180];
    pub const COLLECT_PROTOCOL_FEES: [u8; 8] = [22, 67, 23, 98, 150, 178, 70, 220];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const TWO_HOP_SWAP: [u8; 8] = [195, 96, 237, 108, 68, 162, 219, 230];
    pub const TWO_HOP_SWAP_V2: [u8; 8] = [186, 143, 209, 29, 254, 2, 194, 117];
}

/// Orca Whirlpool 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::ORCA_WHIRLPOOL_PROGRAM_ID;

/// 主要的 Orca Whirlpool 指令解析函数
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
    let instruction_type = OrcaWhirlpoolInstruction::from_discriminator(&discriminator)?;
    let data = &instruction_data[8..];

    match instruction_type {
        OrcaWhirlpoolInstruction::Swap => {
            parse_swap_instruction(data, accounts, 2, signature, slot, tx_index, block_time_us)
        }
        OrcaWhirlpoolInstruction::SwapV2 => {
            parse_swap_instruction(data, accounts, 4, signature, slot, tx_index, block_time_us)
        }
        OrcaWhirlpoolInstruction::IncreaseLiquidity
        | OrcaWhirlpoolInstruction::IncreaseLiquidityV2 => parse_increase_liquidity_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            instruction_type == OrcaWhirlpoolInstruction::IncreaseLiquidityV2,
        ),
        OrcaWhirlpoolInstruction::DecreaseLiquidity
        | OrcaWhirlpoolInstruction::DecreaseLiquidityV2 => parse_decrease_liquidity_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            instruction_type == OrcaWhirlpoolInstruction::DecreaseLiquidityV2,
        ),
        OrcaWhirlpoolInstruction::InitializePool | OrcaWhirlpoolInstruction::InitializePoolV2 => {
            parse_initialize_pool_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                instruction_type == OrcaWhirlpoolInstruction::InitializePoolV2,
            )
        }
        _ => None, // 其他指令暂不解析
    }
}

/// 解析 Swap 指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    whirlpool_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount = read_u64_le(data, offset)?;
    offset += 8;

    let other_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let sqrt_price_limit = read_u128_le(data, offset)?;
    offset += 16;

    let amount_specified_is_input = read_bool(data, offset)?;
    offset += 1;

    let a_to_b = read_bool(data, offset)?;

    let whirlpool = get_account(accounts, whirlpool_index)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, whirlpool);

    let mut event = OrcaWhirlpoolSwapEvent {
        metadata,
        whirlpool,
        a_to_b,
        ix_name: if whirlpool_index == 4 { "swap_v2" } else { "swap" }.to_string(),
        amount,
        other_amount_threshold,
        sqrt_price_limit,
        amount_specified_is_input,
        pre_sqrt_price: 0,
        post_sqrt_price: 0,
        input_amount: 0,
        output_amount: 0,
        input_transfer_fee: 0,
        output_transfer_fee: 0,
        lp_fee: 0,
        protocol_fee: 0,
        ..Default::default()
    };
    crate::core::account_fillers::orca::fill_whirlpool_swap_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::OrcaWhirlpoolSwap(event))
}

/// 解析 Increase Liquidity 指令
fn parse_increase_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    v2: bool,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_amount = read_u128_le(data, offset)?;
    offset += 16;

    let token_max_a = read_u64_le(data, offset)?;
    offset += 8;

    let token_max_b = read_u64_le(data, offset)?;

    let whirlpool = get_account(accounts, 0)?;
    let position = get_account(accounts, if v2 { 5 } else { 3 })?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, whirlpool);

    Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index: 0, // 从日志中获取
        tick_upper_index: 0, // 从日志中获取
        liquidity: liquidity_amount,
        token_a_amount: token_max_a, // 从指令获取最大值，日志会覆盖实际值
        token_b_amount: token_max_b, // 从指令获取最大值，日志会覆盖实际值
        token_a_transfer_fee: 0,     // 从日志中获取
        token_b_transfer_fee: 0,     // 从日志中获取
    }))
}

/// 解析 Decrease Liquidity 指令
fn parse_decrease_liquidity_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    v2: bool,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity_amount = read_u128_le(data, offset)?;
    offset += 16;

    let token_min_a = read_u64_le(data, offset)?;
    offset += 8;

    let token_min_b = read_u64_le(data, offset)?;

    let whirlpool = get_account(accounts, 0)?;
    let position = get_account(accounts, if v2 { 5 } else { 3 })?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, whirlpool);

    Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent {
        metadata,
        whirlpool,
        position,
        tick_lower_index: 0, // 从日志中获取
        tick_upper_index: 0, // 从日志中获取
        liquidity: liquidity_amount,
        token_a_amount: token_min_a, // 从指令获取最小值，日志会覆盖实际值
        token_b_amount: token_min_b, // 从指令获取最小值，日志会覆盖实际值
        token_a_transfer_fee: 0,     // 从日志中获取
        token_b_transfer_fee: 0,     // 从日志中获取
    }))
}

/// 解析 Initialize Pool 指令
fn parse_initialize_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    v2: bool,
) -> Option<DexEvent> {
    // Legacy initialize_pool begins with WhirlpoolBumps (one u8).
    let mut offset = if v2 { 0 } else { 1 };

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let initial_sqrt_price = read_u128_le(data, offset)?;

    let whirlpool = get_account(accounts, if v2 { 6 } else { 4 })?;
    let whirlpools_config = get_account(accounts, 0)?;
    let token_mint_a = get_account(accounts, 1)?;
    let token_mint_b = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, whirlpool);

    Some(DexEvent::OrcaWhirlpoolPoolInitialized(OrcaWhirlpoolPoolInitializedEvent {
        metadata,
        whirlpool,
        whirlpools_config,
        token_mint_a,
        token_mint_b,
        tick_spacing,
        token_program_a: get_account(accounts, if v2 { 10 } else { 8 }).unwrap_or_default(),
        token_program_b: get_account(accounts, if v2 { 11 } else { 8 }).unwrap_or_default(),
        decimals_a: 0, // 从日志中获取
        decimals_b: 0, // 从日志中获取
        initial_sqrt_price,
    }))
}

#[cfg(test)]
mod idl_layout_tests {
    use super::*;

    #[test]
    fn unified_dispatch_recognizes_idl_initialization_and_v2_liquidity() {
        for disc in [
            [95, 180, 10, 172, 84, 174, 232, 40],
            [207, 45, 87, 242, 27, 63, 204, 67],
            [133, 29, 89, 223, 69, 238, 176, 10],
            [58, 127, 188, 62, 79, 82, 196, 96],
        ] {
            assert!(crate::instr::instruction_data_may_parse(&PROGRAM_ID_PUBKEY, &disc));
        }
        assert!(!crate::instr::instruction_data_may_parse(
            &PROGRAM_ID_PUBKEY,
            &[17, 43, 80, 74, 168, 202, 6, 113]
        ));
    }

    #[test]
    fn initialize_versions_use_exact_wire_offsets_and_idl_accounts() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        for v2 in [false, true] {
            for bump in [0u8, 255] {
                let mut data = Vec::from(if v2 {
                    discriminators::INITIALIZE_POOL_V2
                } else {
                    discriminators::INITIALIZE_POOL
                });
                if !v2 {
                    data.push(bump);
                }
                data.extend_from_slice(&513u16.to_le_bytes());
                data.extend_from_slice(&u128::MAX.to_le_bytes());
                let DexEvent::OrcaWhirlpoolPoolInitialized(e) =
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap()
                else {
                    panic!("pool")
                };
                assert_eq!(e.whirlpool, accounts[if v2 { 6 } else { 4 }]);
                assert_eq!(e.whirlpools_config, accounts[0]);
                assert_eq!((e.token_mint_a, e.token_mint_b), (accounts[1], accounts[2]));
                assert_eq!(e.tick_spacing, 513);
                assert_eq!(e.initial_sqrt_price, u128::MAX);
                assert_eq!(e.token_program_a, accounts[if v2 { 10 } else { 8 }]);
                assert_eq!(e.token_program_b, accounts[if v2 { 11 } else { 8 }]);
                assert!(parse_instruction(
                    &data[..data.len() - 1],
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
    fn liquidity_versions_read_pool_and_position_from_idl_accounts() {
        for v2 in [false, true] {
            let mut accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
            if v2 {
                accounts[3] = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
            }
            for increase in [false, true] {
                let disc = match (v2, increase) {
                    (false, true) => discriminators::INCREASE_LIQUIDITY,
                    (true, true) => discriminators::INCREASE_LIQUIDITY_V2,
                    (false, false) => discriminators::DECREASE_LIQUIDITY,
                    (true, false) => discriminators::DECREASE_LIQUIDITY_V2,
                };
                let mut data = Vec::from(disc);
                data.extend_from_slice(&u128::MAX.to_le_bytes());
                data.extend_from_slice(&101u64.to_le_bytes());
                data.extend_from_slice(&202u64.to_le_bytes());
                if v2 {
                    data.push(0);
                }
                let event =
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap();
                let get = |i: usize| accounts.get(i).copied().unwrap_or_default();
                let (pool, position) = match event {
                    DexEvent::OrcaWhirlpoolLiquidityIncreased(mut e) => {
                        assert_eq!(e.position, accounts[if v2 { 5 } else { 3 }]);
                        e.position = Pubkey::default();
                        crate::core::account_fillers::orca::fill_whirlpool_liquidity_increased_accounts(&mut e, &get);
                        (e.whirlpool, e.position)
                    }
                    DexEvent::OrcaWhirlpoolLiquidityDecreased(mut e) => {
                        assert_eq!(e.position, accounts[if v2 { 5 } else { 3 }]);
                        e.position = Pubkey::default();
                        crate::core::account_fillers::orca::fill_whirlpool_liquidity_decreased_accounts(&mut e, &get);
                        (e.whirlpool, e.position)
                    }
                    _ => panic!("liquidity"),
                };
                assert_eq!(pool, accounts[0]);
                assert_eq!(position, accounts[if v2 { 5 } else { 3 }]);
                let required = if v2 { 6 } else { 4 };
                assert!(parse_instruction(
                    &data,
                    &accounts[..required - 1],
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
mod swap_wire_semantics_tests {
    use super::*;

    #[test]
    fn swap_wire_parameters_are_distinct_from_execution_for_both_layouts() {
        let mut accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        // Deliberately ambiguous legacy accounts: the discriminator must win.
        accounts[2] = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        for v2 in [false, true] {
            for input_mode in [false, true] {
                for direction in [false, true] {
                    for (amount, threshold, price) in [(0, u64::MAX, u128::MAX), (u64::MAX, 0, 0)] {
                        let mut data =
                            if v2 { discriminators::SWAP_V2 } else { discriminators::SWAP }
                                .to_vec();
                        data.extend_from_slice(&amount.to_le_bytes());
                        data.extend_from_slice(&threshold.to_le_bytes());
                        data.extend_from_slice(&price.to_le_bytes());
                        data.extend_from_slice(&[input_mode as u8, direction as u8]);
                        let DexEvent::OrcaWhirlpoolSwap(e) =
                            parse_instruction(&data, &accounts, Signature::default(), 1, 0, None)
                                .unwrap()
                        else {
                            panic!("swap")
                        };
                        assert_eq!(e.ix_name, if v2 { "swap_v2" } else { "swap" });
                        assert_eq!(
                            (e.amount, e.other_amount_threshold, e.sqrt_price_limit),
                            (amount, threshold, price)
                        );
                        assert_eq!(
                            (e.amount_specified_is_input, e.a_to_b),
                            (input_mode, direction)
                        );
                        assert_eq!(
                            (e.input_amount, e.output_amount, e.pre_sqrt_price, e.post_sqrt_price),
                            (0, 0, 0, 0)
                        );
                        assert_eq!(e.token_authority, accounts[if v2 { 3 } else { 1 }]);
                        assert_eq!(e.token_owner_account_a, accounts[if v2 { 7 } else { 3 }]);
                        assert_eq!(e.token_owner_account_b, accounts[if v2 { 9 } else { 5 }]);
                        assert_eq!(e.oracle, accounts[if v2 { 14 } else { 10 }]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod review_borsh_flag_regressions {
    use super::*;
    #[test]
    fn swap_versions_reject_noncanonical_direction_and_amount_flags() {
        let accounts: Vec<_> = (0..15).map(|_| Pubkey::new_unique()).collect();
        for disc in [[248,198,158,145,225,117,135,200], [43,4,237,11,26,201,30,98]] {
            let mut wire = disc.to_vec(); wire.extend_from_slice(&[0;35]);
            for offset in [40,41] {
                for value in 0..=u8::MAX {
                    wire[offset] = value;
                    assert_eq!(parse_instruction(&wire, &accounts, Signature::default(), 1, 0, None).is_some(), value <= 1, "offset={offset}, byte={value}");
                }
                wire[offset] = 0;
            }
        }
    }
}
