// 公用模块 - 简化的通用功能
pub mod config;
pub mod metrics;
pub mod constants;
pub mod subscription;
pub mod simd_utils;

// 重新导出主要类型
pub use config::*;
pub use metrics::*;
pub use constants::*;
pub use subscription::*;
pub use simd_utils::*;

// 常用类型别名
pub type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;