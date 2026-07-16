//! 同笔交易内 Pump 事件后处理（**零 RPC**，供 gRPC/Shred 热路径）：
//! - Create/CreateV2 + 后续 Buy 分离时，将 Buy 的 fee recipient 回填到 `observed_fee_recipient`
//! - 将 **Create / CreateV2 指令**里的 `is_cashback_enabled`、`is_mayhem_mode` 合并进同一 mint 的 Trade 事件，
//!   避免仅有外层指令、TradeEvent 日志缺字段时 `sol-trade-sdk` 误判返现与 Mayhem fee 池

use std::collections::HashMap;

use solana_sdk::pubkey::Pubkey;

use crate::core::events::{
    is_pumpfun_solscan_sol_quote_mint, normalize_pumpfun_quote_mint, DexEvent,
    PumpFunCreateTokenEvent, PumpFunCreateV2TokenEvent, PumpFunTradeEvent,
};

fn pumpfun_buy_like_mint_fee(e: &DexEvent) -> Option<(Pubkey, Pubkey)> {
    match e {
        DexEvent::PumpFunTrade(t) if t.is_buy && t.mint != Pubkey::default() => {
            Some((t.mint, t.fee_recipient))
        }
        DexEvent::PumpFunBuy(t) if t.mint != Pubkey::default() => Some((t.mint, t.fee_recipient)),
        DexEvent::PumpFunBuyExactSolIn(t) if t.mint != Pubkey::default() => {
            Some((t.mint, t.fee_recipient))
        }
        _ => None,
    }
}

fn pumpfun_trade_mint_quote(e: &DexEvent) -> Option<(Pubkey, Pubkey)> {
    let t = match e {
        DexEvent::PumpFunTrade(t)
        | DexEvent::PumpFunBuy(t)
        | DexEvent::PumpFunSell(t)
        | DexEvent::PumpFunBuyExactSolIn(t) => t,
        _ => return None,
    };
    if t.mint == Pubkey::default()
        || t.quote_mint == Pubkey::default()
        || is_pumpfun_solscan_sol_quote_mint(t.quote_mint)
    {
        return None;
    }
    Some((t.mint, t.quote_mint))
}

