//! Official PumpSwap route inputs, without execution metadata or RPC.
//! Layout: pump-public-docs/docs/instructions/MULTI_HOP_SWAP.md.
use crate::core::events::{
    EventMetadata, PumpFunTradeEvent, RawInstructionPayload, TradeParseStatus,
};
use crate::instr::program_ids::{PUMPFUN_PROGRAM_ID, PUMPSWAP_PROGRAM_ID};
use solana_sdk::pubkey::Pubkey;

pub(crate) const MULTI_HOP_SWAP: [u8; 8] = [43, 100, 73, 19, 233, 246, 111, 148];

/// Emit one input-only callback per identifiable bonding-curve hop. The Pump
/// program is a fixed account even in all-AMM routes, so its presence alone is
/// insufficient. Check each venue against the canonical bonding-curve PDA.
pub(crate) fn parse_curve_hops(
    data: &[u8],
    account_indices: &[u8],
    keys: &[Pubkey],
    instruction_index: usize,
    metadata: EventMetadata,
    mut emit: impl FnMut(PumpFunTradeEvent),
) {
    const FIXED: usize = 16;
    const HOP: usize = 5;
    if data.len() != 24
        || !data.starts_with(&MULTI_HOP_SWAP)
        || account_indices.len() < FIXED + HOP
        || (account_indices.len() - FIXED) % HOP != 0
        || data[8..16] == [0; 8]
        || data[16..24] == [0; 8]
    {
        return;
    }
    let get = |i: usize| account_indices.get(i).and_then(|i| keys.get(*i as usize)).copied();
    if get(11) != Some(PUMPSWAP_PROGRAM_ID) || get(12) != Some(PUMPFUN_PROGRAM_ID) {
        return;
    }
    let Some(user) = get(0) else { return };
    let hops = (account_indices.len() - FIXED) / HOP;
    // All hops share one direction. A single hop, ambiguous/cyclic mint pairs,
    // or unresolved ALT mints cannot establish direction from account keys alone.
    let connected = |buy: bool| {
        hops > 1
            && (0..hops - 1).all(|i| {
                let from = get(FIXED + i * HOP + usize::from(!buy));
                let to = get(FIXED + (i + 1) * HOP + usize::from(buy));
                from.is_some_and(|mint| mint != Pubkey::default() && Some(mint) == to)
            })
    };
    let direction = match (connected(true), connected(false)) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    };
    for hop in 0..hops {
        let start = FIXED + hop * HOP;
        let (Some(mint), Some(quote_mint), Some(bonding_curve)) =
            (get(start), get(start + 1), get(start + 2))
        else {
            continue;
        };
        if mint == Pubkey::default() || quote_mint == Pubkey::default() || mint == quote_mint {
            continue;
        }
        let expected =
            Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &PUMPFUN_PROGRAM_ID).0;
        if bonding_curve != expected {
            continue;
        }
        emit(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ix_name: "multi_hop_swap".into(),
            mint,
            quote_mint,
            user,
            bonding_curve,
            associated_bonding_curve: get(start + 3).unwrap_or_default(),
            associated_quote_bonding_curve: get(start + 4).unwrap_or_default(),
            global: get(13).unwrap_or_default(),
            fee_config: get(14).unwrap_or_default(),
            event_authority: get(15).unwrap_or_default(),
            program: PUMPFUN_PROGRAM_ID,
            system_program: get(9).unwrap_or_default(),
            is_buy: direction.unwrap_or(false),
            parse_status: if direction.is_some() {
                TradeParseStatus::InstructionDecoded
            } else {
                TradeParseStatus::Unresolved
            },
            third_program: Some(RawInstructionPayload {
                program_id: PUMPSWAP_PROGRAM_ID,
                instruction_index,
                accounts: account_indices.iter().map(|i| keys.get(*i as usize).copied()).collect(),
                account_indices: account_indices.to_vec(),
                instruction_data: data.to_vec(),
            }),
            // Route amount_in/min_amount_out are not this hop's fill or input.
            // They remain available in third_program.instruction_data.
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_venues_direction_and_unresolved_keys_are_not_guessed() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let wsol = crate::core::events::PUMPFUN_WSOL_QUOTE_MINT;
        let curve = |mint: &Pubkey| {
            Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &PUMPFUN_PROGRAM_ID).0
        };
        let mut keys = vec![Pubkey::new_unique(); 16];
        keys[11] = PUMPSWAP_PROGRAM_ID;
        keys[12] = PUMPFUN_PROGRAM_ID;
        keys.extend([a, wsol, curve(&a), Pubkey::new_unique(), Pubkey::new_unique()]);
        keys.extend([b, a, curve(&b), Pubkey::new_unique(), Pubkey::new_unique()]);
        let indices: Vec<u8> = (0..keys.len() as u8).collect();
        let mut data = MULTI_HOP_SWAP.to_vec();
        data.extend_from_slice(&500u64.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());
        let parse = |keys: &[Pubkey], indices: &[u8], data: &[u8]| {
            let mut events = Vec::new();
            parse_curve_hops(data, indices, keys, 5, EventMetadata::default(), |e| events.push(e));
            events
        };
        let buy = parse(&keys, &indices, &data);
        assert_eq!(buy.iter().map(|e| e.mint).collect::<Vec<_>>(), vec![a, b]);
        assert!(buy
            .iter()
            .all(|e| e.is_buy && e.ix_name == "multi_hop_swap" && e.token_amount == 0));
        let mut reverse = indices[..16].to_vec();
        reverse.extend_from_slice(&indices[21..]);
        reverse.extend_from_slice(&indices[16..21]);
        let sell = parse(&keys, &reverse, &data);
        assert_eq!(sell.iter().map(|e| e.mint).collect::<Vec<_>>(), vec![b, a]);
        assert!(sell
            .iter()
            .all(|e| !e.is_buy && e.parse_status == TradeParseStatus::InstructionDecoded));
        assert!(parse(&keys, &indices[..25], &data).is_empty());
        assert!(parse(&keys, &indices, &data[..23]).is_empty());
        let mut bad_data = data.clone();
        bad_data[0] ^= 1;
        assert!(parse(&keys, &indices, &bad_data).is_empty());
        let mut unknown = indices.clone();
        unknown[23] = 255; // second venue is not resolved; do not fabricate it.
        assert_eq!(parse(&keys, &unknown, &data).len(), 1);
        unknown[19] = 255; // first vault remains explicitly unresolved in the payload.
        assert!(
            parse(&keys, &unknown, &data)[0].third_program.as_ref().unwrap().accounts[19].is_none()
        );
        keys[18] = Pubkey::new_unique();
        assert_eq!(parse(&keys, &indices, &data).len(), 1);
        keys[23] = Pubkey::new_unique();
        assert!(
            parse(&keys, &indices, &data).is_empty(),
            "all-AMM route must not emit PumpFunTrade"
        );
    }
}
