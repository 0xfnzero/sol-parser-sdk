//! PumpSwap DEX 解析器 - 函数式设计
//!
//! 专门解析 PumpSwap 相关的事件，包括：
//! - 买入事件
//! - 卖出事件
//! - 创建池事件

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
// use prost_types::Timestamp;
use crate::parser::events::*;

/// PumpSwap discriminator 常量 (需要根据实际合约获取)
pub mod discriminators {
    pub const BUY: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    pub const SELL: [u8; 8] = [11, 12, 13, 14, 15, 16, 17, 18];
    pub const CREATE_POOL: [u8; 8] = [12, 13, 14, 15, 16, 17, 18, 19];
}

/// PumpSwap 程序 ID (需要确认实际的程序ID)
pub const PUMPSWAP_PROGRAM_ID: &str = "PumpSWaP7evteam3bP1234567890123456789012345";

/// 原始 PumpSwap 买入事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpSwapBuyEvent {
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub price: u64,
    pub slippage: u16,
}

/// 原始 PumpSwap 卖出事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpSwapSellEvent {
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub token_amount: u64,
    pub sol_amount: u64,
    pub price: u64,
    pub slippage: u16,
}

/// 原始 PumpSwap 创建池事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpSwapCreatePoolEvent {
    pub pool_id: Pubkey,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub initial_sol_amount: u64,
    pub initial_token_amount: u64,
    pub fee_rate: u16,
}

/// 检查日志是否来自 PumpSwap 程序
pub fn is_pumpswap_program(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PUMPSWAP_PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PUMPSWAP_PROGRAM_ID)) ||
    log.contains("pumpswap") || // 简单的关键词检测
    log.contains("PumpSwap")
}

/// 从日志中提取 Program data
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    if let Some(data_start) = log.find("Program data: ") {
        let data_part = &log[data_start + 14..];
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(data_part.trim()).ok()
    } else {
        None
    }
}

/// 解析原始事件数据 - 纯函数
pub fn parse_raw_event<T: BorshDeserialize>(data: &[u8], expected_discriminator: [u8; 8]) -> Option<T> {
    if data.len() < 8 {
        return None;
    }

    // 检查 discriminator
    let discriminator: [u8; 8] = data[0..8].try_into().ok()?;
    if discriminator != expected_discriminator {
        return None;
    }

    // 反序列化剩余数据
    let mut event_data = &data[8..];
    T::deserialize(&mut event_data).ok()
}



/// 创建事件元数据 - 纯函数
pub fn create_event_metadata(
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
    program_id: Pubkey,
) -> EventMetadata {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    EventMetadata {
        signature,
        slot,
        block_time,
        block_time_ms: block_time.map(|ts| ts * 1000),
        program_id,
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: current_time,
        handle_us: current_time,
    }
}


/// 转换为 PumpSwap 买入事件 - 纯函数
pub fn convert_to_buy_event(
    raw: RawPumpSwapBuyEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> PumpSwapBuyEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    PumpSwapBuyEvent {
        metadata,
        pool_id: raw.pool_id,
        user: raw.user,
        token_mint: raw.token_mint,
        sol_amount: raw.sol_amount,
        token_amount: raw.token_amount,
        price: raw.price,
        slippage: raw.slippage,
    }
}

/// 转换为 PumpSwap 卖出事件 - 纯函数
pub fn convert_to_sell_event(
    raw: RawPumpSwapSellEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> PumpSwapSellEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    PumpSwapSellEvent {
        metadata,
        pool_id: raw.pool_id,
        user: raw.user,
        token_mint: raw.token_mint,
        token_amount: raw.token_amount,
        sol_amount: raw.sol_amount,
        price: raw.price,
        slippage: raw.slippage,
    }
}

/// 转换为 PumpSwap 创建池事件 - 纯函数
pub fn convert_to_create_pool_event(
    raw: RawPumpSwapCreatePoolEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> PumpSwapCreatePoolEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_id);

    PumpSwapCreatePoolEvent {
        metadata,
        pool_id: raw.pool_id,
        creator: raw.creator,
        token_mint: raw.token_mint,
        initial_sol_amount: raw.initial_sol_amount,
        initial_token_amount: raw.initial_token_amount,
        fee_rate: raw.fee_rate,
    }
}

