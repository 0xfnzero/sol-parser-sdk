//! Lossless outer instruction payload, without assumptions about program behavior.
use super::EventMetadata;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInstructionEvent {
    pub metadata: EventMetadata,
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