/// 扫描同签名下的买入类事件，按 mint 记录 `fee_recipient`（ShredStream 外层的 buy 已从 accounts[1] 解析）。
pub fn enrich_create_observed_fee_recipient(events: &mut [DexEvent]) {
    let mut mint_to_fee: HashMap<Pubkey, Pubkey> = HashMap::new();
    for e in events.iter() {
        if let Some((mint, fee)) = pumpfun_buy_like_mint_fee(e) {
            if fee != Pubkey::default() {
                mint_to_fee.entry(mint).or_insert(fee);
            }
        }
    }
    if mint_to_fee.is_empty() {
        return;
    }
    for e in events.iter_mut() {
        match e {
            DexEvent::PumpFunCreate(c) => {
                if c.observed_fee_recipient == Pubkey::default() {
                    if let Some(&f) = mint_to_fee.get(&c.mint) {
                        c.observed_fee_recipient = f;
                    }
                }
            }
            DexEvent::PumpFunCreateV2(c) => {
                if c.observed_fee_recipient == Pubkey::default() {
                    if let Some(&f) = mint_to_fee.get(&c.mint) {
                        c.observed_fee_recipient = f;
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn enrich_create_quote_mint_from_trades(events: &mut [DexEvent]) {
    let mut mint_to_quote: HashMap<Pubkey, Pubkey> = HashMap::new();
    for e in events.iter() {
        if let Some((mint, quote_mint)) = pumpfun_trade_mint_quote(e) {
            mint_to_quote.entry(mint).or_insert(quote_mint);
        }
    }
    if mint_to_quote.is_empty() {
        return;
    }
    for e in events.iter_mut() {
        match e {
            DexEvent::PumpFunCreate(c) => {
                if (c.quote_mint == Pubkey::default()
                    || is_pumpfun_solscan_sol_quote_mint(c.quote_mint))
                    && mint_to_quote.contains_key(&c.mint)
                {
                    c.quote_mint = mint_to_quote[&c.mint];
                }
            }
            DexEvent::PumpFunCreateV2(c) => {
                if (c.quote_mint == Pubkey::default()
                    || is_pumpfun_solscan_sol_quote_mint(c.quote_mint))
                    && mint_to_quote.contains_key(&c.mint)
                {
                    c.quote_mint = mint_to_quote[&c.mint];
                }
            }
            _ => {}
        }
    }
}

#[inline]
fn collect_create_cashback_and_mayhem(events: &[DexEvent]) -> HashMap<Pubkey, (bool, bool)> {
    let mut m: HashMap<Pubkey, (bool, bool)> = HashMap::new();
    for e in events {
        match e {
            DexEvent::PumpFunCreateV2(c) if c.mint != Pubkey::default() => {
                m.entry(c.mint).or_insert((c.is_cashback_enabled, c.is_mayhem_mode));
            }
            DexEvent::PumpFunCreate(c) if c.mint != Pubkey::default() => {
                m.entry(c.mint).or_insert((c.is_cashback_enabled, c.is_mayhem_mode));
            }
            _ => {}
        }
    }
    m
}

#[inline]
fn collect_create_events_by_mint(events: &[DexEvent]) -> HashMap<Pubkey, PumpFunCreateTokenEvent> {
    let mut m: HashMap<Pubkey, PumpFunCreateTokenEvent> = HashMap::new();
    for e in events {
        if let DexEvent::PumpFunCreate(c) = e {
            if c.mint != Pubkey::default() {
                m.entry(c.mint).or_insert_with(|| c.clone());
            }
        }
    }
    m
}

#[inline]
fn fill_str_if_empty(to: &mut String, from: &str) {
    if to.is_empty() && !from.is_empty() {
        to.push_str(from);
    }
}

#[inline]
fn fill_pk_if_default(to: &mut Pubkey, from: Pubkey) {
    if *to == Pubkey::default() && from != Pubkey::default() {
        *to = from;
    }
}

#[inline]
fn fill_pumpfun_quote_mint_if_default(to: &mut Pubkey, from: Pubkey) {
    let from = normalize_pumpfun_quote_mint(from);
    if (*to == Pubkey::default() || is_pumpfun_solscan_sol_quote_mint(*to))
        && from != Pubkey::default()
    {
        *to = from;
    }
}

#[inline]
fn fill_u64_if_zero(to: &mut u64, from: u64) {
    if *to == 0 && from != 0 {
        *to = from;
    }
}

#[inline]
fn fill_i64_if_zero(to: &mut i64, from: i64) {
    if *to == 0 && from != 0 {
        *to = from;
    }
}

#[inline]
fn fill_create_v2_from_create(
    create_v2: &mut PumpFunCreateV2TokenEvent,
    create: &PumpFunCreateTokenEvent,
) {
    fill_str_if_empty(&mut create_v2.name, &create.name);
    fill_str_if_empty(&mut create_v2.symbol, &create.symbol);
    fill_str_if_empty(&mut create_v2.uri, &create.uri);
    fill_pk_if_default(&mut create_v2.bonding_curve, create.bonding_curve);
    fill_pk_if_default(&mut create_v2.user, create.user);
    fill_pk_if_default(&mut create_v2.creator, create.creator);
    fill_pk_if_default(&mut create_v2.token_program, create.token_program);
    fill_pumpfun_quote_mint_if_default(&mut create_v2.quote_mint, create.quote_mint);
    fill_i64_if_zero(&mut create_v2.timestamp, create.timestamp);
    fill_u64_if_zero(&mut create_v2.virtual_token_reserves, create.virtual_token_reserves);
    fill_u64_if_zero(&mut create_v2.virtual_sol_reserves, create.virtual_sol_reserves);
    fill_u64_if_zero(&mut create_v2.real_token_reserves, create.real_token_reserves);
    fill_u64_if_zero(&mut create_v2.token_total_supply, create.token_total_supply);
    fill_u64_if_zero(&mut create_v2.virtual_quote_reserves, create.virtual_quote_reserves);
    create_v2.is_mayhem_mode |= create.is_mayhem_mode;
    create_v2.is_cashback_enabled |= create.is_cashback_enabled;
}

/// Copy the official `CreateEvent` payload onto the same-mint `create_v2` instruction event.
/// The `create_v2` instruction itself carries accounts and flags; the event payload carries the
/// initial reserves and quote mint used by quote-denominated pools.
pub fn enrich_create_v2_from_create_events(events: &mut [DexEvent]) {
    let creates = collect_create_events_by_mint(events);
    if creates.is_empty() {
        return;
    }

    for e in events.iter_mut() {
        if let DexEvent::PumpFunCreateV2(create_v2) = e {
            if create_v2.mint == Pubkey::default() {
                continue;
            }
            if let Some(create) = creates.get(&create_v2.mint) {
                fill_create_v2_from_create(create_v2, create);
            }
        }
    }
}

#[inline]
fn trade_event_mut(e: &mut DexEvent) -> Option<&mut PumpFunTradeEvent> {
    match e {
        DexEvent::PumpFunTrade(t)
        | DexEvent::PumpFunBuy(t)
        | DexEvent::PumpFunSell(t)
        | DexEvent::PumpFunBuyExactSolIn(t) => Some(t),
        _ => None,
    }
}

/// 将同笔交易中 Create / CreateV2 的 `is_cashback_enabled`、`is_mayhem_mode` 并入 Trade 事件（与官方 `create_v2` / bonding 语义一致）。
pub fn enrich_pumpfun_trades_from_create_instructions(events: &mut [DexEvent]) {
    let flags = collect_create_cashback_and_mayhem(events);
    if flags.is_empty() {
        return;
    }
    for e in events.iter_mut() {
        if let Some(t) = trade_event_mut(e) {
            if t.mint == Pubkey::default() {
                continue;
            }
            let Some(&(cb_en, mayhem_create)) = flags.get(&t.mint) else {
                continue;
            };
            t.is_cashback_coin |= cb_en;
            t.mayhem_mode |= mayhem_create;
            if cb_en {
                t.track_volume = true;
            }
        }
    }
}

/// 合并调用：fee 回填 + Create→Trade 标志（gRPC / Shred 在 `merge` 之后调用一次即可）。
pub fn enrich_pumpfun_same_tx_post_merge(events: &mut [DexEvent]) {
    enrich_create_v2_from_create_events(events);
    enrich_create_observed_fee_recipient(events);
    enrich_create_quote_mint_from_trades(events);
    enrich_pumpfun_trades_from_create_instructions(events);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{EventMetadata, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT};
    use solana_sdk::signature::Signature;

    #[test]
    fn enrich_fills_create_from_same_tx_buy() {
        let sig = Signature::default();
        let mint = Pubkey::new_unique();
        let fee = Pubkey::new_unique();
        let meta = EventMetadata {
            signature: sig,
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let mut events: Vec<DexEvent> = vec![
            DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: meta.clone(),
                mint,
                ..Default::default()
            }),
            DexEvent::PumpFunTrade(PumpFunTradeEvent {
                metadata: meta,
                mint,
                fee_recipient: fee,
                is_buy: true,
                ..Default::default()
            }),
        ];
        enrich_create_observed_fee_recipient(&mut events);
        if let DexEvent::PumpFunCreate(c) = &events[0] {
            assert_eq!(c.observed_fee_recipient, fee);
        } else {
            panic!("expected Create");
        }
    }

    #[test]
    fn enrich_merges_cashback_and_mayhem_from_create_v2_to_trade() {
        let sig = Signature::default();
        let mint = Pubkey::new_unique();
        let meta = EventMetadata {
            signature: sig,
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let mut events: Vec<DexEvent> = vec![
            DexEvent::PumpFunCreateV2(PumpFunCreateV2TokenEvent {
                metadata: meta.clone(),
                mint,
                is_mayhem_mode: true,
                is_cashback_enabled: true,
                ..Default::default()
            }),
            DexEvent::PumpFunTrade(PumpFunTradeEvent {
                metadata: meta,
                mint,
                mayhem_mode: false,
                is_cashback_coin: false,
                track_volume: false,
                ..Default::default()
            }),
        ];
        enrich_pumpfun_same_tx_post_merge(&mut events);
        if let DexEvent::PumpFunTrade(t) = &events[1] {
            assert!(t.mayhem_mode);
            assert!(t.is_cashback_coin);
            assert!(t.track_volume);
        } else {
            panic!("expected trade");
        }
    }

    #[test]
    fn enrich_copies_quote_pool_create_payload_to_create_v2() {
        let sig = Signature::default();
        let mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let token_program = Pubkey::new_unique();
        let meta = EventMetadata {
            signature: sig,
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let mut events: Vec<DexEvent> = vec![
            DexEvent::PumpFunCreateV2(PumpFunCreateV2TokenEvent {
                metadata: meta.clone(),
                mint,
                ..Default::default()
            }),
            DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: meta,
                name: "USD Coin Pool".to_string(),
                symbol: "USDCX".to_string(),
                uri: "https://example.invalid/token.json".to_string(),
                mint,
                timestamp: 42,
                virtual_token_reserves: 1_073_000_000_000_000,
                virtual_sol_reserves: 0,
                real_token_reserves: 793_100_000_000_000,
                token_total_supply: 1_000_000_000_000_000,
                token_program,
                is_cashback_enabled: true,
                quote_mint,
                virtual_quote_reserves: 4_292_000_000,
                ..Default::default()
            }),
        ];

        enrich_create_v2_from_create_events(&mut events);

        if let DexEvent::PumpFunCreateV2(c) = &events[0] {
            assert_eq!(c.quote_mint, quote_mint);
            assert_eq!(c.virtual_quote_reserves, 4_292_000_000);
            assert_eq!(c.token_program, token_program);
            assert!(c.is_cashback_enabled);
            assert_eq!(c.name, "USD Coin Pool");
        } else {
            panic!("expected CreateV2");
        }
    }

    #[test]
    fn enrich_replaces_create_sol_placeholder_with_trade_quote_mint() {
        let sig = Signature::default();
        let mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let meta = EventMetadata {
            signature: sig,
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let mut events: Vec<DexEvent> = vec![
            DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
                metadata: meta.clone(),
                mint,
                quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
                ..Default::default()
            }),
            DexEvent::PumpFunBuy(PumpFunTradeEvent {
                metadata: meta,
                mint,
                quote_mint,
                ..Default::default()
            }),
        ];

        enrich_create_quote_mint_from_trades(&mut events);

        if let DexEvent::PumpFunCreate(c) = &events[0] {
            assert_eq!(c.quote_mint, quote_mint);
        } else {
            panic!("expected Create");
        }
    }
}
