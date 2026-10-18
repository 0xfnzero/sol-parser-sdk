//! ShredStream 模块 - Jito ShredStream 超低延迟交易订阅
//!
//! 提供从 Jito ShredStream 直接订阅 Solana Entry 数据的能力，
//! 相比 gRPC 订阅具有更低的延迟（约 50-100ms 优势）。
//!
//! 实现拆分：`client` 负责网络与队列；`pump_ix` 为 **DEX 外层**指令热路径：
//! Pump.fun 使用专用解析，其它已支持池子协议复用 `instr::parse_instruction_unified`；
//!
//! ## 使用示例
//! ```rust,no_run
//! use sol_parser_sdk::shredstream::{ShredStreamClient, ShredStreamConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let client = ShredStreamClient::new("http://localhost:10800").await?;
//!
//!     // 订阅并获取事件队列
//!     let queue = client.subscribe().await?;
//!
//!     // 消费事件
//!     loop {
//!         if let Some(event) = queue.pop() {
//!             println!("Received: {:?}", event);
//!         } else {
//!             std::hint::spin_loop();
//!         }
//!     }
//! }
//! ```
//!
//! ## 事件过滤
//! ShredStream proxy 本身推送完整 entry 流，不能像 Yellowstone gRPC 一样把交易账户过滤条件下发给服务端。
//! SDK 仍支持本地热路径事件过滤：使用 [`ShredStreamClient::subscribe_with_filter`] 或
//! [`ShredStreamClient::subscribe_with_filter_callback`]，只解析/回调指定的 SDK 事件类型。
//!
//! ```rust,no_run
//! use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
//! use sol_parser_sdk::shredstream::ShredStreamClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let client = ShredStreamClient::new("http://localhost:10800").await?;
//!     let filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
//!     let queue = client.subscribe_with_filter(Some(filter)).await?;
//!
//!     loop {
//!         if let Some(event) = queue.pop() {
//!             println!("Received: {:?}", event);
//!         } else {
//!             tokio::task::yield_now().await;
//!         }
//!     }
//! }
//! ```
//!
//! ## 限制说明
//! ShredStream 相比 gRPC 订阅有以下限制：
//! - 默认跳过未解析的 ALT 指令账户；通过缓存 resolver 或 loaded-addresses API 补齐。显式 best-effort API 保留旧版占位行为。
//! - 官方多跳与已知 wrapper 输出 `PumpRouteIntent`，不是实际成交；任意 CPI 结果仍需执行元数据。
//! - 无 block_time，恒为 0
//! - tx_index 在单个 ShredStream payload 内递增，不保证等同于完整 slot 全局交易索引

pub mod client;
pub mod config;
pub mod proto;
pub(crate) mod pump_ix;

pub use client::{AddressLookupResolver, ShredStreamClient};
pub use config::ShredStreamConfig;
pub use pump_ix::{
    parse_transaction_dex_events, parse_transaction_dex_events_best_effort,
    parse_transaction_dex_events_with_filter, parse_transaction_dex_events_with_loaded_addresses,
};