/// 解析所有 PumpSwap 事件 - 单次循环返回统一事件数组
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    // 只循环一次logs！
    for log in logs {
        if !is_pumpswap_program(log) {
            continue;
        }

        if let Some(program_data) = extract_program_data(log) {
            if program_data.len() < 8 {
                continue;
            }

            // 提取discriminator
            let discriminator: [u8; 8] = match program_data[0..8].try_into() {
                Ok(disc) => disc,
                Err(_) => continue,
            };

            // 根据discriminator分发到不同的处理逻辑
            match discriminator {
                discriminators::BUY => {
                    if let Some(raw) = parse_raw_event::<RawPumpSwapBuyEvent>(&program_data, discriminators::BUY) {
                        let event = convert_to_buy_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpSwapBuy(event));
                    }
                }
                discriminators::SELL => {
                    if let Some(raw) = parse_raw_event::<RawPumpSwapSellEvent>(&program_data, discriminators::SELL) {
                        let event = convert_to_sell_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpSwapSell(event));
                    }
                }
                discriminators::CREATE_POOL => {
                    if let Some(raw) = parse_raw_event::<RawPumpSwapCreatePoolEvent>(&program_data, discriminators::CREATE_POOL) {
                        let event = convert_to_create_pool_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpSwapCreatePool(event));
                    }
                }
                _ => {
                    // 不是PumpSwap的事件，跳过
                    continue;
                }
            }
        }
    }

    events
}

/// 计算 PumpSwap 价格影响 - 纯函数
pub fn calculate_price_impact(buy_event: &PumpSwapBuyEvent) -> f64 {
    let slippage_percent = buy_event.slippage as f64 / 100.0; // 假设 slippage 是百分比 * 100
    slippage_percent
}

/// 判断是否是大额 PumpSwap 交易 - 纯函数
pub fn is_large_pumpswap_trade(sol_amount: u64) -> bool {
    sol_amount >= 500_000_000 // 0.5 SOL 及以上
}

/// 检查是否是 PumpSwap 日志
#[inline(always)]
pub fn is_pumpswap_log(log: &str) -> bool {
    log.contains(PUMPSWAP_PROGRAM_ID) || log.contains("Program data:")
}

/// 从日志字符串解析 PumpSwap 事件
pub fn parse_pumpswap_from_log_string(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<i64>,
) -> Option<DexEvent> {
    if !is_pumpswap_log(log) {
        return None;
    }

    // TODO: 实现完整的 PumpSwap 日志解析
    // 这里需要根据实际的 PumpSwap 合约日志格式来解析
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pumpswap_program_detection() {
        let log1 = format!("Program {} invoke [1]", PUMPSWAP_PROGRAM_ID);
        let log2 = format!("Program {} success", PUMPSWAP_PROGRAM_ID);
        let log3 = "Program other_program invoke [1]";
        let log4 = "This log contains pumpswap trading";

        assert!(is_pumpswap_program(&log1));
        assert!(is_pumpswap_program(&log2));
        assert!(!is_pumpswap_program(log3));
        assert!(is_pumpswap_program(log4));
    }

    #[test]
    fn test_large_trade_detection() {
        assert!(is_large_pumpswap_trade(1_000_000_000)); // 1 SOL
        assert!(is_large_pumpswap_trade(500_000_000));   // 0.5 SOL
        assert!(!is_large_pumpswap_trade(100_000_000));  // 0.1 SOL
    }

    #[test]
    fn test_price_impact_calculation() {
        let metadata = create_event_metadata(
            Signature::default(),
            0,
            None,
            Pubkey::default(),
        );

        let buy_event = PumpSwapBuyEvent {
            metadata,
            pool_id: Pubkey::default(),
            user: Pubkey::default(),
            token_mint: Pubkey::default(),
            sol_amount: 1_000_000_000,
            token_amount: 1000000,
            price: 1000,
            slippage: 250, // 2.5%
        };

        let impact = calculate_price_impact(&buy_event);
        assert_eq!(impact, 2.5);
    }
}