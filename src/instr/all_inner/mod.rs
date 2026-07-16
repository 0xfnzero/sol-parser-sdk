//! 所有协议的 Inner Instruction 解析器统一入口
//!
//! 采用简洁高效的实现方式，所有协议共享通用工具函数
#![allow(unused_imports)]
//!
//! ## 解析器插件系统
//!
//! 所有协议支持两种可插拔的解析器实现：
//!
//! ### 1. Borsh 反序列化解析器（默认，推荐）
//! - **启用**: `cargo build --features parse-borsh` （默认）
//! - **优点**: 类型安全、代码简洁、易维护、自动验证
//! - **适用**: 一般场景、需要稳定性和可维护性的项目
//!
//! ### 2. 零拷贝解析器（高性能）
//! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
//! - **优点**: 最快、零拷贝、无验证开销、适合超高频场景
//! - **适用**: 性能关键路径、每秒数万次解析的场景

use crate::core::events::*;
use crate::instr::inner_common::*;
use solana_sdk::pubkey::Pubkey;

pub mod meteora_amm;
pub mod meteora_damm;
pub mod meteora_dlmm;
pub mod orca;
pub mod pump_fees;
pub mod raydium_amm;
pub mod raydium_cpmm;
pub mod raydium_launchlab;
