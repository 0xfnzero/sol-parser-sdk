//! gRPC 模块 - 支持gRPC订阅、事件过滤、账号过滤
//!
//! 这个模块提供了完整的Solana DEX事件gRPC流式处理功能，包括：
//! - gRPC连接和订阅管理
//! - 事件类型过滤
//! - 账户和交易过滤
//! - 多协议支持（PumpFun, RaydiumLaunchlab, Raydium等）
//! - [`subscribe_builder`]：构造 Yellowstone `SubscribeRequest`（DEX 与 mentions 监控共用）
//! - [`transaction_meta`]：原始 `Transaction` / `TransactionStatusMeta` 工具（转账分析等）

pub mod buffers;
pub mod client;
pub mod config;
pub mod event_parser;
pub mod filter;
pub mod geyser_connect;
pub mod instruction_parser; // 增强的 instruction 解析器
pub(crate) mod log_instr_dedup;
pub mod program_ids;
pub mod subscribe_builder;
pub mod transaction_meta;
pub mod types;
pub mod yellowstone_tx_parse;

// 重新导出主要API
pub use client::YellowstoneGrpc;
pub use geyser_connect::{connect_yellowstone_geyser, GeyserConnectConfig};
pub use subscribe_builder::{
    build_subscribe_request, build_subscribe_request_with_commitment,
    build_subscribe_request_with_event_filter, build_subscribe_transaction_filters_named,
};
pub use transaction_meta::{
    collect_account_keys_bs58, collect_watch_transfer_counterparty_pairs,
    heuristic_sol_counterparties_for_watched_keys, lamport_balance_deltas,
    spl_token_counterparty_by_owner, token_balance_raw_amount, try_yellowstone_signature,
};
pub use types::{
    account_filter_memcmp, AccountFilter, ClientConfig, EventType as StreamingEventType,
    EventTypeFilter, OrderMode, Protocol, SlotFilter, TransactionFilter,
};
pub use yellowstone_tx_parse::{
    parse_subscribe_update_transaction, parse_subscribe_update_transaction_low_latency,
};

// 事件解析器重新导出
pub use event_parser::*;

// 兼容性别名
pub use StreamingEventType as EventType;
