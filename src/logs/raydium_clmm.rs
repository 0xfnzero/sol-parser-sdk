//! Raydium CLMM 日志解析器
//!
//! 使用 match discriminator 模式解析 Raydium CLMM 事件

use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium CLMM discriminator 常量
pub mod discriminators {
    pub const SWAP: [u8; 8] = [64, 198, 205, 232, 38, 8, 113, 226];
    pub const INCREASE_LIQUIDITY: [u8; 8] = [49, 79, 105, 212, 32, 34, 30, 84];
    pub const DECREASE_LIQUIDITY: [u8; 8] = [58, 222, 86, 58, 68, 50, 85, 56];
    pub const LIQUIDITY_CHANGE: [u8; 8] = [126, 240, 175, 206, 158, 88, 153, 107];
    pub const CONFIG_CHANGE: [u8; 8] = [247, 189, 7, 119, 106, 112, 95, 151];
    pub const CREATE_PERSONAL_POSITION: [u8; 8] = [100, 30, 87, 249, 196, 223, 154, 206];
    pub const LIQUIDITY_CALCULATE: [u8; 8] = [237, 112, 148, 230, 57, 84, 180, 162];
    pub const OPEN_LIMIT_ORDER: [u8; 8] = [106, 24, 71, 85, 57, 169, 158, 216];
    pub const INCREASE_LIMIT_ORDER: [u8; 8] = [11, 120, 13, 204, 199, 87, 19, 200];
    pub const DECREASE_LIMIT_ORDER: [u8; 8] = [70, 48, 40, 221, 219, 237, 212, 163];
    pub const SETTLE_LIMIT_ORDER: [u8; 8] = [88, 119, 77, 164, 125, 124, 10, 194];
    pub const UPDATE_REWARD_INFOS: [u8; 8] = [109, 127, 186, 78, 114, 65, 37, 236];
    pub const CREATE_POOL: [u8; 8] = [25, 94, 75, 47, 112, 99, 53, 63];
    pub const COLLECT_PERSONAL_FEE: [u8; 8] = [166, 174, 105, 192, 81, 161, 83, 105];
    pub const COLLECT_PROTOCOL_FEE: [u8; 8] = [206, 87, 17, 79, 45, 41, 213, 61];
}

/// Raydium CLMM 程序 ID
pub const PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// 检查日志是否来自 Raydium CLMM 程序
pub fn is_raydium_clmm_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID))
        || log.contains(&format!("Program {} success", PROGRAM_ID))
        || log.contains("raydium")
        || log.contains("Raydium")
}

/// 主要的 Raydium CLMM 日志解析函数
#[inline]
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, tx_index, block_time_us, grpc_recv_us)
}

/// 结构化日志解析（基于 Program data）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;
    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::SWAP => {
            parse_swap_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::INCREASE_LIQUIDITY => parse_increase_liquidity_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::DECREASE_LIQUIDITY => parse_decrease_liquidity_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::LIQUIDITY_CHANGE => parse_liquidity_change_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CONFIG_CHANGE => {
            parse_config_change_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CREATE_PERSONAL_POSITION => parse_create_personal_position_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::LIQUIDITY_CALCULATE => parse_liquidity_calculate_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::OPEN_LIMIT_ORDER => parse_open_limit_order_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::INCREASE_LIMIT_ORDER => parse_increase_limit_order_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::DECREASE_LIMIT_ORDER => parse_decrease_limit_order_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::SETTLE_LIMIT_ORDER => parse_settle_limit_order_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::UPDATE_REWARD_INFOS => parse_update_reward_infos_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CREATE_POOL => {
            parse_create_pool_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::COLLECT_PERSONAL_FEE => parse_collect_personal_fee_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::COLLECT_PROTOCOL_FEE => parse_collect_protocol_fee_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        _ => None,
    }
}

/// 解析交换事件
fn parse_swap_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let sender = read_pubkey(data, offset)?;
    offset += 32;

    let token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_1 = read_u64_le(data, offset)?;
    offset += 8;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let tick = read_i32_le(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,
        pool_state,
        sender,
        token_account_0,
        token_account_1,
        amount_0,
        transfer_fee_0,
        amount_1,
        transfer_fee_1,
        zero_for_one,
        sqrt_price_x64,
        liquidity,
        tick,
    }))
}

/// 解析增加流动性事件
fn parse_increase_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_0_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_transfer_fee = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        position_nft_mint,
        liquidity,
        amount_0,
        amount_1,
        amount_0_transfer_fee,
        amount_1_transfer_fee,
        pool: Pubkey::default(),
        user: Pubkey::default(), // TODO: extract from instruction accounts
        amount0_max: 0,
        amount1_max: 0,
    }))
}

