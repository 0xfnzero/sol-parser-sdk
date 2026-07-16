//! Nonce Account 解析
//!
//! 提供 Nonce Account 的解析功能

use crate::core::events::{EventMetadata, NonceAccountEvent};
use crate::DexEvent;

use super::token::AccountData;

// Nonce account 固定大小: 80 bytes
const NONCE_ACCOUNT_SIZE: usize = 80;
// Authority pubkey 位置 (bytes 8-39)
const AUTHORITY_OFFSET: usize = 8;
// Nonce/blockhash 位置 (bytes 40-71)
const NONCE_OFFSET: usize = 40;

pub fn parse_nonce_account(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    let data = &account.data;

    // Check size
    if data.len() != NONCE_ACCOUNT_SIZE {
        return None;
    }

    // Extract authority (32 bytes at offset 8)
    let authority_bytes: [u8; 32] =
        data[AUTHORITY_OFFSET..AUTHORITY_OFFSET + 32].try_into().ok()?;
    let authority = bs58::encode(&authority_bytes).into_string();

    // Extract nonce/blockhash (32 bytes at offset 40)
    let nonce_bytes: [u8; 32] = data[NONCE_OFFSET..NONCE_OFFSET + 32].try_into().ok()?;
    let nonce = bs58::encode(&nonce_bytes).into_string();

    let event = NonceAccountEvent {
        metadata,
        pubkey: account.pubkey,
        executable: account.executable,
        lamports: account.lamports,
        owner: account.owner,
        rent_epoch: account.rent_epoch,
        nonce,
        authority,
    };

    Some(DexEvent::NonceAccount(event))
}

/// Helper function to detect if account is a nonce account
///
/// Nonce accounts have a discriminator of [1, 0, 0, 0, 1, 0, 0, 0]
pub fn is_nonce_account(data: &[u8]) -> bool {
    data.len() >= 8 && data[0..8] == [1, 0, 0, 0, 1, 0, 0, 0]
}
