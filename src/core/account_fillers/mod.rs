//! 账户填充模块 - 按协议拆分
//!
//! 每个协议有独立的填充模块，便于维护和扩展

pub mod meteora;
pub mod orca;
pub mod pumpfun;
pub mod pumpswap;
pub mod raydium;
pub mod raydium_launchlab;

use solana_sdk::pubkey::Pubkey;

/// 账户获取辅助函数类型
pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;