/// 解析减少流动性事件
fn parse_decrease_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let decrease_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let decrease_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let fee_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let fee_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let mut reward_amounts = [0u64; 3];
    for reward_amount in &mut reward_amounts {
        *reward_amount = read_u64_le(data, offset)?;
        offset += 8;
    }

    let transfer_fee_0 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_1 = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        position_nft_mint,
        liquidity,
        decrease_amount_0,
        decrease_amount_1,
        fee_amount_0,
        fee_amount_1,
        reward_amounts,
        transfer_fee_0,
        transfer_fee_1,
        pool: Pubkey::default(),
        user: Pubkey::default(), // TODO: extract from instruction accounts
        amount0_min: 0,
        amount1_min: 0,
    }))
}

/// 解析流动性变化事件
fn parse_liquidity_change_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let tick = read_i32_le(data, offset)?;
    offset += 4;

    let tick_lower = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity_before = read_u128_le(data, offset)?;
    offset += 16;

    let liquidity_after = read_u128_le(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumClmmLiquidityChange(RaydiumClmmLiquidityChangeEvent {
        metadata,
        pool_state,
        tick,
        tick_lower,
        tick_upper,
        liquidity_before,
        liquidity_after,
    }))
}

fn parse_config_change_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );
    parse_config_change_from_data(data, metadata)
}

fn parse_create_personal_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_state = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);
    parse_create_personal_position_from_data(data, metadata)
}

fn parse_liquidity_calculate_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );
    parse_liquidity_calculate_from_data(data, metadata)
}

fn parse_open_limit_order_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_id = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_id, grpc_recv_us);
    parse_open_limit_order_from_data(data, metadata)
}

fn parse_increase_limit_order_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_id = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_id, grpc_recv_us);
    parse_increase_limit_order_from_data(data, metadata)
}

fn parse_decrease_limit_order_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_id = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_id, grpc_recv_us);
    parse_decrease_limit_order_from_data(data, metadata)
}

fn parse_settle_limit_order_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_id = read_pubkey(data, 0).unwrap_or_default();
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_id, grpc_recv_us);
    parse_settle_limit_order_from_data(data, metadata)
}

fn parse_update_reward_infos_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );
    parse_update_reward_infos_from_data(data, metadata)
}

/// 解析池创建事件
fn parse_create_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let token_0_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_1_mint = read_pubkey(data, offset)?;
    offset += 32;

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let tick = read_i32_le(data, offset)?;
    offset += 4;

    let token_vault_0 = read_pubkey(data, offset)?;
    offset += 32;

    let token_vault_1 = read_pubkey(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool: pool_state,
        token_0_mint,
        token_1_mint,
        tick_spacing,
        sqrt_price_x64,
        tick,
        token_vault_0,
        token_vault_1,
        fee_rate: 0,
        creator: Pubkey::default(),
        open_time: 0,
    }))
}

/// 解析个人费用收集事件
fn parse_collect_personal_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state: Pubkey::default(),
        position_nft_mint,
        recipient_token_account_0,
        recipient_token_account_1,
        amount_0,
        amount_1,
    }))
}

/// 解析协议费用收集事件
fn parse_collect_protocol_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state,
        position_nft_mint: Pubkey::default(),
        recipient_token_account_0,
        recipient_token_account_1,
        amount_0,
        amount_1,
    }))
}

