use crate::core::events::*;
use crate::grpc::types::EventType;
use solana_sdk::pubkey::Pubkey;
pub fn event_type(disc: u64, program: Option<&Pubkey>) -> Option<EventType> {
    if disc == 18146529233607700591
        && program.is_none_or(|p| {
            *p == solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
        })
    {
        return Some(EventType::PumpFunPostCompleteBuy);
    }
    if disc == 3118876958563052404
        && program.is_none_or(|p| {
            *p == solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
        })
    {
        return Some(EventType::PumpFunSweepBondingCurveFee);
    }
    if disc == 619296439455019615
        && program.is_none_or(|p| {
            *p == solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
        })
    {
        return Some(EventType::PumpFunComplete);
    }
    if disc == 11927646055507993730
        && program.is_none_or(|p| {
            *p == solana_sdk::pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
        })
    {
        return Some(EventType::PumpSwapSweepPoolFee);
    }
    None
}
pub fn parse(
    disc: u64,
    data: &[u8],
    metadata: EventMetadata,
    program: Option<&Pubkey>,
) -> Option<DexEvent> {
    let kind = event_type(disc, program)?;
    if kind == EventType::PumpFunPostCompleteBuy {
        if data.len() != 224 {
            return None;
        }
        return Some(DexEvent::PumpFunPostCompleteBuy(PumpFunPostCompleteBuyEvent {
            metadata,
            user: Pubkey::new_from_array(data[0..32].try_into().ok()?),
            mint: Pubkey::new_from_array(data[32..64].try_into().ok()?),
            bonding_curve: Pubkey::new_from_array(data[64..96].try_into().ok()?),
            quote_mint: Pubkey::new_from_array(data[96..128].try_into().ok()?),
            timestamp: i64::from_le_bytes(data[128..136].try_into().ok()?),
            base_out: u64::from_le_bytes(data[136..144].try_into().ok()?),
            quote_in: u64::from_le_bytes(data[144..152].try_into().ok()?),
            fee_basis_points: u64::from_le_bytes(data[152..160].try_into().ok()?),
            fee: u64::from_le_bytes(data[160..168].try_into().ok()?),
            creator_fee_basis_points: u64::from_le_bytes(data[168..176].try_into().ok()?),
            creator_fee: u64::from_le_bytes(data[176..184].try_into().ok()?),
            buyback_fee: u64::from_le_bytes(data[184..192].try_into().ok()?),
            pool_base_reserves_before: u64::from_le_bytes(data[192..200].try_into().ok()?),
            pool_quote_reserves_before: u64::from_le_bytes(data[200..208].try_into().ok()?),
            pool_base_reserves_after: u64::from_le_bytes(data[208..216].try_into().ok()?),
            pool_quote_reserves_after: u64::from_le_bytes(data[216..224].try_into().ok()?),
        }));
    }
    if kind == EventType::PumpFunSweepBondingCurveFee {
        if data.len() != 145 {
            return None;
        }
        return Some(DexEvent::PumpFunSweepBondingCurveFee(PumpFunSweepBondingCurveFeeEvent {
            metadata,
            timestamp: i64::from_le_bytes(data[0..8].try_into().ok()?),
            mint: Pubkey::new_from_array(data[8..40].try_into().ok()?),
            bonding_curve: Pubkey::new_from_array(data[40..72].try_into().ok()?),
            quote_mint: Pubkey::new_from_array(data[72..104].try_into().ok()?),
            recipient: Pubkey::new_from_array(data[104..136].try_into().ok()?),
            amount: u64::from_le_bytes(data[136..144].try_into().ok()?),
            bucket: data[144],
        }));
    }
    if kind == EventType::PumpFunComplete {
        if data.len() != 136 && data.len() != 104 {
            return None;
        }
        return Some(DexEvent::PumpFunComplete(PumpFunCompleteEvent {
            metadata,
            user: Pubkey::new_from_array(data[0..32].try_into().ok()?),
            mint: Pubkey::new_from_array(data[32..64].try_into().ok()?),
            bonding_curve: Pubkey::new_from_array(data[64..96].try_into().ok()?),
            timestamp: i64::from_le_bytes(data[96..104].try_into().ok()?),
            quote_mint: if data.len() == 104 {
                solana_sdk::pubkey!("So11111111111111111111111111111111111111112")
            } else {
                Pubkey::new_from_array(data[104..136].try_into().ok()?)
            },
        }));
    }
    if kind == EventType::PumpSwapSweepPoolFee {
        if data.len() != 177 {
            return None;
        }
        return Some(DexEvent::PumpSwapSweepPoolFee(PumpSwapSweepPoolFeeEvent {
            metadata,
            timestamp: i64::from_le_bytes(data[0..8].try_into().ok()?),
            pool: Pubkey::new_from_array(data[8..40].try_into().ok()?),
            base_mint: Pubkey::new_from_array(data[40..72].try_into().ok()?),
            quote_mint: Pubkey::new_from_array(data[72..104].try_into().ok()?),
            recipient: Pubkey::new_from_array(data[104..136].try_into().ok()?),
            payer: Pubkey::new_from_array(data[136..168].try_into().ok()?),
            amount: u64::from_le_bytes(data[168..176].try_into().ok()?),
            bucket: data[176],
        }));
    }
    None
}

