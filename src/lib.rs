// 核心模块 - 扁平化结构
pub mod common;
pub mod parser;
// pub mod grpc;
// pub mod protos;
// pub mod shred;
// pub mod shred_stream;
pub mod utils;

// 兼容性模块 - 保持向后兼容
pub mod streaming {
    //! 兼容性模块 - 逐步迁移到新架构
    pub use crate::parser as event_parser;
    pub use crate::common::*;
    // pub use crate::grpc::*;
    // pub use crate::shred::*;
}

// 重新导出主要API - 包含指令解析和日志解析
pub use parser::{
    // 基础接口
    SimpleEventParser, SimpleEventListener, DexEvent, EventDispatcher,
    // 指令解析器
    pumpfun_ix, bonk_ix, pumpswap_ix, raydium_clmm_ix, raydium_cpmm_ix,
    // DEX解析器模块
    pumpfun, pumpfun_logs_parser, bonk, pumpswap, raydium_clmm, raydium_cpmm,
    // 通用解析器
    instruction_parser, log_parser, unified_parser,
};
// pub use grpc::{YellowstoneGrpc, SystemEvent, TransferInfo};
