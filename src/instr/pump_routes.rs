//! Versioned, program-bound Pump route adapters. No account fetches or execution.
use crate::core::events::{EventMetadata, PumpRouteHop, PumpRouteIntentEvent, PumpRouteKind};
use solana_sdk::pubkey::Pubkey;

pub const MULTI_HOP_SWAP: [u8; 8] = [43, 100, 73, 19, 233, 246, 111, 148];
pub const MULTI_HOP_CURVE_SWAP: [u8; 8] = [225, 154, 117, 22, 215, 86, 246, 103];
const PUMP: Pubkey = solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
const AMM: Pubkey = solana_sdk::pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
const RAPID: Pubkey = solana_sdk::pubkey!("rapidV5DAKtecYzetwCNoxXWHpezjeg4hGt9BGiVcGW");
const DELTA: Pubkey = solana_sdk::pubkey!("J7pourVwqP1VtRNBiFNqbdGxwkx7ta2LHAURURRRJqmd");
const TOKEN: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022: Pubkey = solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const ATA: Pubkey = solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");
const GLOBAL: Pubkey = solana_sdk::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
const EVENT_AUTHORITY: Pubkey = solana_sdk::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
const FEES: Pubkey = solana_sdk::pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ");
const DELTA_BUY: [u8; 8] = [122, 28, 94, 147, 180, 45, 240, 97];

/// Reject unrelated programs/opcodes before building account or event vectors.
#[inline]
pub fn may_parse(program: &Pubkey, data: &[u8]) -> bool {
    if *program == AMM {
        return data.get(..8) == Some(MULTI_HOP_SWAP.as_slice());
    }
    if *program == PUMP {
        return data.get(..8) == Some(MULTI_HOP_CURVE_SWAP.as_slice());
    }
    if *program == RAPID {
        return data.len() == 41
            && data[0] == 0
            && data[9..17] == [1, 0, 0, 27, 15, 14, 8, 24]
            && data[17..25] == crate::instr::pump::discriminators::BUY_EXACT_QUOTE_IN_V2
            && data[25..33] == [0; 8];
    }
    *program == DELTA && data.len() == 28 && data[..8] == DELTA_BUY
}

#[inline]
fn u64_at(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(data.get(at..at.checked_add(8)?)?.try_into().ok()?))
}

/// Shared official-AMM shape validation, also used by the legacy intent helper.
pub(crate) fn multi_hop_args(
    program: Pubkey,
    data: &[u8],
    accounts: &[Pubkey],
) -> Option<(u64, u64)> {
    if program != AMM
        || data.len() != 24
        || data[..8] != MULTI_HOP_SWAP
        || accounts.len() < 21
        || (accounts.len() - 16) % 5 != 0
        || accounts[7] != TOKEN
        || accounts[8] != TOKEN_2022
        || accounts[9] != SYSTEM
        || accounts[11] != AMM
        || accounts[12] != PUMP
    {
        return None;
    }
    let amount = u64_at(data, 8)?;
    (amount > 0).then_some((amount, u64_at(data, 16)?))
}

/// Account layouts come from the official IDLs and the signed regression
/// captures for the two wrapper versions. Never scan arbitrary programs for
/// embedded Pump discriminators, or infer balance-dependent input amounts.
pub fn parse(
    program: Pubkey,
    data: &[u8],
    accounts: &[Pubkey],
    metadata: EventMetadata,
    instruction_index: usize,
) -> Option<PumpRouteIntentEvent> {
    if !may_parse(&program, data) {
        return None;
    }
    let (kind, fixed, user, input, output, amount, minimum) = if program == AMM {
        let (amount, minimum) = multi_hop_args(program, data, accounts)?;
        (PumpRouteKind::MultiHopSwap, 16, 0, 1, 2, Some(amount), Some(minimum))
    } else if program == PUMP {
        let n = u32::from_le_bytes(data.get(16..20)?.try_into().ok()?) as usize;
        if n == 0
            || data.len() != 20usize.checked_add(n)?
            || accounts.len() != 13usize.checked_add(n.checked_mul(5)?)?
            || accounts[8] != TOKEN
            || accounts[9] != TOKEN_2022
            || accounts[10] != SYSTEM
            || accounts[11] != EVENT_AUTHORITY
            || accounts[12] != PUMP
        {
            return None;
        }
        (PumpRouteKind::MultiHopCurveSwap, 13, 3, 5, 6, Some(u64_at(data, 8)?), None)
    } else if program == RAPID {
        if accounts.len() != 33
            || accounts[0] != accounts[19]
            || accounts[2] != accounts[8]
            || accounts[3] != TOKEN
            || accounts[4] != SYSTEM
            || accounts[5] != PUMP
            || accounts[6] != GLOBAL
            || ![TOKEN, TOKEN_2022].contains(&accounts[9])
            || accounts[10] != TOKEN
            || accounts[11] != ATA
            || accounts[29] != FEES
            || accounts[30] != SYSTEM
            || accounts[31] != EVENT_AUTHORITY
            || accounts[32] != PUMP
        {
            return None;
        }
        (PumpRouteKind::RapidBuy, 0, 0, 21, 20, Some(u64_at(data, 1)?), Some(u64_at(data, 33)?))
    } else {
        if accounts.len() != 30
            || accounts[0] != accounts[14]
            || accounts[1] != GLOBAL
            || ![TOKEN, TOKEN_2022].contains(&accounts[4])
            || ![TOKEN, TOKEN_2022].contains(&accounts[5])
            || accounts[6] != ATA
            || accounts[24] != FEES
            || accounts[25] != SYSTEM
            || accounts[26] != EVENT_AUTHORITY
            || accounts[27] != PUMP
        {
            return None;
        }
        // BuyPumpDelta chooses spendable_quote_in from account state at runtime.
        // Its opaque arguments must not become a fabricated exact input/output.
        (PumpRouteKind::BuyPumpDelta, 0, 0, 16, 15, None, None)
    };
    if amount == Some(0) {
        return None;
    }
    let hops = if fixed != 0 {
        accounts[fixed..]
            .chunks_exact(5)
            .map(|h| PumpRouteHop {
                base_mint: h[0],
                quote_mint: h[1],
                venue: h[2],
                base_vault: h[3],
                quote_vault: h[4],
            })
            .collect()
    } else {
        let (base, quote, venue, base_vault, quote_vault) =
            if program == RAPID { (7, 2, 16, 17, 18) } else { (2, 3, 11, 12, 13) };
        vec![PumpRouteHop {
            base_mint: accounts[base],
            quote_mint: accounts[quote],
            venue: accounts[venue],
            base_vault: accounts[base_vault],
            quote_vault: accounts[quote_vault],
        }]
    };
    Some(PumpRouteIntentEvent {
        metadata,
        kind,
        is_buy: match kind {
            PumpRouteKind::RapidBuy | PumpRouteKind::BuyPumpDelta => Some(true),
            PumpRouteKind::MultiHopSwap | PumpRouteKind::MultiHopCurveSwap => None,
        },
        ix_name: (kind == PumpRouteKind::RapidBuy).then(|| "buy_exact_quote_in_v2".to_owned()),
        program,
        instruction_index,
        user: accounts[user],
        input_account: accounts[input],
        output_account: accounts[output],
        amount_in: amount,
        min_amount_out: minimum,
        hops,
        instruction_data: data.to_vec(),
    })
}
