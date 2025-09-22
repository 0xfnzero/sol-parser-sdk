//! PumpFun DEX 解析器 - 函数式设计
//!
//! 专门解析 PumpFun 相关的事件，包括：
//! - 代币创建事件
//! - 交易事件
//! - 完成事件（毕业到 Raydium）

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use prost_types::Timestamp;
use crate::parser::events::*;

/// PumpFun discriminator 常量
pub mod discriminators {
    pub const CREATE_EVENT: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
    pub const TRADE_EVENT: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
    pub const COMPLETE_EVENT: [u8; 8] = [95, 114, 97, 156, 212, 46, 152, 8];
}

/// PumpFun 程序 ID
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// 原始 PumpFun 创建事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpFunCreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
}

/// 原始 PumpFun 交易事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpFunTradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
}

/// 原始 PumpFun 完成事件数据结构
#[derive(BorshDeserialize)]
pub struct RawPumpFunCompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

/// 检查日志是否来自 PumpFun 程序
pub fn is_pumpfun_program(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PUMPFUN_PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PUMPFUN_PROGRAM_ID))
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



/// 转换为 PumpFun 创建事件 - 纯函数
pub fn convert_to_create_event(
    raw: RawPumpFunCreateEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> PumpFunCreateTokenEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.mint);

    PumpFunCreateTokenEvent {
        metadata,
        name: raw.name,
        symbol: raw.symbol,
        uri: raw.uri,
        mint: raw.mint,
        bonding_curve: raw.bonding_curve,
        user: raw.user,
        creator: raw.creator,
        virtual_token_reserves: raw.virtual_token_reserves,
        virtual_sol_reserves: raw.virtual_sol_reserves,
        real_token_reserves: raw.real_token_reserves,
        token_total_supply: raw.token_total_supply,
    }
}

/// 转换为 PumpFun 交易事件 - 纯函数
pub fn convert_to_trade_event(
    raw: RawPumpFunTradeEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> PumpFunTradeEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.mint);

    PumpFunTradeEvent {
        metadata,
        mint: raw.mint,
        user: raw.user,
        sol_amount: raw.sol_amount,
        token_amount: raw.token_amount,
        is_buy: raw.is_buy,
        bonding_curve: Pubkey::default(), // 可以从其他数据源获取
        virtual_sol_reserves: raw.virtual_sol_reserves,
        virtual_token_reserves: raw.virtual_token_reserves,
        real_sol_reserves: raw.real_sol_reserves,
        real_token_reserves: raw.real_token_reserves,
    }
}

/// 转换为 PumpFun 完成事件 - 纯函数
pub fn convert_to_complete_event(
    raw: RawPumpFunCompleteEvent,
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> PumpFunCompleteTokenEvent {
    let metadata = create_event_metadata(signature, slot, block_time, raw.mint);

    PumpFunCompleteTokenEvent {
        metadata,
        user: raw.user,
        mint: raw.mint,
        bonding_curve: raw.bonding_curve,
    }
}

/// 解析所有 PumpFun 事件 - 单次循环返回统一事件数组
pub fn parse_all_events(
    logs: &[String],
    signature: Signature,
    slot: u64,
    block_time: Option<Timestamp>,
) -> Vec<DexEvent> {
    let mut events = Vec::new();

    // 只循环一次logs！
    for log in logs {
        if !is_pumpfun_program(log) {
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
                discriminators::CREATE_EVENT => {
                    if let Some(raw) = parse_raw_event::<RawPumpFunCreateEvent>(&program_data, discriminators::CREATE_EVENT) {
                        let event = convert_to_create_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpFunCreate(event));
                    }
                }
                discriminators::TRADE_EVENT => {
                    if let Some(raw) = parse_raw_event::<RawPumpFunTradeEvent>(&program_data, discriminators::TRADE_EVENT) {
                        let event = convert_to_trade_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpFunTrade(event));
                    }
                }
                discriminators::COMPLETE_EVENT => {
                    if let Some(raw) = parse_raw_event::<RawPumpFunCompleteEvent>(&program_data, discriminators::COMPLETE_EVENT) {
                        let event = convert_to_complete_event(raw, signature, slot, block_time.clone());
                        events.push(DexEvent::PumpFunComplete(event));
                    }
                }
                _ => {
                    // 不是PumpFun的事件，跳过
                    continue;
                }
            }
        }
    }

    events
}

/// 计算代币价格 (以 SOL 为单位) - 纯函数
pub fn calculate_token_price_in_sol(event: &PumpFunTradeEvent) -> Option<f64> {
    if event.token_amount == 0 {
        return None;
    }

    let sol_amount = event.sol_amount as f64 / 1_000_000_000.0; // lamports 转 SOL
    let token_amount = event.token_amount as f64;

    Some(sol_amount / token_amount)
}

/// 判断是否是大额交易 (超过1 SOL) - 纯函数
pub fn is_large_trade(event: &PumpFunTradeEvent) -> bool {
    event.sol_amount >= 1_000_000_000 // 1 SOL in lamports
}

/// 获取当前代币的市值 (基于虚拟储备) - 纯函数
pub fn get_market_cap_in_sol(event: &PumpFunTradeEvent) -> f64 {
    if event.virtual_token_reserves == 0 {
        return 0.0;
    }

    let sol_reserves = event.virtual_sol_reserves as f64;
    let token_reserves = event.virtual_token_reserves as f64;
    let total_supply = 1_000_000_000.0; // 假设总供应量是10亿

    (sol_reserves / token_reserves) * total_supply / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pumpfun_program_detection() {
        let log1 = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]";
        let log2 = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success";
        let log3 = "Program other_program invoke [1]";

        assert!(is_pumpfun_program(log1));
        assert!(is_pumpfun_program(log2));
        assert!(!is_pumpfun_program(log3));
    }

    #[test]
    fn test_price_calculation() {
        let metadata = create_event_metadata(
            Signature::default(),
            0,
            None,
            Pubkey::default(),
        );

        let trade_event = PumpFunTradeEvent {
            metadata,
            mint: Pubkey::default(),
            user: Pubkey::default(),
            sol_amount: 1_000_000_000, // 1 SOL
            token_amount: 1_000_000_000, // 1B tokens
            is_buy: true,
            bonding_curve: Pubkey::default(),
            virtual_sol_reserves: 30_000_000_000,
            virtual_token_reserves: 1_073_000_000_000_000,
            real_sol_reserves: 0,
            real_token_reserves: 793_100_000_000_000,
        };

        let price = calculate_token_price_in_sol(&trade_event);
        assert!(price.is_some());
        assert_eq!(price.unwrap(), 1.0 / 1_000_000_000.0);
    }

    #[test]
    fn test_large_trade_detection() {
        let metadata = create_event_metadata(
            Signature::default(),
            0,
            None,
            Pubkey::default(),
        );

        let small_trade = PumpFunTradeEvent {
            metadata: metadata.clone(),
            mint: Pubkey::default(),
            user: Pubkey::default(),
            sol_amount: 500_000_000, // 0.5 SOL
            token_amount: 1_000_000,
            is_buy: true,
            bonding_curve: Pubkey::default(),
            virtual_sol_reserves: 30_000_000_000,
            virtual_token_reserves: 1_073_000_000_000_000,
            real_sol_reserves: 0,
            real_token_reserves: 793_100_000_000_000,
        };

        let large_trade = PumpFunTradeEvent {
            sol_amount: 2_000_000_000, // 2 SOL
            ..small_trade.clone()
        };

        assert!(!is_large_trade(&small_trade));
        assert!(is_large_trade(&large_trade));
    }
}