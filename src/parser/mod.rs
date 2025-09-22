//! 简化的事件解析器模块
//!
//! 每个DEX使用清晰的两文件架构：
//! - dex_ix_parser.rs - 指令解析器（主解析器，会调用日志解析器）
//! - dex_logs_parser.rs - 日志解析器（被指令解析器调用）
//! - 最终事件数据是两者结合的结果

// 核心模块
pub mod events;          // 事件定义
pub mod events_parser;   // 简化的事件解析器
pub mod event_dispatcher;// 简单的事件分发器

// PumpFun DEX 解析器
pub mod pumpfun_ix_parser;    // PumpFun 指令解析器（主解析器）
pub mod pumpfun_logs_parser;  // PumpFun 日志解析器

// Bonk DEX 解析器
pub mod bonk_ix_parser;       // Bonk 指令解析器（主解析器）
pub mod bonk_logs_parser;     // Bonk 日志解析器

// PumpSwap DEX 解析器
pub mod pumpswap_ix_parser;   // PumpSwap 指令解析器（主解析器）
pub mod pumpswap_logs_parser; // PumpSwap 日志解析器

// Raydium CLMM DEX 解析器
pub mod raydium_clmm_ix_parser;   // Raydium CLMM 指令解析器（主解析器）
pub mod raydium_clmm_logs_parser; // Raydium CLMM 日志解析器

// Raydium CPMM DEX 解析器
pub mod raydium_cpmm_ix_parser;   // Raydium CPMM 指令解析器（主解析器）
pub mod raydium_cpmm_logs_parser; // Raydium CPMM 日志解析器

// 其他现有模块（保持兼容性）
pub mod instruction_parser;
pub mod log_parser;
pub mod unified_parser;
// 保留原有文件以确保兼容性
pub mod pumpfun;
pub mod bonk;
pub mod pumpswap;
pub mod raydium_clmm;
pub mod raydium_cpmm;

// 主要导出
pub use events::*;
pub use events_parser::{SimpleEventParser, SimpleEventListener, DexEvent};
pub use event_dispatcher::EventDispatcher;

// 兼容性类型
pub type ParsedEvent = DexEvent;

// DEX解析器导出 - 使用新的命名
pub use pumpfun_ix_parser as pumpfun_ix;
pub use bonk_ix_parser as bonk_ix;
pub use pumpswap_ix_parser as pumpswap_ix;
pub use raydium_clmm_ix_parser as raydium_clmm_ix;
pub use raydium_cpmm_ix_parser as raydium_cpmm_ix;