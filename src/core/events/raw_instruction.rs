//! Lossless outer instruction payload, without assumptions about program behavior.
use super::EventMetadata;
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

impl RawInstructionPayload {
    /// Adapt a callback payload for the optional offline wrapper-rule helpers.
    pub fn with_metadata(&self, metadata: EventMetadata) -> RawInstructionEvent {
        RawInstructionEvent {
            metadata,
            dexes: raw_instruction_dexes(self.program_id, &self.accounts),
            program_id: self.program_id,
            instruction_index: self.instruction_index,
            accounts: self.accounts.clone(),
            account_indices: self.account_indices.clone(),
            instruction_data: self.instruction_data.clone(),
        }
    }
}

impl From<&RawInstructionEvent> for RawInstructionPayload {
    fn from(raw: &RawInstructionEvent) -> Self {
        Self {
            program_id: raw.program_id,
            instruction_index: raw.instruction_index,
            accounts: raw.accounts.clone(),
            account_indices: raw.account_indices.clone(),
            instruction_data: raw.instruction_data.clone(),
        }
    }
}

/// Evidence available to the parser, not a guarantee that every amount is known.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeParseStatus {
    /// Outer instruction references a DEX; no trade or direction is proven.
    Unresolved,
    /// Native instruction or explicitly applied wrapper rule decoded inputs.
    #[default]
    InstructionDecoded,
    /// Successful execution metadata/logs establish the DEX invocation.
    /// Executed amounts may still be unavailable when event logs are missing.
    ExecutionConfirmed,
}

/// Compatibility alias for the original Pump.fun-specific name.
pub type PumpFunParseStatus = TradeParseStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInstructionEvent {
    pub metadata: EventMetadata,
    /// DEX associations from this instruction only, not the transaction-wide key table.
    #[serde(default)]
    pub dexes: Vec<RawInstructionDex>,
    pub program_id: Pubkey,
    pub instruction_index: usize,
    /// Instruction account order, including duplicates. None means an unresolved
    /// ALT address, never the System Program or an invented placeholder.
    pub accounts: Vec<Option<Pubkey>>,
    /// Indices into the transaction's static + loaded account key table.
    pub account_indices: Vec<u8>,
    /// Unmodified bytes, including any program-defined discriminator.
    pub instruction_data: Vec<u8>,
}

/// Stable DEX identifiers shared by every language binding.
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

/// A DEX program directly targeted or referenced by this outer instruction.
/// A reference is not proof of an executed CPI. Unknown wrapper layouts cannot
/// establish a pool address; pool remains None rather than guessing an index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInstructionDex {
    pub dex_type: RawInstructionDexType,
    pub program_id: Pubkey,
    pub pool: Option<Pubkey>,
}

pub fn raw_instruction_dexes(
    program_id: Pubkey,
    accounts: &[Option<Pubkey>],
) -> Vec<RawInstructionDex> {
    use crate::grpc::program_ids::*;
    let known = [
        (RawInstructionDexType::PumpFun, PUMPFUN_PROGRAM),
        (RawInstructionDexType::PumpSwap, PUMPSWAP_PROGRAM),
        (RawInstructionDexType::RaydiumLaunchlab, RAYDIUM_LAUNCHLAB_PROGRAM),
        (RawInstructionDexType::RaydiumCpmm, RAYDIUM_CPMM_PROGRAM),
        (RawInstructionDexType::RaydiumClmm, RAYDIUM_CLMM_PROGRAM),
        (RawInstructionDexType::RaydiumAmmV4, RAYDIUM_AMM_V4_PROGRAM),
        (RawInstructionDexType::OrcaWhirlpool, ORCA_WHIRLPOOL_PROGRAM),
        (RawInstructionDexType::MeteoraPools, METEORA_POOLS_PROGRAM),
        (RawInstructionDexType::MeteoraDammV2, METEORA_DAMM_V2_PROGRAM),
        (RawInstructionDexType::MeteoraDlmm, METEORA_DLMM_PROGRAM),
        (RawInstructionDexType::MeteoraDbc, METEORA_DBC_PROGRAM),
    ];
    known
        .into_iter()
        .filter(|(_, key)| program_id == *key || accounts.contains(&Some(*key)))
        .map(|(name, key)| RawInstructionDex { dex_type: name, program_id: key, pool: None })
        .collect()
}

pub fn raw_instruction_matches_filter(
    dexes: &[RawInstructionDex],
    filter: &crate::grpc::EventTypeFilter,
) -> bool {
    use crate::grpc::EventType;
    if filter.exclude_types.as_ref().is_some_and(|types| types.contains(&EventType::RawInstruction))
    {
        return false;
    }
    if filter.include_only.as_ref().is_some_and(|types| types.contains(&EventType::RawInstruction))
    {
        return true;
    }
    if dexes.is_empty() {
        return filter.include_only.is_none();
    }
    dexes.iter().any(|dex| {
        let event = match dex.dex_type {
            RawInstructionDexType::PumpFun => EventType::PumpFunTrade,
            RawInstructionDexType::PumpSwap => EventType::PumpSwapTrade,
            RawInstructionDexType::RaydiumLaunchlab => EventType::RaydiumLaunchlabTrade,
            RawInstructionDexType::RaydiumCpmm => EventType::RaydiumCpmmSwap,
            RawInstructionDexType::RaydiumClmm => EventType::RaydiumClmmSwap,
            RawInstructionDexType::RaydiumAmmV4 => EventType::RaydiumAmmV4Swap,
            RawInstructionDexType::OrcaWhirlpool => EventType::OrcaWhirlpoolSwap,
            RawInstructionDexType::MeteoraPools => EventType::MeteoraPoolsSwap,
            RawInstructionDexType::MeteoraDammV2 => EventType::MeteoraDammV2Swap,
            RawInstructionDexType::MeteoraDlmm => EventType::MeteoraDlmmSwap,
            RawInstructionDexType::MeteoraDbc => EventType::MeteoraDbcSwap,
        };
        filter.should_include(event)
    })
}

#[cfg(test)]
mod enum_tests {
    use super::*;
    #[test]
    fn dex_identifiers_round_trip_without_changing_json() {
        for (kind, value) in [
            (RawInstructionDexType::PumpFun, "pumpfun"),
            (RawInstructionDexType::PumpSwap, "pumpswap"),
            (RawInstructionDexType::RaydiumLaunchlab, "raydium_launchlab"),
            (RawInstructionDexType::RaydiumCpmm, "raydium_cpmm"),
            (RawInstructionDexType::RaydiumClmm, "raydium_clmm"),
            (RawInstructionDexType::RaydiumAmmV4, "raydium_amm_v4"),
            (RawInstructionDexType::OrcaWhirlpool, "orca_whirlpool"),
            (RawInstructionDexType::MeteoraPools, "meteora_pools"),
            (RawInstructionDexType::MeteoraDammV2, "meteora_damm_v2"),
            (RawInstructionDexType::MeteoraDlmm, "meteora_dlmm"),
            (RawInstructionDexType::MeteoraDbc, "meteora_dbc"),
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), value);
            assert_eq!(
                serde_json::from_value::<RawInstructionDexType>(serde_json::json!(value)).unwrap(),
                kind
            );
        }
        assert!(serde_json::from_str::<RawInstructionDexType>("\"typo\"").is_err());
    }
}