/// 文本回退解析
fn parse_text_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("swap") || log.contains("Swap") {
        return parse_swap_from_text(log, signature, slot, tx_index, block_time_us, grpc_recv_us);
    }

    if log.contains("increase") && log.contains("liquidity") {
        return parse_increase_liquidity_from_text(
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    if log.contains("decrease") && log.contains("liquidity") {
        return parse_decrease_liquidity_from_text(
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    if log.contains("create") && log.contains("pool") {
        return parse_create_pool_from_text(
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    if log.contains("collect") && log.contains("fee") {
        return parse_collect_fee_from_text(
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    None
}

/// 从文本解析交换事件
fn parse_swap_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );
    let is_base_input = detect_trade_type(log).unwrap_or(true);

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_state: Pubkey::default(),
        sender: Pubkey::default(),
        token_account_0: Pubkey::default(),
        token_account_1: Pubkey::default(),
        amount_0: 0,
        transfer_fee_0: 0,
        amount_1: 0,
        transfer_fee_1: 0,
        zero_for_one: is_base_input,
        sqrt_price_x64: 0,
        // is_base_input,
        liquidity: 0,
        tick: 0,
        // 暂时注释，以后会用，AI禁止改动
        // 指令参数字段
        // amount: extract_number_from_text(log, "amount").unwrap_or(1_000_000_000),
        // other_amount_threshold: extract_number_from_text(log, "threshold").unwrap_or(950_000_000),
        // sqrt_price_limit_x64: 0,
    }))
}

/// 从文本解析增加流动性事件
fn parse_increase_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        position_nft_mint: Pubkey::default(),
        liquidity: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000) as u128,
        amount_0: 0,
        amount_1: 0,
        amount_0_transfer_fee: 0,
        amount_1_transfer_fee: 0,
        pool: Pubkey::default(),
        amount0_max: extract_number_from_text(log, "amount0_max").unwrap_or(1_000_000),
        amount1_max: extract_number_from_text(log, "amount1_max").unwrap_or(1_000_000),
        user: Pubkey::default(),
    }))
}

/// 从文本解析减少流动性事件
fn parse_decrease_liquidity_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        position_nft_mint: Pubkey::default(),
        liquidity: extract_number_from_text(log, "liquidity").unwrap_or(1_000_000) as u128,
        decrease_amount_0: 0,
        decrease_amount_1: 0,
        fee_amount_0: 0,
        fee_amount_1: 0,
        reward_amounts: [0; 3],
        transfer_fee_0: 0,
        transfer_fee_1: 0,
        pool: Pubkey::default(),
        amount0_min: extract_number_from_text(log, "amount0_min").unwrap_or(1_000_000),
        amount1_min: extract_number_from_text(log, "amount1_min").unwrap_or(1_000_000),
        user: Pubkey::default(),
    }))
}

/// 从文本解析池创建事件
fn parse_create_pool_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool: Pubkey::default(),
        token_0_mint: Pubkey::default(),
        token_1_mint: Pubkey::default(),
        tick_spacing: 0,
        sqrt_price_x64: 0,
        tick: 0,
        token_vault_0: Pubkey::default(),
        token_vault_1: Pubkey::default(),
        fee_rate: 0,
        creator: Pubkey::default(),
        open_time: 0,
    }))
}

/// 从文本解析费用收集事件
fn parse_collect_fee_from_text(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(
        signature,
        slot,
        tx_index,
        block_time_us,
        Pubkey::default(),
        grpc_recv_us,
    );

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state: Pubkey::default(),
        position_nft_mint: Pubkey::default(),
        recipient_token_account_0: Pubkey::default(),
        recipient_token_account_1: Pubkey::default(),
        amount_0: extract_number_from_text(log, "amount_0").unwrap_or(10_000),
        amount_1: extract_number_from_text(log, "amount_1").unwrap_or(10_000),
    }))
}

// ============================================================================
// Public API for optimized parsing from pre-decoded data
// These functions accept already-decoded data (without discriminator)
// ============================================================================

/// Parse Raydium CLMM Swap event from pre-decoded data
#[inline(always)]
pub fn parse_swap_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let sender = read_pubkey(data, offset)?;
    offset += 32;

    let token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_1 = read_u64_le(data, offset)?;
    offset += 8;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let tick = read_i32_le(data, offset)?;

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,
        pool_state,
        sender,
        token_account_0,
        token_account_1,
        amount_0,
        transfer_fee_0,
        amount_1,
        transfer_fee_1,
        zero_for_one,
        sqrt_price_x64,
        liquidity,
        tick,
    }))
}

/// Parse Raydium CLMM IncreaseLiquidity event from pre-decoded data
#[inline(always)]
pub fn parse_increase_liquidity_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_0_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_transfer_fee = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        position_nft_mint,
        liquidity,
        amount_0,
        amount_1,
        amount_0_transfer_fee,
        amount_1_transfer_fee,
        pool: Pubkey::default(),
        amount0_max: 0,
        amount1_max: 0,
        user: Pubkey::default(),
    }))
}

/// Parse Raydium CLMM DecreaseLiquidity event from pre-decoded data
#[inline(always)]
pub fn parse_decrease_liquidity_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let decrease_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let decrease_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let fee_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let fee_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let mut reward_amounts = [0u64; 3];
    for reward_amount in &mut reward_amounts {
        *reward_amount = read_u64_le(data, offset)?;
        offset += 8;
    }

    let transfer_fee_0 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_1 = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        position_nft_mint,
        liquidity,
        decrease_amount_0,
        decrease_amount_1,
        fee_amount_0,
        fee_amount_1,
        reward_amounts,
        transfer_fee_0,
        transfer_fee_1,
        pool: Pubkey::default(),
        amount0_min: 0,
        amount1_min: 0,
        user: Pubkey::default(),
    }))
}

