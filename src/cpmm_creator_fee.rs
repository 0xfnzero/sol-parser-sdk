//! CPMM creator-fee collection upgrade helpers. No RPC or account-existence check is required.

use crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID;
use crate::instr::raydium_cpmm::discriminators::{
    COLLECT_CREATOR_FEE, COLLECT_CREATOR_FEE_PERMISSIONLESS,
};
pub use solana_instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

pub const CREATOR_FEE_SHARE_DENOMINATOR: u64 = 1_000_000;

/// Canonical address, including when the PDA has not been created on-chain.
pub fn derive_creator_fee_share(creator: &Pubkey, amm_config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"creator_fee_share", creator.as_ref(), amm_config.as_ref()],
        &RAYDIUM_CPMM_PROGRAM_ID,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionUpgradeError {
    WrongProgram,
    WrongInstruction,
    WrongAccountCount,
    WrongConfig,
    WrongShareAddress,
    UnexpectedSigner,
}
impl std::fmt::Display for CollectionUpgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CPMM collection upgrade: {self:?}")
    }
}
impl std::error::Error for CollectionUpgradeError {}

/// Append mandatory readonly accounts to a pre-upgrade collection call.
/// The first 14 accounts (including all privileges) remain unchanged. Already-upgraded
/// calls have their appended addresses and signer flags checked and are returned
/// unchanged. Writable supersets are accepted. Errors leave the instruction unchanged.
/// `amm_config` must be the pool's config; this helper does not fetch PoolState.
pub fn upgrade_creator_fee_collection_instruction(
    ix: &mut Instruction,
    amm_config: &Pubkey,
) -> Result<Pubkey, CollectionUpgradeError> {
    if ix.program_id != RAYDIUM_CPMM_PROGRAM_ID {
        return Err(CollectionUpgradeError::WrongProgram);
    }
    let permissionless = match ix.data.as_slice() {
        d if d == COLLECT_CREATOR_FEE => false,
        d if d == COLLECT_CREATOR_FEE_PERMISSIONLESS => true,
        _ => return Err(CollectionUpgradeError::WrongInstruction),
    };
    let upgraded_count = if permissionless { 16 } else { 15 };
    if ix.accounts.len() != 14 && ix.accounts.len() != upgraded_count {
        return Err(CollectionUpgradeError::WrongAccountCount);
    }
    if !permissionless && ix.accounts[3].pubkey != *amm_config {
        return Err(CollectionUpgradeError::WrongConfig);
    }
    // Validate cheap metadata before performing the curve checks used by PDA
    // derivation. All failures still leave the caller's instruction untouched.
    if ix.accounts.len() == upgraded_count {
        if permissionless && ix.accounts[14].pubkey != *amm_config {
            return Err(CollectionUpgradeError::WrongConfig);
        }
        if ix.accounts[14..].iter().any(|account| account.is_signer) {
            // The appended config/share PDAs cannot supply transaction signatures.
            return Err(CollectionUpgradeError::UnexpectedSigner);
        }
    }
    let creator = ix.accounts[usize::from(permissionless)].pubkey;
    let share = derive_creator_fee_share(&creator, amm_config).0;
    if ix.accounts.len() == upgraded_count {
        if ix.accounts[upgraded_count - 1].pubkey != share {
            return Err(CollectionUpgradeError::WrongShareAddress);
        }
        return Ok(share);
    }
    ix.accounts.reserve(upgraded_count - 14);
    if permissionless {
        ix.accounts.push(AccountMeta::new_readonly(*amm_config, false));
    }
    ix.accounts.push(AccountMeta::new_readonly(share, false));
    Ok(share)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorFeeSplit {
    pub creator_amount: u64,
    pub protocol_amount: u64,
}

/// Use the existing PDA's rate (including zero), otherwise the config's rate.
/// The caller must supply a validated PDA for the relevant creator/config pair.
pub fn effective_creator_fee_share_rate(
    config_rate: u64,
    override_rate: Option<u64>,
) -> Option<u64> {
    let rate = override_rate.unwrap_or(config_rate);
    (rate <= CREATOR_FEE_SHARE_DENOMINATOR).then_some(rate)
}

/// Exact collection-time estimate: protocol share floors; all remaining dust goes
/// to the creator. Apply independently to creator_fees_token_0 and _1. This does
/// not mutate PoolState or infer a collection result from a shred transaction.
pub fn split_creator_fee(accrued_fee: u64, share_rate: u64) -> Option<CreatorFeeSplit> {
    if share_rate > CREATOR_FEE_SHARE_DENOMINATOR {
        return None;
    }
    let protocol_amount =
        ((accrued_fee as u128 * share_rate as u128) / CREATOR_FEE_SHARE_DENOMINATOR as u128) as u64;
    Some(CreatorFeeSplit { creator_amount: accrued_fee - protocol_amount, protocol_amount })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn collection(permissionless: bool) -> Instruction {
        Instruction {
            program_id: RAYDIUM_CPMM_PROGRAM_ID,
            data: if permissionless {
                COLLECT_CREATOR_FEE_PERMISSIONLESS
            } else {
                COLLECT_CREATOR_FEE
            }
            .to_vec(),
            accounts: (0..14)
                .map(|i| {
                    if i % 2 == 0 {
                        AccountMeta::new(Pubkey::new_unique(), i == 0)
                    } else {
                        AccountMeta::new_readonly(Pubkey::new_unique(), false)
                    }
                })
                .collect(),
        }
    }
    #[test]
    fn already_upgraded_pdas_cannot_be_required_transaction_signers() {
        for permissionless in [false, true] {
            let mut ix = collection(permissionless);
            let config = if permissionless { Pubkey::new_unique() } else { ix.accounts[3].pubkey };
            upgrade_creator_fee_collection_instruction(&mut ix, &config).unwrap();
            for index in 14..ix.accounts.len() {
                let mut invalid = ix.clone();
                invalid.accounts[index].is_signer = true;
                let before = invalid.clone();
                assert_eq!(
                    upgrade_creator_fee_collection_instruction(&mut invalid, &config),
                    Err(CollectionUpgradeError::UnexpectedSigner)
                );
                assert_eq!(invalid, before);
            }
            // Writable flags can be a valid superset; only impossible signatures fail.
            for account in &mut ix.accounts[14..] {
                account.is_writable = true;
            }
            assert!(upgrade_creator_fee_collection_instruction(&mut ix, &config).is_ok());
        }
    }
    #[test]
    fn appends_only_required_accounts_without_chain_existence_and_is_idempotent() {
        for permissionless in [false, true] {
            let mut ix = collection(permissionless);
            let first = ix.accounts.clone();
            let config = if permissionless { Pubkey::new_unique() } else { ix.accounts[3].pubkey };
            let creator = ix.accounts[usize::from(permissionless)].pubkey;
            let share = upgrade_creator_fee_collection_instruction(&mut ix, &config).unwrap();
            assert_eq!(ix.accounts[..14], first);
            assert_eq!(ix.accounts.len(), if permissionless { 16 } else { 15 });
            assert_eq!(
                share,
                Pubkey::find_program_address(
                    &[b"creator_fee_share", creator.as_ref(), config.as_ref()],
                    &RAYDIUM_CPMM_PROGRAM_ID
                )
                .0
            );
            assert_eq!(ix.accounts.last().unwrap(), &AccountMeta::new_readonly(share, false));
            if permissionless {
                assert_eq!(ix.accounts[14], AccountMeta::new_readonly(config, false));
            }
            let complete = ix.clone();
            assert_eq!(upgrade_creator_fee_collection_instruction(&mut ix, &config), Ok(share));
            assert_eq!(ix, complete);
        }
    }
    #[test]
    fn rejects_wrong_program_data_count_config_or_share_without_mutation() {
        for case in 0..5 {
            let mut ix = collection(false);
            let config = ix.accounts[3].pubkey;
            match case {
                0 => ix.program_id = Pubkey::new_unique(),
                1 => ix.data.push(0),
                2 => {
                    ix.accounts.pop();
                }
                3 => ix.accounts[3].pubkey = Pubkey::new_unique(),
                _ => ix.accounts.push(AccountMeta::new_readonly(Pubkey::new_unique(), false)),
            }
            let before = ix.clone();
            assert!(upgrade_creator_fee_collection_instruction(&mut ix, &config).is_err());
            assert_eq!(ix, before);
        }
    }
    #[test]
    fn exact_split_conserves_value_and_floors_protocol_share() {
        for fee in [0, 1, 5, 999_999, 1_000_001, u64::MAX] {
            for rate in [0, 1, 200_000, 999_999, 1_000_000] {
                let split = split_creator_fee(fee, rate).unwrap();
                assert_eq!(
                    split.creator_amount as u128 + split.protocol_amount as u128,
                    fee as u128
                );
                assert_eq!(split.protocol_amount as u128, fee as u128 * rate as u128 / 1_000_000);
            }
        }
        assert_eq!(split_creator_fee(1, 200_000).unwrap().creator_amount, 1);
        assert!(split_creator_fee(u64::MAX, 1_000_001).is_none());
        assert_eq!(effective_creator_fee_share_rate(200_000, None), Some(200_000));
        assert_eq!(effective_creator_fee_share_rate(200_000, Some(0)), Some(0));
        assert_eq!(effective_creator_fee_share_rate(0, Some(300_000)), Some(300_000));
        assert_eq!(effective_creator_fee_share_rate(0, Some(1_000_001)), None);
    }
}
