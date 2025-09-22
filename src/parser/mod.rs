//! 优化的事件解析器
//!
//! 设计目标：
//! - 简化架构，移除过度设计
//! - 提升性能，减少动态分配
//! - 解耦DEX实现，便于维护和扩展
//! - 函数式设计，避免复杂的trait系统
//! - 扁平化目录结构，避免过度嵌套

// 核心模块 - 只保留必要的
pub mod events;          // 事件定义
pub mod events_parser;   // 简化的事件解析器
pub mod event_dispatcher;// 简单的事件分发器
pub mod simple_example;  // 使用示例

// DEX 特定的函数式解析器
pub mod pumpfun;         // PumpFun DEX 解析器
pub mod bonk;            // Bonk DEX 解析器
pub mod pumpswap;        // PumpSwap DEX 解析器
// TODO: 添加 Raydium 解析器
// pub mod raydium_clmm;    // Raydium CLMM DEX 解析器
// pub mod raydium_cpmm;    // Raydium CPMM DEX 解析器

// 函数式架构演示
pub mod functional_demo; // 演示函数式DEX解析器的使用

// 性能对比测试
#[cfg(test)]
pub mod performance_comparison; // 对比单次循环vs多次循环的性能

// 重命名验证测试
#[cfg(test)]
pub mod rename_test; // 验证重命名后功能正常

#[cfg(feature = "benchmarks")]
pub mod performance;   // 性能测试

// 注意：移除了以下复杂和重复的模块：
// - unified_parser.rs (复杂的两阶段解析器，被简单的 event_dispatcher 替代)
// - callbacks.rs (复杂回调系统)
// - core.rs (被 simple_parser 替代)
// - parser_*.rs (各DEX独立解析器，已整合到 simple_parser)
// - examples.rs, usage_example.rs (重复示例)
// - benchmarks.rs (重复基准测试)
// - tests.rs (使用旧回调系统的测试)

// 主要导出 - 只导出用户需要的简单接口
pub use events::*;  // 所有事件类型定义（包括 DexEvent）
pub use events_parser::{SimpleEventParser, SimpleEventListener, DexEvent};
pub use event_dispatcher::EventDispatcher;
pub use simple_example::{SimpleTradeMonitor, TradeStats, TokenInfo};