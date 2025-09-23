// 核心模块 - 扁平化结构
pub mod common;
pub mod core;
pub mod instr;    // 指令解析器
pub mod logs;     // 日志解析器
pub mod merge;    // 事件合并器
// 已合并到 streaming 模块中
pub mod utils;

// gRPC 模块 - 支持gRPC订阅和过滤
pub mod grpc;

// 兼容性别名 - 重新导出 grpc 为 streaming
pub use grpc as streaming;

// 兼容性别名
pub mod parser {
    pub use crate::core::*;
}

// 重新导出主要API - 简化的单一入口解析器
pub use core::{
    // 事件类型
    DexEvent, EventMetadata, ParsedEvent,
    // 主要解析函数
    parse_transaction_events, parse_logs_only, parse_transaction_with_listener,
    // 流式解析函数
    parse_transaction_events_streaming, parse_logs_streaming, parse_transaction_with_streaming_listener,
    // 事件监听器
    EventListener, StreamingEventListener,
    // 事件合并器
    merge_instruction_and_log_events,
};
