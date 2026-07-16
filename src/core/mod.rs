//! Solana DEX 事件解析器核心模块
//!
//! 提供纯函数式的 DEX 事件解析能力，支持：
//! - PumpFun、RaydiumLaunchlab、PumpSwap、Raydium CLMM/CPMM
//! - 指令+日志数据的智能合并
//! - 零拷贝、高性能解析
//! - 统一的事件格式

// 核心模块
pub mod account_dispatcher; // 账户填充调度器 - 主入口，路由到各协议
pub mod account_fillers; // 账户填充器实现 - 按协议拆分的具体实现
pub mod cache;
pub mod clock; // 高性能时钟 - 微秒级时间戳获取
pub mod common_filler;
pub mod events; // 事件定义
pub mod merger; // 事件合并器 - instruction + inner instruction
pub mod pumpfun_fee_enrich; // 同 tx Pump 后处理：CreateV2 fee 回填、Create→Trade cashback/mayhem（零 RPC）
pub mod unified_parser; // 统一解析器 - 单一入口 // 解析器缓存 - 减少内存分配

// 主要导出 - 核心事件处理功能
pub use cache::{build_account_pubkeys_with_cache, AccountPubkeyCache};
pub use clock::{elapsed_micros_since, now_micros, now_nanos};
pub use events::*;
pub use unified_parser::{
    parse_logs_only, parse_logs_streaming, parse_transaction_events,
    parse_transaction_events_streaming, parse_transaction_with_listener,
    parse_transaction_with_streaming_listener, EventListener, StreamingEventListener,
};

pub use crate::accounts::{
    is_nonce_account, parse_account_unified, parse_nonce_account, parse_token_account, AccountData,
};

// 兼容性类型
pub type ParsedEvent = DexEvent;