/// Parse Raydium CLMM LiquidityChange event from pre-decoded data
#[inline(always)]
pub fn parse_liquidity_change_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let tick = read_i32_le(data, offset)?;
    offset += 4;

    let tick_lower = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity_before = read_u128_le(data, offset)?;
    offset += 16;

    let liquidity_after = read_u128_le(data, offset)?;

    Some(DexEvent::RaydiumClmmLiquidityChange(RaydiumClmmLiquidityChangeEvent {
        metadata,
        pool_state,
        tick,
        tick_lower,
        tick_upper,
        liquidity_before,
        liquidity_after,
    }))
}

/// Parse Raydium CLMM ConfigChange event from pre-decoded data
#[inline(always)]
pub fn parse_config_change_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let index = read_u16_le(data, offset)?;
    offset += 2;

    let owner = read_pubkey(data, offset)?;
    offset += 32;

    let protocol_fee_rate = read_u32_le(data, offset)?;
    offset += 4;

    let trade_fee_rate = read_u32_le(data, offset)?;
    offset += 4;

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let fund_fee_rate = read_u32_le(data, offset)?;
    offset += 4;

    let fund_owner = read_pubkey(data, offset)?;

    Some(DexEvent::RaydiumClmmConfigChange(RaydiumClmmConfigChangeEvent {
        metadata,
        index,
        owner,
        protocol_fee_rate,
        trade_fee_rate,
        tick_spacing,
        fund_fee_rate,
        fund_owner,
    }))
}

/// Parse Raydium CLMM CreatePersonalPosition event from pre-decoded data
#[inline(always)]
pub fn parse_create_personal_position_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let minter = read_pubkey(data, offset)?;
    offset += 32;

    let nft_owner = read_pubkey(data, offset)?;
    offset += 32;

    let tick_lower_index = read_i32_le(data, offset)?;
    offset += 4;

    let tick_upper_index = read_i32_le(data, offset)?;
    offset += 4;

    let liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let deposit_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let deposit_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let deposit_amount_0_transfer_fee = read_u64_le(data, offset)?;
    offset += 8;

    let deposit_amount_1_transfer_fee = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmCreatePersonalPosition(RaydiumClmmCreatePersonalPositionEvent {
        metadata,
        pool_state,
        minter,
        nft_owner,
        tick_lower_index,
        tick_upper_index,
        liquidity,
        deposit_amount_0,
        deposit_amount_1,
        deposit_amount_0_transfer_fee,
        deposit_amount_1_transfer_fee,
    }))
}

/// Parse Raydium CLMM LiquidityCalculate event from pre-decoded data
#[inline(always)]
pub fn parse_liquidity_calculate_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_liquidity = read_u128_le(data, offset)?;
    offset += 16;

    let pool_sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let pool_tick = read_i32_le(data, offset)?;
    offset += 4;

    let calc_amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let calc_amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    let trade_fee_owed_0 = read_u64_le(data, offset)?;
    offset += 8;

    let trade_fee_owed_1 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_0 = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee_1 = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmLiquidityCalculate(RaydiumClmmLiquidityCalculateEvent {
        metadata,
        pool_liquidity,
        pool_sqrt_price_x64,
        pool_tick,
        calc_amount_0,
        calc_amount_1,
        trade_fee_owed_0,
        trade_fee_owed_1,
        transfer_fee_0,
        transfer_fee_1,
    }))
}

/// Parse Raydium CLMM OpenLimitOrder event from pre-decoded data
#[inline(always)]
pub fn parse_open_limit_order_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_id = read_pubkey(data, offset)?;
    offset += 32;

    let limit_order = read_pubkey(data, offset)?;
    offset += 32;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let tick_index = read_i32_le(data, offset)?;
    offset += 4;

    let total_amount = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmOpenLimitOrder(RaydiumClmmOpenLimitOrderEvent {
        metadata,
        pool_id,
        limit_order,
        zero_for_one,
        tick_index,
        total_amount,
        transfer_fee,
    }))
}

