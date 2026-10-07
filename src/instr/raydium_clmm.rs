//! Raydium CLMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium CLMM 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium CLMM discriminator 常量
/// 参考: solana-streamer/src/streaming/event_parser/protocols/raydium_clmm/events.rs
pub mod discriminators {
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const INCREASE_LIQUIDITY_V2: [u8; 8] = [133, 29, 89, 223, 69, 238, 176, 10];
    pub const DECREASE_LIQUIDITY_V2: [u8; 8] = [58, 127, 188, 62, 79, 82, 196, 96]; // ✅ 修复：使用 V2 discriminator
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
    pub const CREATE_CUSTOMIZABLE_POOL: [u8; 8] = [43, 68, 212, 167, 89, 47, 164, 1];
    pub const OPEN_POSITION: [u8; 8] = [135, 128, 47, 77, 15, 152, 240, 49];
    pub const OPEN_POSITION_V2: [u8; 8] = [77, 184, 74, 214, 112, 86, 241, 199];
    pub const OPEN_POSITION_WITH_TOKEN_22_NFT: [u8; 8] = [77, 255, 174, 82, 125, 29, 201, 46];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
}

/// Raydium CLMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_CLMM_PROGRAM_ID;

/// 主要的 Raydium CLMM 指令解析函数
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
        discriminators::SWAP => {
            parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us, "swap")
        }
        discriminators::SWAP_V2 => {
            parse_swap_v2_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::INCREASE_LIQUIDITY_V2 => parse_increase_liquidity_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::DECREASE_LIQUIDITY_V2 => parse_decrease_liquidity_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::CREATE_POOL => {
            parse_create_pool_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::CREATE_CUSTOMIZABLE_POOL => parse_create_customizable_pool_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::OPEN_POSITION => parse_open_position_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            5,
        ),
        discriminators::OPEN_POSITION_V2 => parse_open_position_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::OPEN_POSITION_WITH_TOKEN_22_NFT => {
            parse_open_position_with_token_22_nft_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::CLOSE_POSITION => parse_close_position_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        _ => None,
    }
}

/// 解析交换指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    ix_name: &str,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount = read_u64_le(data, offset)?;
    offset += 8;

    let other_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let sqrt_price_limit_x64 = read_u128_le(data, offset)?;
    offset += 16;

    // Instruction data bool is `is_base_input` (exact-in vs exact-out), NOT
    // swap direction. Real `zero_for_one` comes from the SwapEvent log; leave
    // a placeholder here so log-preferred merge keeps the log value.
    let is_base_input = data.get(offset)? == &1;

    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    let mut event = RaydiumClmmSwapEvent {
        metadata,
        pool_state: pool,
        sender: get_account(accounts, 0).unwrap_or_default(),
        token_account_0: get_account(accounts, 3).unwrap_or_default(),
        token_account_1: get_account(accounts, 4).unwrap_or_default(),
        amount_0: 0,
        transfer_fee_0: 0,
        amount_1: 0,
        transfer_fee_1: 0,
        zero_for_one: false,
        sqrt_price_x64: 0,
        ix_name: ix_name.to_string(),
        amount,
        other_amount_threshold,
        sqrt_price_limit_x64,
        is_base_input,
        liquidity: 0,
        tick: 0,
        ..Default::default()
    };
    crate::core::account_fillers::raydium::fill_clmm_swap_accounts_with_count(
        &mut event,
        &|i| accounts.get(i).copied().unwrap_or_default(),
        accounts.len(),
    );
    Some(DexEvent::RaydiumClmmSwap(event))
}

/// 解析 Swap V2 指令（支持 Token2022）
fn parse_swap_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // SwapV2 与 Swap 参数相同，只是支持 Token2022
    parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us, "swap_v2")
}

/// 解析增加流动性 V2 指令
fn parse_increase_liquidity_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount_0_max = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_max = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        position_nft_mint: get_account(accounts, 1).unwrap_or_default(),
        liquidity,
        amount_0: 0,
        amount_1: 0,
        amount_0_transfer_fee: 0,
        amount_1_transfer_fee: 0,
        pool,
        amount0_max: amount_0_max,
        amount1_max: amount_1_max,
        user: get_account(accounts, 0).unwrap_or_default(),
    }))
}

/// 解析减少流动性 V2 指令
fn parse_decrease_liquidity_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount_0_min = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_min = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 3)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        position_nft_mint: get_account(accounts, 1).unwrap_or_default(),
        liquidity,
        decrease_amount_0: 0,
        decrease_amount_1: 0,
        fee_amount_0: 0,
        fee_amount_1: 0,
        reward_amounts: [0; 3],
        transfer_fee_0: 0,
        transfer_fee_1: 0,
        pool,
        amount0_min: amount_0_min,
        amount1_min: amount_1_min,
        user: get_account(accounts, 0).unwrap_or_default(),
    }))
}

/// 解析池创建指令
fn parse_create_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let open_time = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool,
        token_0_mint: get_account(accounts, 3).unwrap_or_default(),
        token_1_mint: get_account(accounts, 4).unwrap_or_default(),
        tick_spacing: 0, // 从主指令解析
        fee_rate: 0,     // 从主指令解析
        sqrt_price_x64,
        tick: 0,
        token_vault_0: get_account(accounts, 5).unwrap_or_default(),
        token_vault_1: get_account(accounts, 6).unwrap_or_default(),
        creator: get_account(accounts, 0).unwrap_or_default(),
        open_time,
    }))
}

