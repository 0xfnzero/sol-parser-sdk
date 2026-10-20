//! DBC emits legacy and v2 events for the same execution. Prefer the v2 layout.
use crate::core::events::{DexEvent, MeteoraDbcSwapEvent};
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};

type Key = (Pubkey, Pubkey, u8, u64, u64, u128, u64, u64, u64, u64, Option<(Pubkey, usize)>);
fn key(e: &MeteoraDbcSwapEvent) -> Key {
    (
        e.pool,
        e.config,
        e.trade_direction,
        e.amount_in,
        e.output_amount,
        e.next_sqrt_price,
        e.trading_fee,
        e.protocol_fee,
        e.referral_fee,
        e.current_timestamp,
        e.third_program.as_ref().map(|source| (source.program_id, source.instruction_index)),
    )
}

pub(crate) fn prefer_current_dbc_events(mut events: Vec<DexEvent>) -> Vec<DexEvent> {
    if !events.iter().any(|e| matches!(e, DexEvent::MeteoraDbcSwap(s) if s.event_version == 2)) {
        return events;
    }
    let mut groups: HashMap<Key, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for (i, e) in events.iter().enumerate() {
        if let DexEvent::MeteoraDbcSwap(s) = e {
            let (legacy, current) = groups.entry(key(s)).or_default();
            if s.event_version == 2 {
                current.push(i);
            } else {
                legacy.push(i);
            }
        }
    }
    let mut removed = HashSet::new();
    for (legacy, current) in groups.values() {
        // Unequal source counts cannot identify which occurrence is missing.
        if legacy.len() != current.len() {
            continue;
        }
        for (&old, &new) in legacy.iter().zip(current) {
            events[old] = events[new].clone();
            removed.insert(new);
        }
    }
    events
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| (!removed.contains(&i)).then_some(e))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatibility_events_from_different_wrappers_are_not_paired() {
        let old = MeteoraDbcSwapEvent {
            third_program: Some(crate::core::events::RawInstructionPayload {
                instruction_index: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut current = old.clone();
        current.event_version = 2;
        current.third_program.as_mut().unwrap().instruction_index = 2;
        assert_eq!(
            prefer_current_dbc_events(vec![
                DexEvent::MeteoraDbcSwap(old),
                DexEvent::MeteoraDbcSwap(current)
            ])
            .len(),
            2
        );
    }
    #[test]
    fn compatibility_pairs_preserve_repeated_and_incomplete_executions() {
        let old = DexEvent::MeteoraDbcSwap(MeteoraDbcSwapEvent {
            pool: Pubkey::new_unique(),
            amount_in: 100,
            ..Default::default()
        });
        let mut current = old.clone();
        if let DexEvent::MeteoraDbcSwap(e) = &mut current {
            e.event_version = 2;
            e.swap_mode = 1;
        }
        assert_eq!(prefer_current_dbc_events(vec![old.clone(), current.clone()]).len(), 1);
        assert_eq!(
            prefer_current_dbc_events(vec![
                old.clone(),
                old.clone(),
                current.clone(),
                current.clone()
            ])
            .len(),
            2
        );
        assert_eq!(prefer_current_dbc_events(vec![old.clone(), old, current]).len(), 3);
    }
}
