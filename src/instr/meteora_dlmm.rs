//! Meteora DLMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora DLMM 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

pub mod discriminators {
    pub const ADD_LIQUIDITY: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
    pub const ADD_LIQUIDITY2: [u8; 8] = [228, 162, 78, 28, 70, 219, 116, 115];
    pub const CLAIM_FEE: [u8; 8] = [169, 32, 79, 137, 136, 232, 70, 137];
    pub const CLAIM_FEE2: [u8; 8] = [112, 191, 101, 171, 28, 144, 127, 187];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    pub const CLOSE_POSITION2: [u8; 8] = [174, 90, 35, 115, 186, 40, 147, 226];
    pub const INITIALIZE_BIN_ARRAY: [u8; 8] = [35, 86, 19, 185, 78, 212, 75, 211];
    pub const INITIALIZE_LB_PAIR: [u8; 8] = [45, 154, 237, 210, 221, 15, 166, 92];
    pub const INITIALIZE_LB_PAIR2: [u8; 8] = [73, 59, 36, 120, 237, 83, 108, 198];
    pub const INITIALIZE_POSITION: [u8; 8] = [219, 192, 234, 71, 190, 191, 102, 80];
    pub const INITIALIZE_POSITION2: [u8; 8] = [143, 19, 242, 145, 213, 15, 104, 115];
    pub const INITIALIZE_POSITION_PDA: [u8; 8] = [46, 82, 125, 146, 85, 141, 228, 153];
    pub const REMOVE_LIQUIDITY: [u8; 8] = [80, 85, 209, 72, 24, 206, 177, 108];
    pub const REMOVE_LIQUIDITY2: [u8; 8] = [230, 215, 82, 127, 241, 101, 227, 146];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const SWAP2: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];
    pub const SWAP_EXACT_OUT: [u8; 8] = [250, 73, 101, 33, 38, 207, 75, 184];
    pub const SWAP_EXACT_OUT2: [u8; 8] = [43, 215, 247, 132, 137, 60, 243, 81];
    pub const SWAP_WITH_PRICE_IMPACT: [u8; 8] = [56, 173, 230, 208, 173, 228, 156, 205];
    pub const SWAP_WITH_PRICE_IMPACT2: [u8; 8] = [74, 98, 192, 214, 177, 51, 75, 51];
}

/// Meteora DLMM 程序 ID (使用常量)
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::METEORA_DLMM_PROGRAM_ID;

/// 主要的 Meteora DLMM 指令解析函数
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

    let discriminator: [u8; 8] = instruction_data[..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::INITIALIZE_LB_PAIR => parse_initialize_lb_pair_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::INITIALIZE_LB_PAIR2 => parse_initialize_lb_pair2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::INITIALIZE_BIN_ARRAY => parse_initialize_bin_array_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::ADD_LIQUIDITY => {
            parse_add_liquidity_instruction(accounts, 11, signature, slot, tx_index, block_time_us)
        }
        discriminators::ADD_LIQUIDITY2 => {
            parse_add_liquidity_instruction(accounts, 9, signature, slot, tx_index, block_time_us)
        }
        discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity_instruction(
            accounts,
            11,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::REMOVE_LIQUIDITY2 => parse_remove_liquidity_instruction(
            accounts,
            9,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::INITIALIZE_POSITION | discriminators::INITIALIZE_POSITION2 => {
            parse_initialize_position_instruction(
                data,
                accounts,
                1,
                2,
                3,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::INITIALIZE_POSITION_PDA => parse_initialize_position_instruction(
            data,
            accounts,
            2,
            3,
            4,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::SWAP | discriminators::SWAP2 => {
            parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::SWAP_EXACT_OUT | discriminators::SWAP_EXACT_OUT2 => {
            parse_swap_exact_out_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::SWAP_WITH_PRICE_IMPACT | discriminators::SWAP_WITH_PRICE_IMPACT2 => {
            parse_swap_with_price_impact_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::CLAIM_FEE => {
            parse_claim_fee_instruction(accounts, 4, signature, slot, tx_index, block_time_us)
        }
        discriminators::CLAIM_FEE2 => {
            parse_claim_fee_instruction(accounts, 2, signature, slot, tx_index, block_time_us)
        }
        discriminators::CLOSE_POSITION => parse_close_position_instruction(
            accounts,
            Some(1),
            4,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::CLOSE_POSITION2 => parse_close_position_instruction(
            accounts,
            None,
            1,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        _ => None,
    }
}

/// Parse an `initialize_lb_pair2` instruction.
fn parse_initialize_lb_pair2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let active_id = read_i32_le(data, 0)?;
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
        metadata,
        pool,
        creator: get_account(accounts, 8).unwrap_or_default(),
        active_bin_id: active_id,
        bin_step: 0,
    }))
}

/// 解析初始化LB池指令
fn parse_initialize_lb_pair_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let active_id = read_i32_le(data, offset)?;
    offset += 4;

    let bin_step = read_u16_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
        metadata,
        pool,
        creator: get_account(accounts, 8).unwrap_or_default(),
        active_bin_id: active_id,
        bin_step,
    }))
}

/// 解析初始化Bin数组指令
fn parse_initialize_bin_array_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let index = read_i64_le(data, 0)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmInitializeBinArray(MeteoraDlmmInitializeBinArrayEvent {
        metadata,
        pool,
        bin_array: get_account(accounts, 1).unwrap_or_default(),
        index,
    }))
}

/// 解析添加流动性指令
fn parse_add_liquidity_instruction(
    accounts: &[Pubkey],
    sender_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
        metadata,
        pool,
        from: get_account(accounts, sender_index)?,
        position: get_account(accounts, 0).unwrap_or_default(),
        amounts: [0, 0],
        active_bin_id: 0,
    }))
}

