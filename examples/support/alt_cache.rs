//! Example-only immutable ALT cache. RPC runs once, before Shred subscription.
use anyhow::{anyhow, ensure, Context, Result};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{message::v0::LoadedAddresses, pubkey::Pubkey, transaction::VersionedTransaction};
use std::{collections::HashMap, time::Duration};

struct CachedTable {
    snapshot_slot: u64,
    last_extended_slot: u64,
    last_extended_start: usize,
    addresses: Vec<Pubkey>,
}

pub struct AltCache(HashMap<Pubkey, CachedTable>);

impl AltCache {
    /// Fetch finalized snapshots at startup. No endpoint or credentials are logged.
    pub fn preload(url: String, table_keys: &[Pubkey]) -> Result<Self> {
        ensure!(!table_keys.is_empty() && table_keys.len() <= 100, "supply 1–100 ALT keys");
        let rpc = RpcClient::new_with_timeout(url, Duration::from_secs(30));
        let response = rpc
            .get_multiple_accounts_with_commitment(table_keys, rpc.commitment())
            .map_err(|_| anyhow!("ALT preload RPC failed; check SOLANA_RPC_URL"))?;
        let mut tables = HashMap::with_capacity(table_keys.len());
        for (key, account) in table_keys.iter().zip(response.value) {
            let account = account.with_context(|| format!("ALT account missing: {key}"))?;
            ensure!(
                account.owner == solana_sdk::pubkey!("AddressLookupTab1e1111111111111111111111111"),
                "account is not an ALT: {key}"
            );
            let table = AddressLookupTable::deserialize(&account.data)
                .map_err(|_| anyhow!("invalid ALT account: {key}"))?;
            // Handling deactivation cooldown requires a current SlotHashes cache.
            // Keep this startup-only example conservative instead of guessing it.
            ensure!(table.meta.deactivation_slot == u64::MAX, "ALT is deactivating: {key}");
            ensure!(table.meta.last_extended_slot <= response.context.slot, "future ALT state");
            tables.insert(
                *key,
                CachedTable {
                    snapshot_slot: response.context.slot,
                    last_extended_slot: table.meta.last_extended_slot,
                    last_extended_start: table.meta.last_extended_slot_start_index as usize,
                    addresses: table.addresses.into_owned(),
                },
            );
        }
        Ok(Self(tables))
    }

    /// Memory lookups only. Missing/stale snapshots and unavailable indices fail.
    /// Ordering is all writable keys followed by all readonly keys, across tables.
    pub fn resolve(
        &self,
        transaction: &VersionedTransaction,
        slot: u64,
    ) -> Result<LoadedAddresses> {
        let lookups = transaction.message.address_table_lookups().unwrap_or_default();
        let mut loaded = LoadedAddresses {
            writable: Vec::with_capacity(lookups.iter().map(|l| l.writable_indexes.len()).sum()),
            readonly: Vec::with_capacity(lookups.iter().map(|l| l.readonly_indexes.len()).sum()),
        };
        for lookup in lookups {
            let table = self.0.get(&lookup.account_key).context("ALT cache miss")?;
            ensure!(slot >= table.snapshot_slot, "ALT snapshot is newer than transaction");
            let active_len = if slot > table.last_extended_slot {
                table.addresses.len()
            } else {
                table.last_extended_start
            };
            for (indices, destination) in [
                (&lookup.writable_indexes, &mut loaded.writable),
                (&lookup.readonly_indexes, &mut loaded.readonly),
            ] {
                for &index in indices {
                    ensure!(
                        (index as usize) < active_len,
                        "ALT index unavailable; refresh cache outside receive task"
                    );
                    destination
                        .push(*table.addresses.get(index as usize).context("invalid ALT index")?);
                }
            }
        }
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::message::{
        v0::{Message, MessageAddressTableLookup},
        VersionedMessage,
    };

    #[test]
    fn preserves_runtime_order_and_rejects_missing_or_unusable_addresses() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let keys: Vec<_> = (0..6).map(|_| Pubkey::new_unique()).collect();
        let cache = AltCache(HashMap::from([
            (
                a,
                CachedTable {
                    snapshot_slot: 10,
                    last_extended_slot: 10,
                    last_extended_start: 2,
                    addresses: keys[..3].to_vec(),
                },
            ),
            (
                b,
                CachedTable {
                    snapshot_slot: 10,
                    last_extended_slot: 9,
                    last_extended_start: 0,
                    addresses: keys[3..].to_vec(),
                },
            ),
        ]));
        let mut tx = VersionedTransaction {
            signatures: vec![],
            message: VersionedMessage::V0(Message {
                address_table_lookups: vec![
                    MessageAddressTableLookup {
                        account_key: a,
                        writable_indexes: vec![2, 0, 2],
                        readonly_indexes: vec![1],
                    },
                    MessageAddressTableLookup {
                        account_key: b,
                        writable_indexes: vec![1],
                        readonly_indexes: vec![2, 0],
                    },
                ],
                ..Default::default()
            }),
        };
        let loaded = cache.resolve(&tx, 11).unwrap();
        assert_eq!(loaded.writable, vec![keys[2], keys[0], keys[2], keys[4]]);
        assert_eq!(loaded.readonly, vec![keys[1], keys[5], keys[3]]);
        assert!(cache.resolve(&tx, 9).is_err()); // Snapshot from a future slot.
        assert!(cache.resolve(&tx, 10).is_err()); // Newly appended key is not active yet.
        let VersionedMessage::V0(message) = &mut tx.message else { unreachable!() };
        message.address_table_lookups[0].account_key = Pubkey::new_unique();
        assert!(cache.resolve(&tx, 11).is_err());
    }
}
