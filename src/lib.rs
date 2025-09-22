// 核心模块 - 扁平化结构
pub mod common;
pub mod core;
pub mod instr;    // 指令解析器
pub mod logs;     // 日志解析器
// pub mod grpc;
// pub mod protos;
// pub mod shred;
// pub mod shred_stream;
pub mod utils;

// 兼容性模块 - 保持向后兼容
pub mod streaming {
    //! 兼容性模块 - 逐步迁移到新架构
    pub use crate::core as event_parser;
    pub use crate::common::*;
    // pub use crate::grpc::*;
    // pub use crate::shred::*;
}

// 兼容性别名
pub mod parser {
    pub use crate::core::*;
}

// 重新导出主要API - 包含指令解析和日志解析
pub use core::{
    // 基础接口
    SimpleEventParser, SimpleEventListener, DexEvent, EventDispatcher,
    // 事件合并器
    event_mergers,
    // 通用解析器
    unified_parser,
};

// 解析器模块已经通过 pub mod 声明，可以直接使用
// pub use grpc::{YellowstoneGrpc, SystemEvent, TransferInfo};