/// Parse Raydium CLMM IncreaseLimitOrder event from pre-decoded data
#[inline(always)]
pub fn parse_increase_limit_order_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_id = read_pubkey(data, offset)?;
    offset += 32;

    let limit_order = read_pubkey(data, offset)?;
    offset += 32;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let tick_index = read_i32_le(data, offset)?;
    offset += 4;

    let total_amount = read_u64_le(data, offset)?;
    offset += 8;

    let increased_amount = read_u64_le(data, offset)?;
    offset += 8;

    let transfer_fee = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmIncreaseLimitOrder(RaydiumClmmIncreaseLimitOrderEvent {
        metadata,
        pool_id,
        limit_order,
        zero_for_one,
        tick_index,
        total_amount,
        increased_amount,
        transfer_fee,
    }))
}

/// Parse Raydium CLMM DecreaseLimitOrder event from pre-decoded data
#[inline(always)]
pub fn parse_decrease_limit_order_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_id = read_pubkey(data, offset)?;
    offset += 32;

    let limit_order = read_pubkey(data, offset)?;
    offset += 32;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let tick_index = read_i32_le(data, offset)?;
    offset += 4;

    let total_amount = read_u64_le(data, offset)?;
    offset += 8;

    let filled_amount = read_u64_le(data, offset)?;
    offset += 8;

    let settled_output_amount = read_u64_le(data, offset)?;
    offset += 8;

    let decreased_amount = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmDecreaseLimitOrder(RaydiumClmmDecreaseLimitOrderEvent {
        metadata,
        pool_id,
        limit_order,
        zero_for_one,
        tick_index,
        total_amount,
        filled_amount,
        settled_output_amount,
        decreased_amount,
    }))
}

/// Parse Raydium CLMM SettleLimitOrder event from pre-decoded data
#[inline(always)]
pub fn parse_settle_limit_order_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_id = read_pubkey(data, offset)?;
    offset += 32;

    let limit_order = read_pubkey(data, offset)?;
    offset += 32;

    let zero_for_one = read_bool(data, offset)?;
    offset += 1;

    let tick_index = read_i32_le(data, offset)?;
    offset += 4;

    let total_amount = read_u64_le(data, offset)?;
    offset += 8;

    let filled_amount = read_u64_le(data, offset)?;
    offset += 8;

    let settled_amount_out = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmSettleLimitOrder(RaydiumClmmSettleLimitOrderEvent {
        metadata,
        pool_id,
        limit_order,
        zero_for_one,
        tick_index,
        total_amount,
        filled_amount,
        settled_amount_out,
    }))
}

/// Parse Raydium CLMM UpdateRewardInfos event from pre-decoded data
#[inline(always)]
pub fn parse_update_reward_infos_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;
    let mut reward_growth_global_x64 = [0u128; 3];
    for reward_growth in &mut reward_growth_global_x64 {
        *reward_growth = read_u128_le(data, offset)?;
        offset += 16;
    }

    Some(DexEvent::RaydiumClmmUpdateRewardInfos(RaydiumClmmUpdateRewardInfosEvent {
        metadata,
        reward_growth_global_x64,
    }))
}

/// Parse Raydium CLMM CreatePool event from pre-decoded data
#[inline(always)]
pub fn parse_create_pool_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    let mut offset = 0;

    let token_0_mint = read_pubkey(data, offset)?;
    offset += 32;

    let token_1_mint = read_pubkey(data, offset)?;
    offset += 32;

    let tick_spacing = read_u16_le(data, offset)?;
    offset += 2;

    let pool = read_pubkey(data, offset)?;
    offset += 32;

    let sqrt_price_x64 = read_u128_le(data, offset)?;
    offset += 16;

    let tick = read_i32_le(data, offset)?;
    offset += 4;

    let token_vault_0 = read_pubkey(data, offset)?;
    offset += 32;

    let token_vault_1 = read_pubkey(data, offset)?;

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool,
        token_0_mint,
        token_1_mint,
        tick_spacing,
        sqrt_price_x64,
        tick,
        token_vault_0,
        token_vault_1,
        fee_rate: 0,
        creator: Pubkey::default(),
        open_time: 0,
    }))
}

/// Parse Raydium CLMM CollectPersonalFee event from pre-decoded data
#[inline(always)]
pub fn parse_collect_personal_fee_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let position_nft_mint = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state: Pubkey::default(),
        position_nft_mint,
        recipient_token_account_0,
        recipient_token_account_1,
        amount_0,
        amount_1,
    }))
}

