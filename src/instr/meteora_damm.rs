//! Meteora DAMM V2 指令解析器
//!
//! 使用 match discriminator 模式解析 Meteora DAMM V2 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Meteora DAMM V2 discriminator 常量
pub mod discriminators {
    pub const SWAP_LOG: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const SWAP2_LOG: [u8; 8] = [189, 66, 51, 168, 38, 80, 117, 153];
    pub const CREATE_POSITION_LOG: [u8; 8] = [156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION_LOG: [u8; 8] = [20, 145, 144, 68, 143, 142, 214, 178];
    pub const ADD_LIQUIDITY_LOG: [u8; 8] = [175, 242, 8, 157, 30, 247, 185, 169];
    pub const REMOVE_LIQUIDITY_LOG: [u8; 8] = [87, 46, 88, 98, 175, 96, 34, 91];
    pub const INITIALIZE_POOL: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];
}

/// Meteora DAMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::METEORA_DAMM_V2_PROGRAM_ID;

/// 主要的 Meteora DAMM V2 指令解析函数
#[allow(unused_variables)]
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    if discriminator == discriminators::INITIALIZE_POOL {
        return parse_initialize_pool_outer_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    if instruction_data.len() < 16 {
        return None;
    }

    let cpi_discriminator: [u8; 8] = instruction_data[8..16].try_into().ok()?;
    let cpi_data = &instruction_data[16..];

    match cpi_discriminator {
        discriminators::SWAP_LOG => parse_swap_log_instruction(
            cpi_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::SWAP2_LOG => parse_swap2_log_instruction(
            cpi_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CREATE_POSITION_LOG => parse_create_position_log_instruction(
            cpi_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CLOSE_POSITION_LOG => parse_close_position_log_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::ADD_LIQUIDITY_LOG => parse_add_liquidity_log_instruction(
            cpi_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::REMOVE_LIQUIDITY_LOG => parse_remove_liquidity_log_instruction(
            cpi_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        _ => None,
    }
}

/// 解析外层 initialize_pool 指令
#[allow(unused_variables)]
fn parse_initialize_pool_outer_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if data.len() < 33 || accounts.len() < 10 {
        return None;
    }

    let liquidity = read_u128_le(data, 0)?;
    let sqrt_price = read_u128_le(data, 16)?;
    let activation_point = match read_u8(data, 32)? {
        0 => None,
        1 => Some(read_u64_le(data, 33)?),
        _ => return None,
    };

    let pool = accounts.get(6).copied().unwrap_or_default();
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent {
        metadata,
        creator: accounts.first().copied().unwrap_or_default(),
        position_nft_mint: accounts.get(1).copied().unwrap_or_default(),
        pool,
        position: accounts.get(7).copied().unwrap_or_default(),
        token_a_mint: accounts.get(8).copied().unwrap_or_default(),
        token_b_mint: accounts.get(9).copied().unwrap_or_default(),
        liquidity,
        sqrt_price,
        activation_point,
        ..Default::default()
    }))
}

/// 解析 Swap 指令
#[allow(unused_variables)]
fn parse_swap_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // tradeDirection (u8 - 1 byte)
    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    // hasReferral (bool - 1 byte)
    let has_referral = read_bool(data, offset)?;
    offset += 1;

    // params.amountIn (u64 - 8 bytes)
    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    // params.minimumAmountOut (u64 - 8 bytes)
    let min_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.outputAmount (u64 - 8 bytes)
    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.nextSqrtPrice (u128 - 16 bytes)
    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    // swapResult.lpFee (u64 - 8 bytes)
    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.protocolFee (u64 - 8 bytes)
    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.partnerFee (u64 - 8 bytes)
    let partner_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.referralFee (u64 - 8 bytes)
    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    // actualAmountIn (u64 - 8 bytes)
    let actual_amount_in = read_u64_le(data, offset)?;
    offset += 8;

    // currentTimestamp (u64 - 8 bytes)
    let current_timestamp = read_u64_le(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out: min_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee,
        referral_fee,
        actual_amount_in,
        current_timestamp,
        ..Default::default()
    }))
}

/// 解析 Swap2 指令 (EvtSwap2 格式)
#[allow(unused_variables)]
fn parse_swap2_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // trade_direction (u8 - 1 byte)
    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    // collect_fee_mode (u8 - 1 byte)
    let _collect_fee_mode = read_u8(data, offset)?;
    offset += 1;

    // has_referral (bool - 1 byte)
    let has_referral = read_bool(data, offset)?;
    offset += 1;

    // SwapParameters2
    // params.amount_0 (u64 - 8 bytes)
    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    // params.amount_1 (u64 - 8 bytes)
    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    // params.swap_mode (u8 - 1 byte)
    let swap_mode = read_u8(data, offset)?;
    offset += 1;

    // SwapResult2
    // swapResult.included_fee_input_amount (u64 - 8 bytes)
    let included_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.excluded_fee_input_amount (u64 - 8 bytes)
    let _excluded_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.amount_left (u64 - 8 bytes)
    let _amount_left = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.output_amount (u64 - 8 bytes)
    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.next_sqrt_price (u128 - 16 bytes)
    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    // swapResult.trading_fee (u64 - 8 bytes)
    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.protocol_fee (u64 - 8 bytes)
    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.referral_fee (u64 - 8 bytes)
    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    // quote_reserve_amount (u64 - 8 bytes)
    let _quote_reserve_amount = read_u64_le(data, offset)?;
    offset += 8;

    // migration_threshold (u64 - 8 bytes)
    let _migration_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // currentTimestamp (u64 - 8 bytes)
    let current_timestamp = read_u64_le(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    // 根据 swap_mode 和 trade_direction 确定实际的 amount_in
    // swap_mode: 0 = ExactIn, 1 = ExactOut
    let (amount_in, minimum_amount_out) = if swap_mode == 0 {
        // ExactIn: amount_0 is amount_in, amount_1 is minimum_amount_out
        (amount_0, amount_1)
    } else {
        // ExactOut: amount_1 is maximum_amount_in, amount_0 is amount_out
        (amount_1, amount_0)
    };

    let actual_amount_in = included_fee_input_amount;

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee: 0, // SwapResult2 没有 partner_fee
        referral_fee,
        actual_amount_in,
        current_timestamp,
        ..Default::default()
    }))
}

