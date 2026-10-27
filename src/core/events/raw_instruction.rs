//! Lossless outer instruction payload, without assumptions about program behavior.
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Original outer instruction associated with a DEX trade callback. Account
/// order and duplicates are preserved; unresolved ALT addresses remain None.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInstructionPayload {
    pub program_id: Pubkey,
    pub instruction_index: usize,
    pub accounts: Vec<Option<Pubkey>>,
    pub account_indices: Vec<u8>,
    pub instruction_data: Vec<u8>,
}

/// Stable DEX identifiers retained for stock-price configuration compatibility.
/// These identifiers do not trigger raw-instruction parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RawInstructionDexType {
    #[serde(rename = "pumpfun")]
    PumpFun,
    #[serde(rename = "pumpswap")]
    PumpSwap,
    #[serde(rename = "raydium_launchlab")]
    RaydiumLaunchlab,
    #[serde(rename = "raydium_cpmm")]
    RaydiumCpmm,
    #[serde(rename = "raydium_clmm")]
    RaydiumClmm,
    #[serde(rename = "raydium_amm_v4")]
    RaydiumAmmV4,
    #[serde(rename = "orca_whirlpool")]
    OrcaWhirlpool,
    #[serde(rename = "meteora_pools")]
    MeteoraPools,
    #[serde(rename = "meteora_damm_v2")]
    MeteoraDammV2,
    #[serde(rename = "meteora_dlmm")]
    MeteoraDlmm,
    #[serde(rename = "meteora_dbc")]
    MeteoraDbc,
}

impl RawInstructionDexType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PumpFun => "pumpfun",
            Self::PumpSwap => "pumpswap",
            Self::RaydiumLaunchlab => "raydium_launchlab",
            Self::RaydiumCpmm => "raydium_cpmm",
            Self::RaydiumClmm => "raydium_clmm",
            Self::RaydiumAmmV4 => "raydium_amm_v4",
            Self::OrcaWhirlpool => "orca_whirlpool",
            Self::MeteoraPools => "meteora_pools",
            Self::MeteoraDammV2 => "meteora_damm_v2",
            Self::MeteoraDlmm => "meteora_dlmm",
            Self::MeteoraDbc => "meteora_dbc",
        }
    }
}