/// Instruction intent only. Each hop's executed amounts come from its trade events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PumpMultiHopIntent {
    pub user: Pubkey,
    pub input_account: Pubkey,
    pub output_account: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub hops: Vec<PumpMultiHopAccounts>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PumpMultiHopAccounts {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub venue: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
}
pub fn decode_multi_hop_intent(
    program: Pubkey,
    data: &[u8],
    accounts: &[Pubkey],
) -> Option<PumpMultiHopIntent> {
    if program != solana_sdk::pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
        || data.len() != 24
        || data[..8] != [43, 100, 73, 19, 233, 246, 111, 148]
        || accounts.len() < 21
        || (accounts.len() - 16) % 5 != 0
    {
        return None;
    }
    let amount_in = u64::from_le_bytes(data[8..16].try_into().ok()?);
    let min_amount_out = u64::from_le_bytes(data[16..24].try_into().ok()?);
    if amount_in == 0 || min_amount_out == 0 {
        return None;
    }
    Some(PumpMultiHopIntent {
        user: accounts[0],
        input_account: accounts[1],
        output_account: accounts[2],
        amount_in,
        min_amount_out,
        hops: accounts[16..]
            .chunks_exact(5)
            .map(|h| PumpMultiHopAccounts {
                base_mint: h[0],
                quote_mint: h[1],
                venue: h[2],
                base_vault: h[3],
                quote_vault: h[4],
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_event_layouts() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/pump_upgrade/events.json"))
                .unwrap();
        for f in fixture.as_array().unwrap() {
            let n = f["discString"].as_str().unwrap().parse().unwrap();
            let hex = f["body"].as_str().unwrap();
            let body: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let program = f["program"].as_str().unwrap().parse().unwrap();
            assert!(parse(n, &body, EventMetadata::default(), Some(&program)).is_some());
            assert!(parse(n, &body[..body.len() - 1], EventMetadata::default(), Some(&program))
                .is_none());
            assert!(parse(n, &body, EventMetadata::default(), Some(&Pubkey::default())).is_none());
        }
        assert!(parse(619296439455019615, &[0; 104], EventMetadata::default(), None).is_some());
    }
    #[test]
    fn different_route_venues_do_not_merge() {
        let mut base = DexEvent::PumpSwapSell(PumpSwapSellEvent {
            pool: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            ..Default::default()
        });
        let inner = DexEvent::PumpSwapSell(PumpSwapSellEvent {
            pool: Pubkey::new_unique(),
            ..Default::default()
        });
        let mut unmerged = None;
        assert!(!crate::core::merger::try_merge_events(&mut base, inner, &mut unmerged));
        assert!(unmerged.is_some());
    }
    #[test]
    fn multi_hop_curve_identity_survives_unmarked_log() {
        for reverse in [false, true] {
            let marked = DexEvent::PumpFunTrade(PumpFunTradeEvent {
                mint: Pubkey::new_unique(),
                ix_name: "multi_hop_swap".into(),
                ..Default::default()
            });
            let log = DexEvent::PumpFunTrade(PumpFunTradeEvent {
                mint: Pubkey::new_unique(),
                ..Default::default()
            });
            let (mut base, inner) = if reverse { (log, marked) } else { (marked, log) };
            let mut unmerged = None;
            assert!(!crate::core::merger::try_merge_events(&mut base, inner, &mut unmerged));
            assert!(unmerged.is_some());
        }
    }
}
