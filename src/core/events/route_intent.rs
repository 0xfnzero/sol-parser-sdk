//! Instructions observed before execution, never confirmed fills.
use super::EventMetadata;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PumpRouteKind {
    MultiHopSwap,
    MultiHopCurveSwap,
    RapidBuy,
    BuyPumpDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PumpRouteHop {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    /// Pool or bonding curve; this does not assert which branch executed.
    pub venue: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
}

/// A recognized outer instruction's route intent. Execution status, actual
/// amounts, reserves and balances are deliberately absent. A wrapper may choose
/// amounts or skip a CPI at runtime. Use execution metadata for confirmed fills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpRouteIntentEvent {
    pub metadata: EventMetadata,
    pub kind: PumpRouteKind,
    pub program: Pubkey,
    /// Index within the signed message, distinct from metadata.tx_index.
    pub instruction_index: usize,
    pub user: Pubkey,
    pub input_account: Pubkey,
    pub output_account: Pubkey,
    /// Explicit instruction input only; None for balance-dependent wrappers.
    pub amount_in: Option<u64>,
    /// Explicit output constraint, never actual output.
    pub min_amount_out: Option<u64>,
    pub hops: Vec<PumpRouteHop>,
    /// Original instruction data, including discriminator. Unknown wrapper
    /// parameters remain opaque instead of being guessed as trade amounts.
    pub instruction_data: Vec<u8>,
}
