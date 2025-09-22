//! 简化的事件解析器 - 函数式设计
//! 使用新的事件分发器替代复杂的统一解析器
//!
//! 这个模块提供了完整的 DEX 事件解析功能：
//! - 支持多个 DEX (PumpFun, Bonk, PumpSwap, Raydium CLMM/CPMM)
//! - 纯函数式设计，高性能解析
//! - 统一的 DexEvent 枚举和回调接口

use crate::parser::events::*;
use crate::parser::event_dispatcher::EventDispatcher;
use crate::parser::{pumpfun, bonk, pumpswap};
use prost_types::Timestamp;
use solana_sdk::signature::Signature;

/// 使用统一的 DexEvent 枚举 - 已定义在 events.rs 中
pub use crate::parser::events::DexEvent;

/// DEX 程序 ID 常量
pub mod program_ids {
    pub const PUMPFUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
    pub const BONK: &str = "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1";
    pub const PUMPSWAP: &str = "PumpSWaP7evteam3bP1234567890123456789012345";
    pub const RAYDIUM_AMM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    pub const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
    pub const RAYDIUM_CPMM: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
}

/// DEX 类型识别 - 纯函数
pub fn identify_dex_from_logs(logs: &[String]) -> Option<&'static str> {
    for log in logs {
        if pumpfun::is_pumpfun_program(log) {
            return Some(program_ids::PUMPFUN);
        }
        if bonk::is_bonk_program(log) {
            return Some(program_ids::BONK);
        }
        if pumpswap::is_pumpswap_program(log) {
            return Some(program_ids::PUMPSWAP);
        }
        // TODO: 添加 Raydium CLMM 和 CPMM 解析器后，启用这些检查
        // if raydium_clmm::is_raydium_clmm_program(log) {
        //     return Some(program_ids::RAYDIUM_CLMM);
        // }
        // if raydium_cpmm::is_raydium_cpmm_program(log) {
        //     return Some(program_ids::RAYDIUM_CPMM);
        // }
        if log.contains(&format!("Program {} invoke", program_ids::RAYDIUM_AMM)) ||
           log.contains(&format!("Program {} success", program_ids::RAYDIUM_AMM)) {
            return Some(program_ids::RAYDIUM_AMM);
        }
    }
    None
}

/// 简化的事件解析器 - 使用事件分发器
pub struct SimpleEventParser;

impl SimpleEventParser {
    /// 优化的主入口 - 单次循环解析所有事件！🚀
    pub fn dispatch_dex_parsing(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
    ) -> Vec<DexEvent> {
        // 使用简化的事件分发器
        EventDispatcher::parse_all_dex_events(logs, signature, slot, block_time)
    }

    /// 根据程序 ID 解析特定 DEX 事件
    pub fn parse_by_program_id(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
        program_id: &str,
    ) -> Vec<DexEvent> {
        EventDispatcher::parse_by_program_id(logs, signature, slot, block_time, program_id)
    }

    /// 计算代币价格 (以 SOL 为单位) - 纯函数
    pub fn calculate_token_price_in_sol(event: &PumpFunTradeEvent) -> Option<f64> {
        pumpfun::calculate_token_price_in_sol(event)
    }

    /// 判断是否是大额交易 - 纯函数
    pub fn is_large_trade(event: &PumpFunTradeEvent) -> bool {
        pumpfun::is_large_trade(event)
    }

    /// 获取当前代币的市值 - 纯函数
    pub fn get_market_cap_in_sol(event: &PumpFunTradeEvent) -> f64 {
        pumpfun::get_market_cap_in_sol(event)
    }

    /// 计算 PumpSwap 价格影响 - 纯函数
    pub fn calculate_pumpswap_price_impact(event: &PumpSwapBuyEvent) -> f64 {
        pumpswap::calculate_price_impact(event)
    }

    /// 判断是否是大额 PumpSwap 交易 - 纯函数
    pub fn is_large_pumpswap_trade(sol_amount: u64) -> bool {
        pumpswap::is_large_pumpswap_trade(sol_amount)
    }