/// 解析移除流动性指令
fn parse_remove_liquidity_instruction(
    accounts: &[Pubkey],
    sender_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 1)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
        metadata,
        pool,
        from: get_account(accounts, sender_index)?,
        position: get_account(accounts, 0).unwrap_or_default(),
        amounts: [0, 0],
        active_bin_id: 0,
    }))
}

/// 解析初始化头寸指令
fn parse_initialize_position_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    position_index: usize,
    pool_index: usize,
    owner_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let lower_bin_id = read_i32_le(data, offset)?;
    offset += 4;

    let width = u32::try_from(read_i32_le(data, offset)?).ok()?;

    let pool = get_account(accounts, pool_index)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
        metadata,
        pool,
        position: get_account(accounts, position_index)?,
        owner: get_account(accounts, owner_index)?,
        lower_bin_id,
        width,
    }))
}

/// 解析交换指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if data.len() < 16 {
        return None;
    }
    let amount_in = read_u64_le(data, 0)?;
    let amount_out = read_u64_le(data, 8).unwrap_or(0);

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        token_x_mint: Pubkey::default(),
        token_y_mint: Pubkey::default(),
        pool,
        from: get_account(accounts, 10).unwrap_or_default(),
        start_bin_id: 0,
        end_bin_id: 0,
        amount_in,
        amount_out,
        swap_for_y: false,
        fee: 0,
        protocol_fee: 0,
        fee_bps: 0,
        host_fee: 0,
    }))
}

fn parse_swap_exact_out_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let max_in_amount = read_u64_le(data, 0)?;
    let out_amount = read_u64_le(data, 8)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        token_x_mint: Pubkey::default(),
        token_y_mint: Pubkey::default(),
        pool,
        from: get_account(accounts, 10).unwrap_or_default(),
        start_bin_id: 0,
        end_bin_id: 0,
        amount_in: max_in_amount,
        amount_out: out_amount,
        swap_for_y: false,
        fee: 0,
        protocol_fee: 0,
        fee_bps: 0,
        host_fee: 0,
    }))
}

fn parse_swap_with_price_impact_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let option_size = match data.get(8).copied()? {
        0 => 1,
        1 => 5,
        _ => return None,
    };
    if data.len() < 8 + option_size + 2 {
        return None;
    }
    let amount_in = read_u64_le(data, 0)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        metadata,
        token_x_mint: Pubkey::default(),
        token_y_mint: Pubkey::default(),
        pool,
        from: get_account(accounts, 10).unwrap_or_default(),
        start_bin_id: 0,
        end_bin_id: 0,
        amount_in,
        amount_out: 0,
        swap_for_y: false,
        fee: 0,
        protocol_fee: 0,
        fee_bps: 0,
        host_fee: 0,
    }))
}

/// 解析费用领取指令
fn parse_claim_fee_instruction(
    accounts: &[Pubkey],
    owner_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent {
        metadata,
        pool,
        position: get_account(accounts, 1).unwrap_or_default(),
        owner: get_account(accounts, owner_index)?,
        fee_x: 0,
        fee_y: 0,
    }))
}

/// 解析关闭头寸指令
fn parse_close_position_instruction(
    accounts: &[Pubkey],
    pool_index: Option<usize>,
    owner_index: usize,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let position = get_account(accounts, 0)?;
    let pool = pool_index.and_then(|index| get_account(accounts, index)).unwrap_or_default();
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
        metadata,
        pool,
        position,
        owner: get_account(accounts, owner_index)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accounts(count: usize) -> Vec<Pubkey> {
        (0..count).map(|_| Pubkey::new_unique()).collect()
    }

    fn instruction(discriminator: [u8; 8], payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(8 + payload.len());
        data.extend_from_slice(&discriminator);
        data.extend_from_slice(payload);
        data
    }

    #[test]
    fn versioned_account_layouts_use_idl_indices() {
        let accounts = accounts(16);
        let event = parse_instruction(
            &instruction(discriminators::ADD_LIQUIDITY2, &[]),
            &accounts,
            Signature::default(),
            1,
            0,
            None,
        )
        .expect("add_liquidity2");
        let DexEvent::MeteoraDlmmAddLiquidity(event) = event else {
            panic!("unexpected event");
        };
        assert_eq!(event.position, accounts[0]);
        assert_eq!(event.pool, accounts[1]);
        assert_eq!(event.from, accounts[9]);

        let mut payload = Vec::new();
        payload.extend_from_slice(&(-5i32).to_le_bytes());
        payload.extend_from_slice(&10i32.to_le_bytes());
        let event = parse_instruction(
            &instruction(discriminators::INITIALIZE_POSITION_PDA, &payload),
            &accounts,
            Signature::default(),
            1,
            0,
            None,
        )
        .expect("initialize_position_pda");
        let DexEvent::MeteoraDlmmCreatePosition(event) = event else {
            panic!("unexpected event");
        };
        assert_eq!(event.position, accounts[2]);
        assert_eq!(event.pool, accounts[3]);
        assert_eq!(event.owner, accounts[4]);
    }

    #[test]
    fn close_position2_does_not_treat_rent_receiver_as_owner() {
        let accounts = accounts(5);
        let event = parse_instruction(
            &instruction(discriminators::CLOSE_POSITION2, &[]),
            &accounts,
            Signature::default(),
            1,
            0,
            None,
        )
        .expect("close_position2");
        let DexEvent::MeteoraDlmmClosePosition(event) = event else {
            panic!("unexpected event");
        };
        assert_eq!(event.position, accounts[0]);
        assert_eq!(event.owner, accounts[1]);
        assert_eq!(event.pool, Pubkey::default());
    }
}
