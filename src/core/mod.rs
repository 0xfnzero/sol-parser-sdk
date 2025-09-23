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
pub mod account_filler;  // 账户填充器 - 从指令数据填充事件账户

// 主要导出 - 核心事件处理功能
pub use events::*;
pub use unified_parser::{
    parse_transaction_events, parse_logs_only, parse_transaction_with_listener, EventListener,
    parse_transaction_events_streaming, parse_logs_streaming, parse_transaction_with_streaming_listener, StreamingEventListener
};

// 导出事件合并功能
pub use crate::merge::merge_instruction_and_log_events;

// 兼容性类型
pub type ParsedEvent = DexEvent;