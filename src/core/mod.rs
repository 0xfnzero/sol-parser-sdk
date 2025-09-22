//! Solana DEX 事件解析器模块
//!
//! 提供纯函数式的 DEX 事件解析能力，支持：
//! - PumpFun、Bonk、PumpSwap、Raydium CLMM/CPMM
//! - 指令+日志数据的智能合并
//! - 零拷贝、高性能解析
//! - 统一的事件格式

// 核心模块
pub mod events;           // 事件定义
pub mod events_parser;    // 事件解析器
pub mod event_dispatcher; // 事件分发器

// 核心功能模块 - 事件合并和分发

// 事件合并器
pub mod event_mergers;

// 高级解析器 - 支持指令+日志合并
pub mod unified_parser;

// 兼容性模块已移除，使用新的模块化结构

// 主要导出 - 核心事件处理功能
pub use events::*;
pub use events_parser::{SimpleEventParser, SimpleEventListener};
pub use event_dispatcher::EventDispatcher;
pub use event_mergers::*;

// 兼容性类型
pub type ParsedEvent = DexEvent;