/// Parse Raydium CLMM CollectProtocolFee event from pre-decoded data
#[inline(always)]
pub fn parse_collect_protocol_fee_from_data(
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_0 = read_pubkey(data, offset)?;
    offset += 32;

    let recipient_token_account_1 = read_pubkey(data, offset)?;
    offset += 32;

    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1 = read_u64_le(data, offset)?;

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        pool_state,
        position_nft_mint: Pubkey::default(),
        recipient_token_account_0,
        recipient_token_account_1,
        amount_0,
        amount_1,
    }))
}

/// Backward-compatible alias for callers compiled against the older parser API.
#[inline(always)]
pub fn parse_collect_fee_from_data(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    parse_collect_personal_fee_from_data(data, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};

    fn metadata() -> EventMetadata {
        EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        }
    }

    fn pk(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn push_pk(out: &mut Vec<u8>, key: Pubkey) {
        out.extend_from_slice(key.as_ref());
    }

    fn program_data_log(raw: &[u8]) -> String {
        format!("Program data: {}", general_purpose::STANDARD.encode(raw))
    }

    #[test]
    fn official_swap_event_discriminator_and_body_parse() {
        let pool = pk(1);
        let sender = pk(2);
        let token_account_0 = pk(3);
        let token_account_1 = pk(4);
        let mut raw = Vec::new();
        raw.extend_from_slice(&discriminators::SWAP);
        push_pk(&mut raw, pool);
        push_pk(&mut raw, sender);
        push_pk(&mut raw, token_account_0);
        push_pk(&mut raw, token_account_1);
        raw.extend_from_slice(&10u64.to_le_bytes());
        raw.extend_from_slice(&1u64.to_le_bytes());
        raw.extend_from_slice(&20u64.to_le_bytes());
        raw.extend_from_slice(&2u64.to_le_bytes());
        raw.push(1);
        raw.extend_from_slice(&123u128.to_le_bytes());
        raw.extend_from_slice(&456u128.to_le_bytes());
        raw.extend_from_slice(&(-77i32).to_le_bytes());

        let event = parse_log(&program_data_log(&raw), Signature::default(), 1, 0, None, 0)
            .expect("swap event");

        let DexEvent::RaydiumClmmSwap(swap) = event else {
            panic!("expected swap");
        };
        assert_eq!(swap.pool_state, pool);
        assert_eq!(swap.sender, sender);
        assert_eq!(swap.token_account_0, token_account_0);
        assert_eq!(swap.token_account_1, token_account_1);
        assert_eq!(swap.amount_0, 10);
        assert_eq!(swap.transfer_fee_0, 1);
        assert_eq!(swap.amount_1, 20);
        assert_eq!(swap.transfer_fee_1, 2);
        assert!(swap.zero_for_one);
        assert_eq!(swap.sqrt_price_x64, 123);
        assert_eq!(swap.liquidity, 456);
        assert_eq!(swap.tick, -77);
    }

    #[test]
    fn instruction_swap_discriminator_is_not_treated_as_log_event() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&[248, 198, 158, 145, 225, 117, 135, 200]);
        raw.resize(8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 16 + 16 + 4, 0);

        assert!(parse_log(&program_data_log(&raw), Signature::default(), 1, 0, None, 0).is_none());
    }

    #[test]
    fn official_liquidity_and_create_events_parse() {
        let mut inc = Vec::new();
        push_pk(&mut inc, pk(5));
        inc.extend_from_slice(&100u128.to_le_bytes());
        inc.extend_from_slice(&11u64.to_le_bytes());
        inc.extend_from_slice(&22u64.to_le_bytes());
        inc.extend_from_slice(&3u64.to_le_bytes());
        inc.extend_from_slice(&4u64.to_le_bytes());

        let DexEvent::RaydiumClmmIncreaseLiquidity(inc_event) =
            parse_increase_liquidity_from_data(&inc, metadata()).expect("increase")
        else {
            panic!("expected increase");
        };
        assert_eq!(inc_event.position_nft_mint, pk(5));
        assert_eq!(inc_event.liquidity, 100);
        assert_eq!(inc_event.amount_0, 11);
        assert_eq!(inc_event.amount_1, 22);
        assert_eq!(inc_event.amount_0_transfer_fee, 3);
        assert_eq!(inc_event.amount_1_transfer_fee, 4);

        let mut dec = Vec::new();
        push_pk(&mut dec, pk(6));
        dec.extend_from_slice(&200u128.to_le_bytes());
        dec.extend_from_slice(&31u64.to_le_bytes());
        dec.extend_from_slice(&32u64.to_le_bytes());
        dec.extend_from_slice(&5u64.to_le_bytes());
        dec.extend_from_slice(&6u64.to_le_bytes());
        dec.extend_from_slice(
            &[7u64.to_le_bytes(), 8u64.to_le_bytes(), 9u64.to_le_bytes()].concat(),
        );
        dec.extend_from_slice(&10u64.to_le_bytes());
        dec.extend_from_slice(&11u64.to_le_bytes());

        let DexEvent::RaydiumClmmDecreaseLiquidity(dec_event) =
            parse_decrease_liquidity_from_data(&dec, metadata()).expect("decrease")
        else {
            panic!("expected decrease");
        };
        assert_eq!(dec_event.position_nft_mint, pk(6));
        assert_eq!(dec_event.liquidity, 200);
        assert_eq!(dec_event.decrease_amount_0, 31);
        assert_eq!(dec_event.decrease_amount_1, 32);
        assert_eq!(dec_event.fee_amount_0, 5);
        assert_eq!(dec_event.fee_amount_1, 6);
        assert_eq!(dec_event.reward_amounts, [7, 8, 9]);
        assert_eq!(dec_event.transfer_fee_0, 10);
        assert_eq!(dec_event.transfer_fee_1, 11);

        let mut create = Vec::new();
        push_pk(&mut create, pk(7));
        push_pk(&mut create, pk(8));
        create.extend_from_slice(&64u16.to_le_bytes());
        push_pk(&mut create, pk(9));
        create.extend_from_slice(&333u128.to_le_bytes());
        create.extend_from_slice(&(-12i32).to_le_bytes());
        push_pk(&mut create, pk(10));
        push_pk(&mut create, pk(11));

        let DexEvent::RaydiumClmmCreatePool(create_event) =
            parse_create_pool_from_data(&create, metadata()).expect("create")
        else {
            panic!("expected create");
        };
        assert_eq!(create_event.token_0_mint, pk(7));
        assert_eq!(create_event.token_1_mint, pk(8));
        assert_eq!(create_event.tick_spacing, 64);
        assert_eq!(create_event.pool, pk(9));
        assert_eq!(create_event.sqrt_price_x64, 333);
        assert_eq!(create_event.tick, -12);
        assert_eq!(create_event.token_vault_0, pk(10));
        assert_eq!(create_event.token_vault_1, pk(11));
    }

    #[test]
    fn official_collect_and_liquidity_change_events_parse() {
        let mut personal = Vec::new();
        push_pk(&mut personal, pk(12));
        push_pk(&mut personal, pk(13));
        push_pk(&mut personal, pk(14));
        personal.extend_from_slice(&70u64.to_le_bytes());
        personal.extend_from_slice(&80u64.to_le_bytes());

        let DexEvent::RaydiumClmmCollectFee(personal_event) =
            parse_collect_personal_fee_from_data(&personal, metadata()).expect("personal")
        else {
            panic!("expected personal collect");
        };
        assert_eq!(personal_event.position_nft_mint, pk(12));
        assert_eq!(personal_event.recipient_token_account_0, pk(13));
        assert_eq!(personal_event.recipient_token_account_1, pk(14));

        let mut protocol = Vec::new();
        push_pk(&mut protocol, pk(15));
        push_pk(&mut protocol, pk(16));
        push_pk(&mut protocol, pk(17));
        protocol.extend_from_slice(&90u64.to_le_bytes());
        protocol.extend_from_slice(&100u64.to_le_bytes());

        let DexEvent::RaydiumClmmCollectFee(protocol_event) =
            parse_collect_protocol_fee_from_data(&protocol, metadata()).expect("protocol")
        else {
            panic!("expected protocol collect");
        };
        assert_eq!(protocol_event.pool_state, pk(15));
        assert_eq!(protocol_event.recipient_token_account_0, pk(16));
        assert_eq!(protocol_event.recipient_token_account_1, pk(17));

        let mut change = Vec::new();
        push_pk(&mut change, pk(18));
        change.extend_from_slice(&1i32.to_le_bytes());
        change.extend_from_slice(&(-10i32).to_le_bytes());
        change.extend_from_slice(&10i32.to_le_bytes());
        change.extend_from_slice(&1234u128.to_le_bytes());
        change.extend_from_slice(&5678u128.to_le_bytes());

        let DexEvent::RaydiumClmmLiquidityChange(change_event) =
            parse_liquidity_change_from_data(&change, metadata()).expect("liquidity change")
        else {
            panic!("expected liquidity change");
        };
        assert_eq!(change_event.pool_state, pk(18));
        assert_eq!(change_event.tick, 1);
        assert_eq!(change_event.tick_lower, -10);
        assert_eq!(change_event.tick_upper, 10);
        assert_eq!(change_event.liquidity_before, 1234);
        assert_eq!(change_event.liquidity_after, 5678);
    }

    #[test]
    fn official_dynamic_fee_and_limit_order_events_parse() {
        let mut config = Vec::new();
        config.extend_from_slice(&3u16.to_le_bytes());
        push_pk(&mut config, pk(21));
        config.extend_from_slice(&100u32.to_le_bytes());
        config.extend_from_slice(&200u32.to_le_bytes());
        config.extend_from_slice(&64u16.to_le_bytes());
        config.extend_from_slice(&300u32.to_le_bytes());
        push_pk(&mut config, pk(22));

        let DexEvent::RaydiumClmmConfigChange(config_event) =
            parse_config_change_from_data(&config, metadata()).expect("config")
        else {
            panic!("expected config change");
        };
        assert_eq!(config_event.index, 3);
        assert_eq!(config_event.owner, pk(21));
        assert_eq!(config_event.fund_owner, pk(22));

        let mut personal_position = Vec::new();
        push_pk(&mut personal_position, pk(23));
        push_pk(&mut personal_position, pk(24));
        push_pk(&mut personal_position, pk(25));
        personal_position.extend_from_slice(&(-4i32).to_le_bytes());
        personal_position.extend_from_slice(&8i32.to_le_bytes());
        personal_position.extend_from_slice(&900u128.to_le_bytes());
        personal_position.extend_from_slice(&31u64.to_le_bytes());
        personal_position.extend_from_slice(&32u64.to_le_bytes());
        personal_position.extend_from_slice(&1u64.to_le_bytes());
        personal_position.extend_from_slice(&2u64.to_le_bytes());

        let DexEvent::RaydiumClmmCreatePersonalPosition(position_event) =
            parse_create_personal_position_from_data(&personal_position, metadata())
                .expect("personal position")
        else {
            panic!("expected personal position");
        };
        assert_eq!(position_event.pool_state, pk(23));
        assert_eq!(position_event.minter, pk(24));
        assert_eq!(position_event.nft_owner, pk(25));
        assert_eq!(position_event.liquidity, 900);

        let mut open_order = Vec::new();
        push_pk(&mut open_order, pk(26));
        push_pk(&mut open_order, pk(27));
        open_order.push(1);
        open_order.extend_from_slice(&(-40i32).to_le_bytes());
        open_order.extend_from_slice(&1_000u64.to_le_bytes());
        open_order.extend_from_slice(&5u64.to_le_bytes());

        let DexEvent::RaydiumClmmOpenLimitOrder(open_event) =
            parse_open_limit_order_from_data(&open_order, metadata()).expect("open order")
        else {
            panic!("expected open limit order");
        };
        assert_eq!(open_event.pool_id, pk(26));
        assert_eq!(open_event.limit_order, pk(27));
        assert!(open_event.zero_for_one);
        assert_eq!(open_event.tick_index, -40);
        assert_eq!(open_event.total_amount, 1_000);
        assert_eq!(open_event.transfer_fee, 5);

        let mut settle_order = Vec::new();
        push_pk(&mut settle_order, pk(28));
        push_pk(&mut settle_order, pk(29));
        settle_order.push(0);
        settle_order.extend_from_slice(&41i32.to_le_bytes());
        settle_order.extend_from_slice(&2_000u64.to_le_bytes());
        settle_order.extend_from_slice(&1_500u64.to_le_bytes());
        settle_order.extend_from_slice(&3_000u64.to_le_bytes());

        let DexEvent::RaydiumClmmSettleLimitOrder(settle_event) =
            parse_settle_limit_order_from_data(&settle_order, metadata()).expect("settle order")
        else {
            panic!("expected settle limit order");
        };
        assert_eq!(settle_event.pool_id, pk(28));
        assert!(!settle_event.zero_for_one);
        assert_eq!(settle_event.settled_amount_out, 3_000);

        let mut reward = Vec::new();
        reward.extend_from_slice(&10u128.to_le_bytes());
        reward.extend_from_slice(&20u128.to_le_bytes());
        reward.extend_from_slice(&30u128.to_le_bytes());

        let DexEvent::RaydiumClmmUpdateRewardInfos(reward_event) =
            parse_update_reward_infos_from_data(&reward, metadata()).expect("reward")
        else {
            panic!("expected reward infos");
        };
        assert_eq!(reward_event.reward_growth_global_x64, [10, 20, 30]);
    }
}
