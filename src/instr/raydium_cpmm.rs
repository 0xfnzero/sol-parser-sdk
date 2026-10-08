//! Raydium CPMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium CPMM 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium CPMM discriminator 常量
pub mod discriminators {
    pub const COLLECT_CREATOR_FEE_PERMISSIONLESS: [u8; 8] = [202, 202, 34, 83, 226, 122, 145, 229];
    pub const COLLECT_CREATOR_FEE: [u8; 8] = [20, 22, 86, 123, 198, 28, 219, 132];
    pub const SWAP_BASE_IN: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
    pub const SWAP_BASE_OUT: [u8; 8] = [55, 217, 98, 86, 163, 74, 180, 173];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
    pub const WITHDRAW: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
}

/// Raydium CPMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_CPMM_PROGRAM_ID;

/// Check wire minimums and the exact event filter before resolving accounts.
/// Extra accounts/trailing bytes are allowed, matching the parser's semantics.
#[inline]
pub(crate) fn instruction_may_parse(
    data: &[u8],
    account_count: usize,
    filter: Option<&crate::grpc::types::EventTypeFilter>,
) -> bool {
    use crate::grpc::types::EventType;
    let Some(discriminator) = data.get(..8).and_then(|bytes| <[u8; 8]>::try_from(bytes).ok()) else {
        return false;
    };
    let (event_type, minimum_data, minimum_accounts) = match discriminator {
        discriminators::SWAP_BASE_IN | discriminators::SWAP_BASE_OUT =>
            (EventType::RaydiumCpmmSwap, 24, 13),
        discriminators::INITIALIZE => (EventType::RaydiumCpmmInitialize, 32, 20),
        discriminators::DEPOSIT => (EventType::RaydiumCpmmDeposit, 32, 13),
        discriminators::WITHDRAW => (EventType::RaydiumCpmmWithdraw, 32, 14),
        discriminators::COLLECT_CREATOR_FEE => (EventType::RaydiumCpmmCollectCreatorFee, 8, 15),
        discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS =>
            (EventType::RaydiumCpmmCollectCreatorFee, 8, 16),
        _ => return false,
    };
    data.len() >= minimum_data
        && account_count >= minimum_accounts
        && filter.is_none_or(|filter| filter.should_include(event_type))
}

/// 主要的 Raydium CPMM 指令解析函数
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
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::COLLECT_CREATOR_FEE
        | discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS => {
            parse_collect_creator_fee_instruction(
                accounts,
                discriminator == discriminators::COLLECT_CREATOR_FEE_PERMISSIONLESS,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::SWAP_BASE_IN => {
            parse_swap_base_in_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::SWAP_BASE_OUT => parse_swap_base_out_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::INITIALIZE => {
            parse_initialize_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::DEPOSIT => {
            parse_deposit_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::WITHDRAW => {
            parse_withdraw_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        _ => None,
    }
}

/// 解析 Base In 交换指令
fn parse_swap_base_in_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // Both swap layouts have 13 fixed accounts. Shred data is intent only,
    // but a truncated instruction must not become a complete trade hint.
    if accounts.len() < 13 {
        return None;
    }

    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_amount_out = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 3)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    let mut event = RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段 - 从日志填充，这里设置默认值
        pool_id: pool,
        input_vault_before: 0,  // Filled from the event log.
        output_vault_before: 0, // Filled from the event log.
        input_amount: 0,        // Filled from the event log.
        output_amount: 0,       // Filled from the event log.
        input_transfer_fee: 0,  // Filled from the event log.
        output_transfer_fee: 0, // Filled from the event log.
        base_input: true,
        ix_name: "swap_base_input".to_string(),
        amount_in,
        minimum_amount_out,
        // 暂时注释，以后会用，AI禁止改动
        // 指令参数字段
        // amount_in,
        // minimum_amount_out,

        // 指令账户字段 - 从account_filler填充
        // payer: Pubkey::default(),
        // authority: Pubkey::default(),
        // amm_config: Pubkey::default(),
        // pool_state: Pubkey::default(),
        // input_token_account: Pubkey::default(),
        // output_token_account: Pubkey::default(),
        // input_vault: Pubkey::default(),
        // output_vault: Pubkey::default(),
        // input_token_mint: Pubkey::default(),
        // output_token_mint: Pubkey::default(),
        ..Default::default()
    };
    crate::core::account_fillers::raydium::fill_cpmm_swap_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::RaydiumCpmmSwap(event))
}