/// 解析可定制池创建指令（dynamic fee / single-sided fee opt-in）
fn parse_create_customizable_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let sqrt_price_x64 = read_u128_le(data, 0)?;
    let pool = get_account(accounts, 2)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool,
        token_0_mint: get_account(accounts, 3).unwrap_or_default(),
        token_1_mint: get_account(accounts, 4).unwrap_or_default(),
        tick_spacing: 0,
        fee_rate: 0,
        sqrt_price_x64,
        tick: 0,
        token_vault_0: get_account(accounts, 5).unwrap_or_default(),
        token_vault_1: get_account(accounts, 6).unwrap_or_default(),
        creator: get_account(accounts, 0).unwrap_or_default(),
        open_time: 0,
    }))
}

/// 解析开启头寸指令
fn parse_open_position_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    pool_account_index: usize,
) -> Option<DexEvent> {
    let mut offset = 0;

    let tick_lower_index = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper_index = read_i32_le(data, offset)?;
    offset += 4;

    let _tick_array_lower_start_index = read_i32_le(data, offset)?;
    offset += 4;

    let _tick_array_upper_start_index = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let _amount_0_max = read_u64_le(data, offset)?;
    offset += 8;

    let _amount_1_max = read_u64_le(data, offset)?;

    let pool = get_account(accounts, pool_account_index)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        position_nft_mint: get_account(accounts, 2).unwrap_or_default(),
        tick_lower_index,
        tick_upper_index,
        liquidity,
    }))
}

/// 解析关闭头寸指令
fn parse_close_position_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, Pubkey::default());

    Some(DexEvent::RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent {
        metadata,
        pool: Pubkey::default(),
        user: get_account(accounts, 0).unwrap_or_default(),
        position_nft_mint: get_account(accounts, 1).unwrap_or_default(),
    }))
}
/// 解析打开仓位 V2 指令
fn parse_open_position_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    parse_open_position_instruction(data, accounts, signature, slot, tx_index, block_time_us, 5)
}

/// 解析打开仓位（Token22 NFT）指令
fn parse_open_position_with_token_22_nft_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    parse_open_position_instruction(data, accounts, signature, slot, tx_index, block_time_us, 4)
}

#[cfg(test)]
mod swap_wire_tests {
    use super::*;

    #[test]
    fn swap_limits_and_mode_are_separate_from_execution_and_layout_uses_discriminator() {
        for v2 in [false, true] {
            for input_mode in [false, true] {
                let mut accounts: Vec<_> = (0..17).map(|_| Pubkey::new_unique()).collect();
                // Deliberately invert the memo heuristic: layout follows discriminator.
                if !v2 {
                    accounts[10] = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".parse().unwrap();
                }
                let mut data =
                    Vec::from(if v2 { discriminators::SWAP_V2 } else { discriminators::SWAP });
                data.extend_from_slice(&0u64.to_le_bytes());
                data.extend_from_slice(&u64::MAX.to_le_bytes());
                data.extend_from_slice(&u128::MAX.to_le_bytes());
                data.push(u8::from(input_mode));
                let DexEvent::RaydiumClmmSwap(e) =
                    parse_instruction(&data, &accounts, Signature::default(), 1, 0, None).unwrap()
                else {
                    panic!("swap")
                };
                assert_eq!(e.ix_name, if v2 { "swap_v2" } else { "swap" });
                assert_eq!(e.amount, 0);
                assert_eq!(e.other_amount_threshold, u64::MAX);
                assert_eq!(e.sqrt_price_limit_x64, u128::MAX);
                assert_eq!(e.is_base_input, input_mode);
                assert_eq!((e.amount_0, e.amount_1, e.sqrt_price_x64), (0, 0, 0));
                assert!(!e.zero_for_one);
                assert_eq!(e.tick_arrays, accounts[if v2 { 13 } else { 9 }..].to_vec());
                assert_eq!(e.input_mint, if v2 { accounts[11] } else { Pubkey::default() });
                assert_eq!(e.output_mint, if v2 { accounts[12] } else { Pubkey::default() });
                let serialized = serde_json::to_string(&e).unwrap();
                let roundtrip: RaydiumClmmSwapEvent = serde_json::from_str(&serialized).unwrap();
                assert_eq!(roundtrip.sqrt_price_limit_x64, u128::MAX);
                // Value's default numeric representation is limited to u64;
                // legacy JSON omits the new limit field entirely.
                let mut legacy_event = e.clone();
                legacy_event.sqrt_price_limit_x64 = 0;
                let mut old_json = serde_json::to_value(&legacy_event).unwrap();
                for name in [
                    "ix_name",
                    "amount",
                    "other_amount_threshold",
                    "sqrt_price_limit_x64",
                    "is_base_input",
                ] {
                    old_json.as_object_mut().unwrap().remove(name);
                }
                let old: RaydiumClmmSwapEvent = serde_json::from_value(old_json).unwrap();
                assert!(old.ix_name.is_empty());
                assert_eq!(old.sqrt_price_limit_x64, 0);
            }
        }
    }
}
