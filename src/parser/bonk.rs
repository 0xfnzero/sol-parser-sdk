//! Bonk DEX 解析器 - 函数式设计
//!
//! 专门解析 Bonk 相关的事件，包括：
//! - 交易事件
//! - 池创建事件
//! - AMM 迁移事件

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use prost_types::Timestamp;
use crate::parser::events::*;

/// Bonk discriminator 常量 (需要根据实际合约获取)
pub mod discriminators {
    pub const TRADE: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9]; // 需要实际的discriminator
    pub const POOL_CREATE: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const MIGRATE_AMM: [u8; 8] = [3, 4, 5, 6, 7, 8, 9, 10];
}

/// Bonk 程序 ID (需要确认实际的程序ID)
pub const BONK_PROGRAM_ID: &str = "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1";

/// 原始 Bonk 交易事件数据结构
#[derive(BorshDeserialize)]
pub struct RawBonkTradeEvent {
    pub pool_state: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_buy: bool,
    pub exact_in: bool,
}

/// 原始 Bonk 池创建事件数据结构
#[derive(BorshDeserialize)]
pub struct RawBonkPoolCreateEvent {
    pub pool_state: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub creator: Pubkey,
    pub initial_liquidity_a: u64,
    pub initial_liquidity_b: u64,
}

/// 原始 Bonk AMM 迁移事件数据结构
#[derive(BorshDeserialize)]
pub struct RawBonkMigrateAmmEvent {
    pub old_pool: Pubkey,
    pub new_pool: Pubkey,
    pub user: Pubkey,
    pub liquidity_amount: u64,
}

/// 检查日志是否来自 Bonk 程序
pub fn is_bonk_program(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", BONK_PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", BONK_PROGRAM_ID)) ||
    log.contains("bonk") || // 简单的关键词检测
    log.contains("Bonk")
}

/// 从日志中提取 Program data
pub fn extract_program_data(log: &str) -> Option<Vec<u8>> {
    if let Some(data_start) = log.find("Program data: ") {
        let data_part = &log[data_start + 14..];
        base64::decode(data_part.trim()).ok()
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
    block_time: Option<Timestamp>,
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
        block_time_ms: block_time.as_ref().map(|ts| ts.seconds * 1000 + ts.nanos as i64 / 1_000_000),
        program_id,
        outer_index: 0,
        inner_index: None,
        transaction_index: None,
        recv_us: current_time,
        handle_us: current_time,
    }
}


/// 转换为 Bonk 交易事件 - 纯函数
pub fn convert_to_trade_event(
    raw: RawBonkTradeEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> BonkTradeEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_state);

    BonkTradeEvent {
        metadata,
        pool_state: raw.pool_state,
        user: raw.user,
        amount_in: raw.amount_in,
        amount_out: raw.amount_out,
        is_buy: raw.is_buy,
        trade_direction: if raw.is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in: raw.exact_in,
    }
}

/// 转换为 Bonk 池创建事件 - 纯函数
pub fn convert_to_pool_create_event(
    raw: RawBonkPoolCreateEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> BonkPoolCreateEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.pool_state);

    BonkPoolCreateEvent {
        metadata,
        pool_state: raw.pool_state,
        token_a_mint: raw.token_a_mint,
        token_b_mint: raw.token_b_mint,
        creator: raw.creator,
        initial_liquidity_a: raw.initial_liquidity_a,
        initial_liquidity_b: raw.initial_liquidity_b,
    }
}

/// 转换为 Bonk AMM 迁移事件 - 纯函数
pub fn convert_to_migrate_amm_event(
    raw: RawBonkMigrateAmmEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> BonkMigrateAmmEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.old_pool);

    BonkMigrateAmmEvent {
        metadata,
        old_pool: raw.old_pool,
        new_pool: raw.new_pool,
        user: raw.user,
        liquidity_amount: raw.liquidity_amount,
    }
}

/// 解析所有 Bonk 事件 - 单次循环返回统一事件数组
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    // 只循环一次logs！
    for log in logs {
        if !is_bonk_program(log) {
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
                discriminators::TRADE => {
                    if let Some(raw) = parse_raw_event::<RawBonkTradeEvent>(&program_data, discriminators::TRADE) {
                        let event = convert_to_trade_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::BonkTrade(event));
                    }
                }
                discriminators::POOL_CREATE => {
                    if let Some(raw) = parse_raw_event::<RawBonkPoolCreateEvent>(&program_data, discriminators::POOL_CREATE) {
                        let event = convert_to_pool_create_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::BonkPoolCreate(event));
                    }
                }
                discriminators::MIGRATE_AMM => {
                    if let Some(raw) = parse_raw_event::<RawBonkMigrateAmmEvent>(&program_data, discriminators::MIGRATE_AMM) {
                        let event = convert_to_migrate_amm_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::BonkMigrateAmm(event));
                    }
                }
                _ => {
                    // 不是Bonk的事件，跳过
                    continue;
                }
            }
        }
    }

    events
}

/// 简单的模拟解析器 - 用于演示，实际使用时需要根据真实的 Bonk 合约数据解析
pub fn parse_simple_bonk_event(
    log: &str,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Option<BonkTradeEvent> {
    // 这是一个简化的示例解析器
    if log.contains("bonk") && log.contains("trade") {
        let metadata = create_event_metadata(signature, slot, block_time, Pubkey::default());

        return Some(BonkTradeEvent {
            metadata,
            pool_state: Pubkey::default(),
            user: Pubkey::default(),
            amount_in: 1000000, // 示例数据
            amount_out: 950000, // 示例数据，5% 滑点
            is_buy: true,
            trade_direction: TradeDirection::Buy,
            exact_in: true,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bonk_program_detection() {
        let log1 = format!("Program {} invoke [1]", BONK_PROGRAM_ID);
        let log2 = format!("Program {} success", BONK_PROGRAM_ID);
        let log3 = "Program other_program invoke [1]";
        let log4 = "This log contains bonk trading";

        assert!(is_bonk_program(&log1));
        assert!(is_bonk_program(&log2));
        assert!(!is_bonk_program(log3));
        assert!(is_bonk_program(log4));
    }

    #[test]
    fn test_simple_bonk_parsing() {
        let log = "Some bonk trade happened here";
        let event = parse_simple_bonk_event(
            log,
            Signature::default(),
            123,
            None,
        );

        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.amount_in, 1000000);
        assert_eq!(event.amount_out, 950000);
        assert!(event.is_buy);
        assert!(event.exact_in);
    }
}