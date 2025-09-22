// 核心模块 - 扁平化结构
pub mod common;
pub mod parser;
pub mod grpc;
pub mod protos;
pub mod shred;
pub mod shred_stream;
pub mod utils;

// 兼容性模块 - 保持向后兼容
pub mod streaming {
    //! 兼容性模块 - 逐步迁移到新架构
    pub use crate::parser as event_parser;
    pub use crate::common::*;
    pub use crate::grpc::*;
    pub use crate::shred::*;
}

// 重新导出主要API - 只导出简化的接口
pub use parser::{SimpleEventParser, SimpleEventListener, ParsedEvent, SimpleTradeMonitor};
pub use grpc::{YellowstoneGrpc, SystemEvent, TransferInfo};