/// 解析 Create Position Log 指令
#[allow(unused_variables)]
fn parse_create_position_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // owner (Pubkey - 32 bytes)
    let owner = read_pubkey(data, offset)?;
    offset += 32;

    // position (Pubkey - 32 bytes)
    let position = read_pubkey(data, offset)?;
    offset += 32;

    // positionNftMint (Pubkey - 32 bytes)
    let position_nft_mint = read_pubkey(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
        metadata,
        pool,
        owner,
        position,
        position_nft_mint,
    }))
}

/// 解析 Close Position Log 指令
#[allow(unused_variables)]
fn parse_close_position_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // owner (Pubkey - 32 bytes)
    let owner = read_pubkey(data, offset)?;
    offset += 32;

    // position (Pubkey - 32 bytes)
    let position = read_pubkey(data, offset)?;
    offset += 32;

    // positionNftMint (Pubkey - 32 bytes)
    let position_nft_mint = read_pubkey(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
        metadata,
        pool,
        owner,
        position,
        position_nft_mint,
    }))
}

/// 解析 Add Liquidity Log 指令
#[allow(unused_variables)]
fn parse_add_liquidity_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // position (Pubkey - 32 bytes)
    let position = read_pubkey(data, offset)?;
    offset += 32;

    // owner (Pubkey - 32 bytes)
    let owner = read_pubkey(data, offset)?;
    offset += 32;

    // params.liquidityDelta (u128 - 16 bytes)
    let liquidity_delta = read_u128_le(data, offset)?;
    offset += 16;

    // params.tokenAAmountThreshold (u64 - 8 bytes)
    let token_a_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // params.tokenBAmountThreshold (u64 - 8 bytes)
    let token_b_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // tokenAAmount (u64 - 8 bytes)
    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    // tokenBAmount (u64 - 8 bytes)
    let token_b_amount = read_u64_le(data, offset)?;
    offset += 8;

    // totalAmountA (u64 - 8 bytes)
    let total_amount_a = read_u64_le(data, offset)?;
    offset += 8;

    // totalAmountB (u64 - 8 bytes)
    let total_amount_b = read_u64_le(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
        metadata,
        pool,
        position,
        owner,
        liquidity_delta,
        token_a_amount_threshold,
        token_b_amount_threshold,
        token_a_amount,
        token_b_amount,
        total_amount_a,
        total_amount_b,
    }))
}

/// 解析 Add Liquidity Log 指令
#[allow(unused_variables)]
fn parse_remove_liquidity_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // position (Pubkey - 32 bytes)
    let position = read_pubkey(data, offset)?;
    offset += 32;

    // owner (Pubkey - 32 bytes)
    let owner = read_pubkey(data, offset)?;
    offset += 32;

    // params.liquidityDelta (u128 - 16 bytes)
    let liquidity_delta = read_u128_le(data, offset)?;
    offset += 16;

    // params.tokenAAmountThreshold (u64 - 8 bytes)
    let token_a_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // params.tokenBAmountThreshold (u64 - 8 bytes)
    let token_b_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // tokenAAmount (u64 - 8 bytes)
    let token_a_amount = read_u64_le(data, offset)?;
    offset += 8;

    // tokenBAmount (u64 - 8 bytes)
    let token_b_amount = read_u64_le(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
        metadata,
        pool,
        position,
        owner,
        liquidity_delta,
        token_a_amount_threshold,
        token_b_amount_threshold,
        token_a_amount,
        token_b_amount,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outer_initialize_pool_instruction() {
        let mut data = Vec::from(discriminators::INITIALIZE_POOL);
        data.extend_from_slice(&123u128.to_le_bytes());
        data.extend_from_slice(&456u128.to_le_bytes());
        data.push(1);
        data.extend_from_slice(&789u64.to_le_bytes());

        let accounts: Vec<Pubkey> = (0..10).map(|i| Pubkey::new_from_array([i; 32])).collect();
        let event = parse_instruction(&data, &accounts, Signature::default(), 1, 2, Some(3), 4)
            .expect("initialize pool event");

        match event {
            DexEvent::MeteoraDammV2InitializePool(event) => {
                assert_eq!(event.creator, accounts[0]);
                assert_eq!(event.position_nft_mint, accounts[1]);
                assert_eq!(event.pool, accounts[6]);
                assert_eq!(event.position, accounts[7]);
                assert_eq!(event.token_a_mint, accounts[8]);
                assert_eq!(event.token_b_mint, accounts[9]);
                assert_eq!(event.liquidity, 123);
                assert_eq!(event.sqrt_price, 456);
                assert_eq!(event.activation_point, Some(789));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
