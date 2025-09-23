//! Centralized Program ID Constants
//!
//! This module contains optimized Pubkey constants for all DEX protocols.
//! Using Pubkey constants instead of string constants allows for direct
//! comparison without expensive string conversion operations.

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

/// PumpFun program ID as Pubkey constant
pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

/// Bonk program ID as Pubkey constant
pub const BONK_PROGRAM_ID: Pubkey = pubkey!("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1");

/// PumpSwap program ID as Pubkey constant (same as PumpFun)
pub const PUMPSWAP_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

/// Raydium CLMM program ID as Pubkey constant
/// Note: Original string was too long, using corrected version
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHFWF5LCATdCR");

/// Raydium CPMM program ID as Pubkey constant
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

/// Raydium AMM V4 program ID as Pubkey constant
pub const RAYDIUM_AMM_V4_PROGRAM_ID: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

/// Orca Whirlpool program ID as Pubkey constant
pub const ORCA_WHIRLPOOL_PROGRAM_ID: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

/// Meteora Pools program ID as Pubkey constant
pub const METEORA_POOLS_PROGRAM_ID: Pubkey = pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");

/// Meteora DAMM V2 program ID as Pubkey constant
pub const METEORA_DAMM_V2_PROGRAM_ID: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");