    // TODO: 添加 Raydium CLMM 和 CPMM 解析器后，启用这些函数
    // /// 计算 Raydium CLMM 价格 - 纯函数
    // pub fn calculate_clmm_price(sqrt_price_x64: u128) -> f64 {
    //     raydium_clmm::calculate_price_from_sqrt_price(sqrt_price_x64)
    // }

    // /// 计算 Raydium CPMM 价格 - 纯函数
    // pub fn calculate_cpmm_price(pool_token_0_amount: u64, pool_token_1_amount: u64) -> f64 {
    //     raydium_cpmm::calculate_cpmm_price(pool_token_0_amount, pool_token_1_amount)
    // }

    /// 主要的解析入口 - 使用调度器模式
    pub fn parse_all_events_from_logs(
        logs: &[String],
        signature: Signature,
        slot: u64,
        block_time: Option<Timestamp>,
    ) -> Vec<DexEvent> {
        Self::dispatch_dex_parsing(logs, signature, slot, block_time)
    }
}

/// 统一的事件回调类型 - 使用 DexEvent
pub type EventCallback = Box<dyn Fn(&DexEvent) + Send + Sync>;

/// 简单的事件监听器 - 使用统一的回调接口
pub struct SimpleEventListener {
    callbacks: Vec<EventCallback>,
}