/// 解析 Base Out 交换指令
fn parse_swap_base_out_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // Both swap layouts have 13 fixed accounts. Shred data is intent only,
    // but a truncated instruction must not become a complete trade hint.
    if accounts.len() < 13 {
        return None;
    }

    let mut offset = 0;

    let max_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 3)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    let mut event = RaydiumCpmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段 - 从日志填充，这里设置默认值
        pool_id: pool,
        input_vault_before: 0,  // Filled from the event log.
        output_vault_before: 0, // Filled from the event log.
        input_amount: 0,        // Filled from the event log.
        output_amount: 0,       // Filled from the event log.
        input_transfer_fee: 0,  // Filled from the event log.
        output_transfer_fee: 0, // Filled from the event log.
        base_input: false,
        ix_name: "swap_base_output".to_string(),
        max_amount_in,
        amount_out,
        // 暂时注释，以后会用，AI禁止改动
        // 指令参数字段
        // amount_in: maximum_amount_in,
        // minimum_amount_out: amount_out,

        // 指令账户字段 - 从account_filler填充
        // payer: Pubkey::default(),
        // authority: Pubkey::default(),
        // amm_config: Pubkey::default(),
        // pool_state: Pubkey::default(),
        // input_token_account: Pubkey::default(),
        // output_token_account: Pubkey::default(),
        // input_vault: Pubkey::default(),
        // output_vault: Pubkey::default(),
        // input_token_mint: Pubkey::default(),
        // output_token_mint: Pubkey::default(),
        ..Default::default()
    };
    crate::core::account_fillers::raydium::fill_cpmm_swap_accounts(&mut event, &|i| {
        accounts.get(i).copied().unwrap_or_default()
    });
    Some(DexEvent::RaydiumCpmmSwap(event))
}

/// 解析初始化指令
fn parse_initialize_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // IDL fixed accounts must be complete before constructing an intent event.
    if accounts.len() < 20 {
        return None;
    }

    let mut offset = 0;

    let init_amount0 = read_u64_le(data, offset)?;
    offset += 8;

    let init_amount1 = read_u64_le(data, offset)?;
    offset += 8;

    let _open_time = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 3)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent {
        metadata,
        pool,
        creator: get_account(accounts, 0).unwrap_or_default(),
        init_amount0,
        init_amount1,
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
    // IDL fixed accounts must be complete before constructing an intent event.
    if accounts.len() < 13 {
        return None;
    }

    let mut offset = 0;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let maximum_token_1_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
        metadata,
        pool,
        user: get_account(accounts, 0).unwrap_or_default(),
        lp_token_amount,
        token0_amount: maximum_token_0_amount, // Instruction maximum, not an executed LpChangeEvent amount.
        token1_amount: maximum_token_1_amount, // Instruction maximum, not an executed LpChangeEvent amount.
    }))
}

