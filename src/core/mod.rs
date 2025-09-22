//! Solana DEX 事件解析器核心模块
//!
//! 提供纯函数式的 DEX 事件解析能力，支持：
//! - PumpFun、Bonk、PumpSwap、Raydium CLMM/CPMM
//! - 指令+日志数据的智能合并
//! - 零拷贝、高性能解析
//! - 统一的事件格式

// 核心模块
pub mod events;          // 事件定义
pub mod unified_parser;  // 统一解析器 - 单一入口
pub mod event_mergers;   // 事件合并器

// 主要导出 - 核心事件处理功能
pub use events::*;
pub use unified_parser::{
    parse_transaction_events, parse_logs_only, parse_transaction_with_listener, EventListener
};
pub use event_mergers::merge_instruction_and_log_events;

// 兼容性类型
pub type ParsedEvent = DexEvent;