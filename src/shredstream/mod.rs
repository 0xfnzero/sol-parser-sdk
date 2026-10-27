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
//! - 已确认程序 ID 的 Pump.fun 外层指令可在 ALT 未展开时回调，缺失账户为默认值。
//!   其它 DEX 仍要求账户完整；通过缓存 resolver 或 loaded-addresses API 补齐。
//!   默认入口不猜测未知程序，显式 best-effort API 保留旧版猜测行为。
//! - 第三方指令自身账户引用 Pump.fun 时输出 `PumpFunTrade` 候选，
//!   通过 `third_program` 透传原始输入，不推断内部 CPI；其它未知指令不产生事件。
//! - 原始事件保留账户顺序、重复项和 data；未解析 ALT 账户为 None，并保留原始索引。
//! - 无 block_time，恒为 0
//! - tx_index 在单个 ShredStream payload 内递增，不保证等同于完整 slot 全局交易索引

pub mod client;
pub mod config;
pub mod proto;
pub mod pump_ix;
pub(crate) mod pump_multi_hop;

pub use client::{AddressLookupResolver, ShredStreamClient};
pub use config::ShredStreamConfig;
pub use pump_ix::{
    parse_transaction_dex_events, parse_transaction_dex_events_best_effort,
    parse_transaction_dex_events_with_filter, parse_transaction_dex_events_with_loaded_addresses,
};