/// 解析提款指令
fn parse_withdraw_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // IDL fixed accounts must be complete before constructing an intent event.
    if accounts.len() < 14 {
        return None;
    }

    let mut offset = 0;

    let lp_token_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_0_amount = read_u64_le(data, offset)?;
    offset += 8;

    let minimum_token_1_amount = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
        metadata,
        pool,
        user: get_account(accounts, 0).unwrap_or_default(),
        lp_token_amount,
        token0_amount: minimum_token_0_amount, // Instruction minimum, not an executed LpChangeEvent amount.
        token1_amount: minimum_token_1_amount, // Instruction minimum, not an executed LpChangeEvent amount.
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_preserves_parser_and_event_filter_semantics() {
        use crate::grpc::types::{EventType, EventTypeFilter};
        let idl: serde_json::Value = serde_json::from_str(include_str!("../../idl/raydium_cpmm.json")).unwrap();
        let accounts: Vec<_> = (0..65).map(|_| Pubkey::new_unique()).collect();
        let filters = [
            None,
            Some(EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap])),
            Some(EventTypeFilter::exclude_types(vec![EventType::RaydiumCpmmCollectCreatorFee])),
        ];
        let mut checked_instructions = 0;
        for instruction in idl["instructions"].as_array().unwrap() {
            let disc: Vec<u8> = instruction["discriminator"].as_array().unwrap().iter()
                .map(|v| v.as_u64().unwrap() as u8).collect();
            // This preflight only covers instruction types emitted by this parser.
            if !instruction_may_parse(&[disc.as_slice(), &[1; 24]].concat(), 65, None) {
                continue;
            }
            checked_instructions += 1;
            let required_accounts = instruction["accounts"].as_array().unwrap().len();
            let required_data = 8 + 8 * instruction["args"].as_array().unwrap().len();
            for data_len in [7, required_data - 1, required_data, required_data + 1] {
                let mut data = disc.clone();
                data.resize(data_len, 1);
                for count in [required_accounts - 1, required_accounts, 65] {
                    for filter in &filters {
                        let event = parse_instruction(&data, &accounts[..count], Signature::default(), 1, 0, None);
                        let expected = event.as_ref().is_some_and(|event| filter.as_ref().is_none_or(|f| f.should_include_dex_event(event)));
                        assert_eq!(instruction_may_parse(&data, count, filter.as_ref()), expected,
                            "{} data={data_len} accounts={count}", instruction["name"]);
                    }
                }
            }
        }
        assert_eq!(checked_instructions, 7);
    }

    #[test]
    fn swap_retains_wire_limits_separately_from_execution_amounts() {
        let accounts: Vec<_> = (0..13).map(|_| Pubkey::new_unique()).collect();
        for base_input in [false, true] {
            for (first, second) in [(0u64, u64::MAX), (101, 202)] {
                let mut data = Vec::from(if base_input {
                    discriminators::SWAP_BASE_IN
                } else {
                    discriminators::SWAP_BASE_OUT
                });
                data.extend_from_slice(&first.to_le_bytes());
                data.extend_from_slice(&second.to_le_bytes());
                let DexEvent::RaydiumCpmmSwap(e) =
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap()
                else {
                    panic!("swap")
                };
                assert_eq!(e.base_input, base_input);
                assert_eq!(e.payer, accounts[0]);
                assert_eq!(e.authority, accounts[1]);
                assert_eq!(e.input_token_account, accounts[4]);
                assert_eq!(e.output_token_account, accounts[5]);
                assert_eq!((e.input_amount, e.output_amount), (0, 0));
                if base_input {
                    assert_eq!(e.ix_name, "swap_base_input");
                    assert_eq!((e.amount_in, e.minimum_amount_out), (first, second));
                    assert_eq!((e.max_amount_in, e.amount_out), (0, 0));
                } else {
                    assert_eq!(e.ix_name, "swap_base_output");
                    assert_eq!((e.max_amount_in, e.amount_out), (first, second));
                    assert_eq!((e.amount_in, e.minimum_amount_out), (0, 0));
                }
                let mut legacy = serde_json::to_value(&e).unwrap();
                for field in [
                    "ix_name",
                    "amount_in",
                    "minimum_amount_out",
                    "max_amount_in",
                    "amount_out",
                    "payer",
                    "authority",
                    "input_token_account",
                    "output_token_account",
                ] {
                    legacy.as_object_mut().unwrap().remove(field);
                }
                let restored: RaydiumCpmmSwapEvent = serde_json::from_value(legacy).unwrap();
                assert!(restored.ix_name.is_empty());
                assert_eq!(restored.payer, Pubkey::default());
                assert_eq!(restored.amount_out, 0);
            }
        }
    }

    #[test]
    fn liquidity_and_initialize_use_idl_pool_and_owner_accounts() {
        let accounts: Vec<_> = (0..20).map(|_| Pubkey::new_unique()).collect();
        for (disc, pool_index) in [
            (discriminators::INITIALIZE, 3),
            (discriminators::DEPOSIT, 2),
            (discriminators::WITHDRAW, 2),
        ] {
            let mut data = Vec::from(disc);
            for amount in [101u64, 202, 303] {
                data.extend_from_slice(&amount.to_le_bytes());
            }
            let event =
                parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap();
            let (pool, owner) = match event {
                DexEvent::RaydiumCpmmInitialize(e) => (e.pool, e.creator),
                DexEvent::RaydiumCpmmDeposit(e) => (e.pool, e.user),
                DexEvent::RaydiumCpmmWithdraw(e) => (e.pool, e.user),
                _ => panic!("liquidity event"),
            };
            assert_eq!(pool, accounts[pool_index]);
            assert_eq!(owner, accounts[0]);
            for length in 0..=pool_index {
                assert!(parse_instruction(
                    &data,
                    &accounts[..length],
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

fn parse_collect_creator_fee_instruction(
    accounts: &[Pubkey],
    permissionless: bool,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if accounts.len() < if permissionless { 16 } else { 15 } {
        return None;
    }
    let creator = accounts[usize::from(permissionless)];
    let pool_state = accounts[if permissionless { 3 } else { 2 }];
    Some(DexEvent::RaydiumCpmmCollectCreatorFee(RaydiumCpmmCollectCreatorFeeEvent {
        metadata: create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state),
        permissionless,
        payer: accounts[0],
        creator,
        authority: accounts[if permissionless { 2 } else { 1 }],
        pool_state,
        amm_config: accounts[if permissionless { 14 } else { 3 }],
        token_0_vault: accounts[4],
        token_1_vault: accounts[5],
        vault_0_mint: accounts[6],
        vault_1_mint: accounts[7],
        creator_token_0: accounts[8],
        creator_token_1: accounts[9],
        token_0_program: accounts[10],
        token_1_program: accounts[11],
        associated_token_program: accounts[12],
        system_program: accounts[13],
        creator_fee_share: accounts[if permissionless { 15 } else { 14 }],
    }))
}