impl SimpleEventListener {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// 添加统一的事件回调 - 用户通过 match 判断事件类型
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(&DexEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// 处理单个事件并触发所有回调
    pub fn handle_event(&self, event: &DexEvent) {
        for callback in &self.callbacks {
            callback(event);
        }
    }

    /// 处理多个事件并触发回调
    pub fn handle_events(&self, events: Vec<DexEvent>) {
        for event in &events {
            self.handle_event(event);
        }
    }

    /// 从日志处理所有事件 - 使用函数式调度器
    pub fn process_logs(&self, logs: &[String], signature: Signature, slot: u64, block_time: Option<Timestamp>) {
        let all_events = SimpleEventParser::dispatch_dex_parsing(logs, signature, slot, block_time);
        self.handle_events(all_events);
    }
}

impl Default for SimpleEventListener {
    fn default() -> Self {
        Self::new()
    }
}

/// 使用示例 - 统一回调接口
pub fn example_usage() {
    use solana_sdk::signature::Signature;
    use std::str::FromStr;

    // 创建事件监听器
    let mut listener = SimpleEventListener::new();

    // 添加统一的事件回调 - 用户通过 match 判断事件类型
    listener.add_callback(|event| {
        match event {
            DexEvent::PumpFunCreate(create_event) => {
                println!("🎉 新代币创建:");
                println!("  名称: {}", create_event.name);
                println!("  符号: {}", create_event.symbol);
                println!("  铸造地址: {}", create_event.mint);
                println!("  创建者: {}", create_event.creator);
                println!("  虚拟储备: {} SOL", create_event.virtual_sol_reserves as f64 / 1e9);
            }
            DexEvent::PumpFunTrade(trade_event) => {
                let action = if trade_event.is_buy { "买入" } else { "卖出" };
                let sol_amount = trade_event.sol_amount as f64 / 1_000_000_000.0;
                let price = pumpfun::calculate_token_price_in_sol(trade_event)
                    .unwrap_or(0.0);

                println!("💰 PumpFun 交易:");
                println!("  操作: {}", action);
                println!("  SOL 数量: {:.4}", sol_amount);
                println!("  代币数量: {}", trade_event.token_amount);
                println!("  价格: {:.10} SOL/Token", price);
                println!("  用户: {}", trade_event.user);
                println!("  储备变化: {} SOL / {} Token",
                    trade_event.virtual_sol_reserves as f64 / 1e9,
                    trade_event.virtual_token_reserves as f64 / 1e9);

                if pumpfun::is_large_trade(trade_event) {
                    println!("  🚨 大额交易警告!");
                }
            }
            DexEvent::RaydiumClmmSwap(clmm_event) => {
                println!("🔄 Raydium CLMM 交换");
                println!("  金额: {}", clmm_event.amount);
                println!("  用户: {}", clmm_event.user);
            }
            DexEvent::RaydiumCpmmSwap(cpmm_event) => {
                println!("🌊 Raydium CPMM 交换");
                println!("  输入: {} / 输出: {}", cpmm_event.amount_in, cpmm_event.amount_out);
                println!("  用户: {}", cpmm_event.user);
            }
            _ => {
                // 其他事件类型的处理
                println!("🔍 其他事件: {:?}", event);
            }
        }
    });

    // 示例数据
    let signature = Signature::from_str("5VfYmGC5zb9JBKK5Y5uHjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjKjK").unwrap_or_default();
    let slot = 12345;
    let block_time = None;

    let example_logs = vec![
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK invoke [1]".to_string(),
        "Program CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C invoke [1]".to_string(),
    ];

    // 处理日志
    listener.process_logs(&example_logs, signature, slot, block_time);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_identification() {
        // 测试 PumpFun 程序识别
        let pumpfun_logs = vec![
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        ];
        assert_eq!(identify_dex_from_logs(&pumpfun_logs), Some(program_ids::PUMPFUN));

        // 测试 Raydium CLMM 程序识别
        let clmm_logs = vec![
            "Program CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK invoke [1]".to_string(),
        ];
        assert_eq!(identify_dex_from_logs(&clmm_logs), Some(program_ids::RAYDIUM_CLMM));

        // 测试 Raydium CPMM 程序识别
        let cpmm_logs = vec![
            "Program CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C invoke [1]".to_string(),
        ];
        assert_eq!(identify_dex_from_logs(&cpmm_logs), Some(program_ids::RAYDIUM_CPMM));

        // 测试未知程序
        let unknown_logs = vec![
            "Program unknown_program invoke [1]".to_string(),
        ];
        assert_eq!(identify_dex_from_logs(&unknown_logs), None);
    }

    #[test]
    fn test_dispatcher_routing() {
        let signature = Signature::default();
        let slot = 123u64;
        let block_time = None;

        // 测试 PumpFun 路由
        let pumpfun_logs = vec![
            "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]".to_string(),
        ];
        let events = SimpleEventParser::dispatch_dex_parsing(
            &pumpfun_logs, signature, slot, block_time.clone()
        );
        // PumpFun事件会被正确路由（即使没有实际数据也会尝试解析）

        // 测试 Raydium CLMM 路由
        let clmm_logs = vec![
            "Program CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK invoke [1]".to_string(),
        ];
        let events = SimpleEventParser::dispatch_dex_parsing(
            &clmm_logs, signature, slot, block_time.clone()
        );

        // 测试未知程序
        let unknown_logs = vec![
            "Program unknown_program_id invoke [1]".to_string(),
        ];
        let events = SimpleEventParser::dispatch_dex_parsing(
            &unknown_logs, signature, slot, block_time
        );
        // 未知程序会尝试所有解析器，但不会产生有效事件
    }

    #[test]
    fn test_event_listener() {
        let mut listener = SimpleEventListener::new();

        // 添加回调收集事件
        listener.add_callback(|event| {
            match event {
                DexEvent::PumpFunCreate(_) => println!("收到PumpFun创建事件"),
                DexEvent::PumpFunTrade(_) => println!("收到PumpFun交易事件"),
                DexEvent::RaydiumClmmSwap(_) => println!("收到Raydium CLMM交换事件"),
                DexEvent::RaydiumCpmmSwap(_) => println!("收到Raydium CPMM交换事件"),
                _ => println!("收到其他事件"),
            }
        });

        // 测试日志处理
        let logs = vec!["Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success".to_string()];
        let signature = Signature::default();
        let slot = 123;
        let block_time = None;
        listener.process_logs(&logs, signature, slot, block_time);
    }
}