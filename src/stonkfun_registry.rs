//! Caller-owned provenance index for successfully migrated StonkFun CPMM pools.
//! Persist it with serde; it is not a network-wide pool discovery service.
use crate::{
    core::events::{RaydiumLaunchlabMigrateAmmEvent, StonkFunMode},
    DexEvent,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StonkFunGraduatedPool {
    pub curve_pool: Pubkey,
    pub pool: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub platform_config: Pubkey,
    pub migration_signature: Signature,
    pub migration_slot: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn migration() -> DexEvent {
        let mut accounts: Vec<_> = (0..28).map(|_| Pubkey::new_unique()).collect();
        accounts[3] = crate::core::events::STONKFUN_REWARD_PLATFORM_CONFIG;
        accounts[4] = crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID;
        crate::instr::raydium_launchlab::parse_instruction(
            &crate::instr::raydium_launchlab::discriminators::MIGRATE_TO_CPSWAP,
            &accounts,
            Signature::new_unique(),
            123,
            0,
            None,
        )
        .unwrap()
    }
    #[test]
    fn successful_migrations_are_replay_safe_and_json_persistent() {
        let event = migration();
        let mut registry = StonkFunPoolRegistry::default();
        assert_eq!(registry.observe_events(&[event.clone()], false).unwrap(), 0);
        assert!(registry.verified_cpmm_pools().is_empty());
        assert_eq!(registry.observe_events(&[event.clone(), event.clone()], true).unwrap(), 1);
        assert_eq!(registry.observe_events(&[event.clone()], true).unwrap(), 0);
        let DexEvent::RaydiumLaunchlabMigrateAmm(e) = event else { panic!("migration") };
        assert_eq!(registry.pools_for_base_mint(&e.base_mint)[0].quote_mint, e.quote_mint);
        let json = serde_json::to_string(&registry).unwrap();
        let restored: StonkFunPoolRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.get(&e.new_pool), registry.get(&e.new_pool));
    }
    #[test]
    fn conflicting_batch_is_atomic_and_foreign_platforms_are_ignored() {
        let original = migration();
        let mut registry = StonkFunPoolRegistry::default();
        registry.observe_events(&[original.clone()], true).unwrap();
        let mut conflicting = original.clone();
        let DexEvent::RaydiumLaunchlabMigrateAmm(e) = &mut conflicting else { panic!("migration") };
        e.quote_mint = Pubkey::new_unique();
        let before = serde_json::to_string(&registry).unwrap();
        assert!(registry.observe_events(&[migration(), conflicting], true).is_err());
        assert_eq!(serde_json::to_string(&registry).unwrap(), before);
        let mut foreign = migration();
        let DexEvent::RaydiumLaunchlabMigrateAmm(e) = &mut foreign else { panic!("migration") };
        e.platform_config = Pubkey::new_unique();
        assert_eq!(registry.observe_events(&[foreign], true).unwrap(), 0);
    }
}

impl StonkFunGraduatedPool {
    pub fn mode(&self) -> Option<StonkFunMode> {
        crate::core::events::stonkfun_mode_from_platform_config(self.platform_config)
    }
}

/// Entries are sorted by pool for deterministic persistence. Ingest only data
/// from a trusted RPC/subscription, with finalized status if rollback matters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StonkFunPoolRegistry {
    #[serde(with = "pool_entries")]
    pools: BTreeMap<Pubkey, StonkFunGraduatedPool>,
}

mod pool_entries {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        pools: &BTreeMap<Pubkey, StonkFunGraduatedPool>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        pools.values().collect::<Vec<_>>().serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<Pubkey, StonkFunGraduatedPool>, D::Error> {
        let entries = Vec::<StonkFunGraduatedPool>::deserialize(deserializer)?;
        let mut pools = BTreeMap::new();
        for entry in entries {
            if entry.mode().is_none()
                || [entry.curve_pool, entry.pool, entry.base_mint, entry.quote_mint]
                    .contains(&Pubkey::default())
                || entry.curve_pool == entry.pool
                || entry.base_mint == entry.quote_mint
                || pools.contains_key(&entry.pool)
            {
                return Err(serde::de::Error::custom(
                    "Invalid or duplicate StonkFun registry entry",
                ));
            }
            pools.insert(entry.pool, entry);
        }
        Ok(pools)
    }
}

impl StonkFunPoolRegistry {
    pub fn get(&self, pool: &Pubkey) -> Option<&StonkFunGraduatedPool> {
        self.pools.get(pool)
    }

    pub fn pools_for_base_mint(&self, mint: &Pubkey) -> Vec<&StonkFunGraduatedPool> {
        self.pools.values().filter(|entry| entry.base_mint == *mint).collect()
    }

    /// Supply these identities to the route analyzer for graduated attribution.
    pub fn verified_cpmm_pools(&self) -> Vec<Pubkey> {
        self.pools.keys().copied().collect()
    }

    /// Replay-safe and atomic: conflicting provenance leaves the index unchanged.
    /// The execution flag must come from transaction metadata, not log presence.
    pub fn observe_events(
        &mut self,
        events: &[DexEvent],
        succeeded: bool,
    ) -> Result<usize, String> {
        if !succeeded {
            return Ok(0);
        }
        let mut pending = BTreeMap::new();
        for event in events {
            let DexEvent::RaydiumLaunchlabMigrateAmm(event) = event else { continue };
            let Some(entry) = Self::from_migration(event) else { continue };
            if let Some(previous) = pending.get(&entry.pool).or_else(|| self.pools.get(&entry.pool))
            {
                // RPC and stream metadata can assign different receive/index data.
                // Identity fields and original migration provenance must agree.
                if previous != &entry {
                    return Err(format!(
                        "Conflicting StonkFun migration provenance for {}",
                        entry.pool
                    ));
                }
            } else {
                pending.insert(entry.pool, entry);
            }
        }
        let count = pending.len();
        self.pools.extend(pending);
        Ok(count)
    }

    pub fn observe_rpc_transaction(
        &mut self,
        transaction: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<usize, String> {
        let succeeded = transaction
            .transaction
            .meta
            .as_ref()
            .ok_or("Transaction metadata is missing")?
            .err
            .is_none();
        if !succeeded {
            return Ok(0);
        }
        let events =
            crate::parse_rpc_transaction(transaction, None).map_err(|error| error.to_string())?;
        self.observe_events(&events, true)
    }

    fn from_migration(event: &RaydiumLaunchlabMigrateAmmEvent) -> Option<StonkFunGraduatedPool> {
        if event.stonkfun_mode().is_none()
            || event.destination_program != crate::instr::program_ids::RAYDIUM_CPMM_PROGRAM_ID
            || [event.old_pool, event.new_pool, event.base_mint, event.quote_mint]
                .contains(&Pubkey::default())
            || event.old_pool == event.new_pool
            || event.base_mint == event.quote_mint
        {
            return None;
        }
        Some(StonkFunGraduatedPool {
            curve_pool: event.old_pool,
            pool: event.new_pool,
            base_mint: event.base_mint,
            quote_mint: event.quote_mint,
            platform_config: event.platform_config,
            migration_signature: event.metadata.signature,
            migration_slot: event.metadata.slot,
        })
    }
}
