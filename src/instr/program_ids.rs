//! Centralized Program ID Constants
//!
//! This module contains optimized Pubkey constants for all DEX protocols.
//! Using Pubkey constants instead of string constants allows for direct
//! comparison without expensive string conversion operations.

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

/// PumpFun program ID as Pubkey constant
pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

/// Raydium LaunchLab program ID as Pubkey constant.
///
/// Parser routes the Raydium LaunchLab IDL (`idls/raydium_launchpad.json`).
pub const RAYDIUM_LAUNCHLAB_PROGRAM_ID: Pubkey =
    pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");

/// PumpSwap program ID as Pubkey constant
pub const PUMPSWAP_PROGRAM_ID: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");

/// Pump Fees (`pump_fees`) program — 见 `idls/pump_fees.json`
pub const PUMP_FEES_PROGRAM_ID: Pubkey = pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ");

/// Raydium CLMM program ID as Pubkey constant
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");

/// Raydium CPMM program ID as Pubkey constant
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

/// Raydium AMM V4 program ID as Pubkey constant
pub const RAYDIUM_AMM_V4_PROGRAM_ID: Pubkey =
    pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

/// Orca Whirlpool program ID as Pubkey constant
pub const ORCA_WHIRLPOOL_PROGRAM_ID: Pubkey =
    pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

/// Meteora Pools program ID as Pubkey constant
pub const METEORA_POOLS_PROGRAM_ID: Pubkey =
    pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");

/// Meteora DAMM V2 program ID as Pubkey constant
pub const METEORA_DAMM_V2_PROGRAM_ID: Pubkey =
    pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

/// Meteora DLMM program ID as Pubkey constant
pub const METEORA_DLMM_PROGRAM_ID: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");

/// Meteora DBC program ID as Pubkey constant
pub const METEORA_DBC_PROGRAM_ID: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
