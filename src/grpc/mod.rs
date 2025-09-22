//! gRPC 模块 - 支持gRPC订阅、事件过滤、账号过滤
//!
//! 这个模块提供了完整的Solana DEX事件gRPC流式处理功能，包括：
//! - gRPC连接和订阅管理
//! - 事件类型过滤
//! - 账户和交易过滤
//! - 多协议支持（PumpFun, Bonk, Raydium等）

pub mod client;
pub mod types;
pub mod config;
pub mod filter;
pub mod program_ids;
pub mod event_parser;

// 重新导出主要API，保持兼容性
pub use client::YellowstoneGrpc;
pub use types::{ClientConfig, Protocol, EventType as StreamingEventType, TransactionFilter, AccountFilter, EventTypeFilter, SlotFilter};

// 事件解析器重新导出
pub use event_parser::*;

// 兼容性别名
pub use StreamingEventType as EventType;