//! Yellowstone gRPC 单笔交易解析会跑 **log** 与 **instruction** 两路，结果直接拼接，
//! 同一链上事实可能被输出成两条 `DexEvent`。本模块在合并阶段按「业务指纹」去重，
//! 并对同指纹事件做 **log 优先、ix 补充** 的字段合并（见 `merge_grpc_instruction_into_log`）。
//!
//! **去重键必须能区分「不同用户」**：交易类事件用 **`user`（钱包）**、池子 / mint、买卖方向等；
//! **刻意不包含成交量**：instruction 侧数值可能与程序日志不一致，若用金额做键会导致 log/ix 无法配对，
//! 合并后仍以 **log 数值为准**（见 [`crate::core::merger::merge_grpc_instruction_into_log`]）。
//!
//! **同签多笔**：同一 `(mint, user, is_buy, ix_lane)` 可能出现多次（例如捆绑里同一钱包连买两笔）。
//! PumpFun 键上增加 **`lane_occurrence`**（在本路 `log_events` / `instr_events` 各自列表中的出现次序，从 0 递增），
//! 与 log、ix 两路各自遍历顺序一致时，仍能与首条 log 正确配对合并。
//!
//! **合并策略**：先收录 **log** 侧事件，再对同指纹的 **instruction** 侧调用
//! `merge_grpc_instruction_into_log` —— **以日志为权威**，指令只补缺账户等字段。

use std::collections::HashMap;

use solana_sdk::pubkey::Pubkey;

use crate::core::events::DexEvent;

#[derive(Clone, Hash, PartialEq, Eq)]
enum LogInstrDedupKey {
    PumpFunTrade {
        mint: Pubkey,
        user: Pubkey,
        is_buy: bool,
        /// 指令种类桶：`0=buy/未知`、`1=sell`、`2=buy_exact_sol_in/buy_exact_quote_in`（日志侧 `ix_name` 常为空，归入 buy 桶以便与 ix 配对）。
        ix_lane: u8,
        /// 同签、同 `(mint,user,is_buy,ix_lane)` 下第几条（log 路与 ix 路各自从 0 计数）。
        lane_occurrence: u16,
    },
    PumpFunCreate {
        mint: Pubkey,
    },
    PumpFunMigrate {
        mint: Pubkey,
        pool: Pubkey,
        user: Pubkey,
    },
    RaydiumLaunchlabTrade {
        pool: Pubkey,
        user: Pubkey,
        is_buy: bool,
        occurrence: u16,
    },
    RaydiumLaunchlabPoolCreate {
        pool: Pubkey,
    },
    RaydiumLaunchlabMigrateAmm {
        old_pool: Pubkey,
        new_pool: Pubkey,
        user: Pubkey,
    },
    PumpSwapTrade {
        mint: Pubkey,
        user: Pubkey,
        is_buy: bool,
        ix_lane: u8,
    },
    PumpSwapBuy {
        pool: Pubkey,
        user: Pubkey,
    },
    PumpSwapSell {
        pool: Pubkey,
        user: Pubkey,
    },
    PumpSwapCreatePool {
        pool: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
    },
    PumpSwapLiquidityAdded {
        pool: Pubkey,
        user: Pubkey,
    },
    PumpSwapLiquidityRemoved {
        pool: Pubkey,
        user: Pubkey,
    },
    /// `sender` 可能仅 ix 填全，不参与键以免与 log 配对失败。
    RaydiumClmmSwap {
        pool: Pubkey,
        occurrence: u16,
    },
    RaydiumCpmmSwap {
        pool: Pubkey,
        occurrence: u16,
    },
    RaydiumAmmV4Swap {
        base_out: bool,
        instruction_amount: u64,
        occurrence: u16,
    },
    OrcaWhirlpoolSwap {
        whirlpool: Pubkey,
        occurrence: u16,
    },
    MeteoraDlmmSwap {
        pool: Pubkey,
        from: Pubkey,
        swap_for_y: bool,
        occurrence: u16,
    },
}

#[inline]
fn pumpfun_ix_lane(ix_name: &str) -> u8 {
    match ix_name {
        "sell" | "sell_v2" => 1,
        "buy_exact_sol_in" | "buy_exact_quote_in" | "buy_exact_quote_in_v2" => 2,
        _ => 0,
    }
}

#[inline]
fn pumpswap_ix_lane(ix_name: &str) -> u8 {
    pumpfun_ix_lane(ix_name)
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum OccurrenceBase {
    PumpFun { mint: Pubkey, user: Pubkey, is_buy: bool, lane: u8 },
    RaydiumLaunchlab { pool: Pubkey, user: Pubkey, is_buy: bool },
    RaydiumClmm(Pubkey),
    RaydiumCpmm(Pubkey),
    RaydiumAmmV4 { base_out: bool, amount: u64 },
    OrcaWhirlpool(Pubkey),
    MeteoraDlmm { pool: Pubkey, from: Pubkey, swap_for_y: bool },
}

#[inline]
fn next_occurrence(base: OccurrenceBase, counts: &mut HashMap<OccurrenceBase, u16>) -> u16 {
    let entry = counts.entry(base).or_insert(0);
    let occurrence = *entry;
    *entry = occurrence.saturating_add(1);
    occurrence
}

#[inline]
fn pumpfun_trade_key_with_occ(
    t: &crate::core::events::PumpFunTradeEvent,
    lane_occurrence: u16,
) -> LogInstrDedupKey {
    LogInstrDedupKey::PumpFunTrade {
        mint: t.mint,
        user: t.user,
        is_buy: t.is_buy,
        ix_lane: pumpfun_ix_lane(t.ix_name.as_str()),
        lane_occurrence,
    }
}

/// 非 PumpFun 买卖事件的去重键。PumpFun `Trade/Buy/Sell/BuyExactSolIn` 必须用 [`next_pumpfun_dedup_key`] 带 `lane_occurrence`。
#[inline]
fn log_instr_dedup_key(ev: &DexEvent) -> Option<LogInstrDedupKey> {
    use DexEvent::*;
    match ev {
        PumpFunCreate(c) => Some(LogInstrDedupKey::PumpFunCreate { mint: c.mint }),
        PumpFunCreateV2(c) => Some(LogInstrDedupKey::PumpFunCreate { mint: c.mint }),
        PumpFunMigrate(m) => {
            Some(LogInstrDedupKey::PumpFunMigrate { mint: m.mint, pool: m.pool, user: m.user })
        }
        RaydiumLaunchlabTrade(t) => Some(LogInstrDedupKey::RaydiumLaunchlabTrade {
            pool: t.pool_state,
            user: t.user,
            is_buy: t.is_buy,
            occurrence: 0,
        }),
        RaydiumLaunchlabPoolCreate(p) => {
            Some(LogInstrDedupKey::RaydiumLaunchlabPoolCreate { pool: p.pool_state })
        }
        RaydiumLaunchlabMigrateAmm(m) => Some(LogInstrDedupKey::RaydiumLaunchlabMigrateAmm {
            old_pool: m.old_pool,
            new_pool: m.new_pool,
            user: m.user,
        }),
        PumpSwapTrade(t) => Some(LogInstrDedupKey::PumpSwapTrade {
            mint: t.mint,
            user: t.user,
            is_buy: t.is_buy,
            ix_lane: pumpswap_ix_lane(t.ix_name.as_str()),
        }),
        PumpSwapBuy(b) => Some(LogInstrDedupKey::PumpSwapBuy { pool: b.pool, user: b.user }),
        PumpSwapSell(s) => Some(LogInstrDedupKey::PumpSwapSell { pool: s.pool, user: s.user }),
        PumpSwapCreatePool(c) => Some(LogInstrDedupKey::PumpSwapCreatePool {
            pool: c.pool,
            base_mint: c.base_mint,
            quote_mint: c.quote_mint,
        }),
        PumpSwapLiquidityAdded(a) => {
            Some(LogInstrDedupKey::PumpSwapLiquidityAdded { pool: a.pool, user: a.user })
        }
        PumpSwapLiquidityRemoved(r) => {
            Some(LogInstrDedupKey::PumpSwapLiquidityRemoved { pool: r.pool, user: r.user })
        }
        RaydiumClmmSwap(_) => None,
        RaydiumCpmmSwap(_) | RaydiumAmmV4Swap(_) => None,
        OrcaWhirlpoolSwap(_) => None,
        MeteoraDammV2Swap(_) => None,
        MeteoraDlmmSwap(_) => None,
        // 无稳定链上指纹或其它路径：不去重
        _ => None,
    }
}

#[inline]
fn next_dedup_key(
    ev: &DexEvent,
    occurrence_counts: &mut HashMap<OccurrenceBase, u16>,
) -> Option<LogInstrDedupKey> {
    let occurrence =
        occurrence_base(ev).map(|base| next_occurrence(base, occurrence_counts)).unwrap_or(0);
    dedup_key_with_occurrence(ev, occurrence)
}

#[inline]
fn occurrence_base(ev: &DexEvent) -> Option<OccurrenceBase> {
    use DexEvent::*;
    match ev {
        PumpFunTrade(t) | PumpFunBuy(t) | PumpFunSell(t) | PumpFunBuyExactSolIn(t) => {
            let lane = pumpfun_ix_lane(t.ix_name.as_str());
            Some(OccurrenceBase::PumpFun { mint: t.mint, user: t.user, is_buy: t.is_buy, lane })
        }
        RaydiumLaunchlabTrade(t) => Some(OccurrenceBase::RaydiumLaunchlab {
            pool: t.pool_state,
            user: t.user,
            is_buy: t.is_buy,
        }),
        RaydiumClmmSwap(s) => Some(OccurrenceBase::RaydiumClmm(s.pool_state)),
        RaydiumCpmmSwap(s) => Some(OccurrenceBase::RaydiumCpmm(s.pool_id)),
        RaydiumAmmV4Swap(s) => {
            let base_out = s.max_amount_in != 0;
            let amount = if base_out { s.amount_out } else { s.amount_in };
            Some(OccurrenceBase::RaydiumAmmV4 { base_out, amount })
        }
        OrcaWhirlpoolSwap(s) => Some(OccurrenceBase::OrcaWhirlpool(s.whirlpool)),
        MeteoraDlmmSwap(s) => Some(OccurrenceBase::MeteoraDlmm {
            pool: s.pool,
            from: s.from,
            swap_for_y: s.swap_for_y,
        }),
        _ => None,
    }
}

#[inline]
fn dedup_key_with_occurrence(ev: &DexEvent, occurrence: u16) -> Option<LogInstrDedupKey> {
    use DexEvent::*;
    match ev {
        PumpFunTrade(t) | PumpFunBuy(t) | PumpFunSell(t) | PumpFunBuyExactSolIn(t) => {
            Some(pumpfun_trade_key_with_occ(t, occurrence))
        }
        RaydiumLaunchlabTrade(t) => Some(LogInstrDedupKey::RaydiumLaunchlabTrade {
            pool: t.pool_state,
            user: t.user,
            is_buy: t.is_buy,
            occurrence,
        }),
        RaydiumClmmSwap(s) => {
            Some(LogInstrDedupKey::RaydiumClmmSwap { pool: s.pool_state, occurrence })
        }
        RaydiumCpmmSwap(s) => {
            Some(LogInstrDedupKey::RaydiumCpmmSwap { pool: s.pool_id, occurrence })
        }
        RaydiumAmmV4Swap(s) => {
            let base_out = s.max_amount_in != 0;
            let amount = if base_out { s.amount_out } else { s.amount_in };
            Some(LogInstrDedupKey::RaydiumAmmV4Swap {
                base_out,
                instruction_amount: amount,
                occurrence,
            })
        }
        OrcaWhirlpoolSwap(s) => {
            Some(LogInstrDedupKey::OrcaWhirlpoolSwap { whirlpool: s.whirlpool, occurrence })
        }
        MeteoraDlmmSwap(s) => Some(LogInstrDedupKey::MeteoraDlmmSwap {
            pool: s.pool,
            from: s.from,
            swap_for_y: s.swap_for_y,
            occurrence,
        }),
        _ => log_instr_dedup_key(ev),
    }
}

/// 合并 log + instruction 两路解析结果：**同一指纹只保留一条**；log 与 ix 同时存在时 **log 优先、ix 补充**。
pub(crate) fn dedupe_log_instruction_events(
    log_events: Vec<DexEvent>,
    instr_events: Vec<DexEvent>,
) -> Vec<DexEvent> {
    if instr_events.is_empty() || (log_events.is_empty() && instr_events.len() == 1) {
        let mut out = if instr_events.is_empty() { log_events } else { instr_events };
        crate::core::pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge(&mut out);
        return out;
    }

    if log_events.len() == 1 && instr_events.len() == 1 {
        let mut log_event = log_events.into_iter().next().expect("length checked");
        let instr_event = instr_events.into_iter().next().expect("length checked");
        let same_key = dedup_key_with_occurrence(&log_event, 0)
            .zip(dedup_key_with_occurrence(&instr_event, 0))
            .is_some_and(|(log_key, instr_key)| log_key == instr_key);
        let mut out = Vec::with_capacity(if same_key { 1 } else { 2 });
        if same_key {
            crate::core::merger::merge_grpc_instruction_into_log(&mut log_event, instr_event);
            out.push(log_event);
        } else {
            out.push(log_event);
            out.push(instr_event);
        }
        crate::core::pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge(&mut out);
        return out;
    }

    let cap = log_events.len().saturating_add(instr_events.len());
    let mut out: Vec<DexEvent> = Vec::with_capacity(cap);
    let mut idx_by_key: HashMap<LogInstrDedupKey, usize> = HashMap::new();
    let mut log_occurrences: HashMap<OccurrenceBase, u16> = HashMap::new();

    for e in log_events {
        if let Some(k) = next_dedup_key(&e, &mut log_occurrences) {
            idx_by_key.insert(k, out.len());
            out.push(e);
        } else {
            out.push(e);
        }
    }

    let mut ix_occurrences: HashMap<OccurrenceBase, u16> = HashMap::new();
    for e in instr_events {
        if let Some(k) = next_dedup_key(&e, &mut ix_occurrences) {
            if let Some(&idx) = idx_by_key.get(&k) {
                crate::core::merger::merge_grpc_instruction_into_log(&mut out[idx], e);
            } else {
                idx_by_key.insert(k, out.len());
                out.push(e);
            }
        } else {
            out.push(e);
        }
    }
    crate::core::pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{
        EventMetadata, PumpFunCreateTokenEvent, PumpFunCreateV2TokenEvent, PumpFunTradeEvent,
        PumpSwapCreatePoolEvent, RaydiumLaunchlabTradeEvent, TradeDirection,
    };
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    fn dummy_meta() -> EventMetadata {
        EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        }
    }

    #[test]
    fn pumpfun_exact_quote_short_name_uses_exact_buy_lane() {
        assert_eq!(pumpfun_ix_lane("buy_exact_sol_in"), 2);
        assert_eq!(pumpfun_ix_lane("buy_exact_quote_in"), 2);
        assert_eq!(pumpfun_ix_lane("buy_exact_quote_in_v2"), 2);
    }

    #[test]
    fn pumpfun_log_ix_duplicate_collapses() {
        let mint = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let creator = Pubkey::new_unique();

        let t1 = PumpFunTradeEvent {
            metadata: dummy_meta(),
            mint,
            user,
            creator,
            sol_amount: 1_000,
            token_amount: 2_000,
            is_buy: true,
            ix_name: "buy".to_string(),
            ..Default::default()
        };

        let mut t2 = t1.clone();
        t2.sol_amount = 9_999_999; // 模拟 ix 侧金额与日志不一致（应保留日志）
        t2.amount = 2_000;
        t2.max_sol_cost = 9_999_999;
        t2.bonding_curve = Pubkey::new_unique(); // ix 补充账户
        let bc = t2.bonding_curve;

        let log = vec![DexEvent::PumpFunTrade(t1)];
        let ix = vec![DexEvent::PumpFunBuy(t2)];
        let merged = dedupe_log_instruction_events(log, ix);
        assert_eq!(merged.len(), 1, "log+ix 同一笔买卖应合并为 1 条事件");
        match &merged[0] {
            DexEvent::PumpFunTrade(t) => {
                assert_eq!(t.bonding_curve, bc);
                assert_eq!(t.sol_amount, 1_000, "应保留日志侧金额");
                assert_eq!(t.amount, 2_000, "应补齐 ix 侧 amount");
                assert_eq!(t.max_sol_cost, 9_999_999, "应补齐 ix 侧 max_sol_cost");
            }
            e => panic!("expected PumpFunTrade (保留 log 变体), got {:?}", e),
        }
    }

    fn clmm_swap(pool: Pubkey, zero_for_one: bool, amount_0: u64) -> DexEvent {
        DexEvent::RaydiumClmmSwap(crate::core::events::RaydiumClmmSwapEvent {
            metadata: dummy_meta(),
            pool_state: pool,
            sender: Pubkey::default(),
            token_account_0: Pubkey::default(),
            token_account_1: Pubkey::default(),
            amount_0,
            transfer_fee_0: 0,
            amount_1: 0,
            transfer_fee_1: 0,
            zero_for_one,
            sqrt_price_x64: 0,
            liquidity: 0,
            tick: 0,
        })
    }

    #[test]
    fn clmm_log_ix_dedup_ignores_instruction_placeholder_direction() {
        let pool = Pubkey::new_unique();
        let merged = dedupe_log_instruction_events(
            vec![clmm_swap(pool, false, 123)],
            vec![clmm_swap(pool, true, 0)],
        );
        assert_eq!(merged.len(), 1);
        match &merged[0] {
            DexEvent::RaydiumClmmSwap(s) => {
                assert_eq!(s.amount_0, 123);
                assert!(!s.zero_for_one);
            }
            other => panic!("expected RaydiumClmmSwap, got {other:?}"),
        }
    }

    #[test]
    fn clmm_same_pool_occurrences_are_retained() {
        let pool = Pubkey::new_unique();
        let merged = dedupe_log_instruction_events(
            vec![clmm_swap(pool, false, 1), clmm_swap(pool, true, 2)],
            vec![clmm_swap(pool, true, 0), clmm_swap(pool, false, 0)],
        );
        assert_eq!(merged.len(), 2);
    }

    fn launchlab_trade(pool: Pubkey, user: Pubkey, amount_in: u64, quote_mint: Pubkey) -> DexEvent {
        DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
            metadata: dummy_meta(),
            pool_state: pool,
            user,
            amount_in,
            amount_out: amount_in.saturating_mul(2),
            is_buy: true,
            trade_direction: TradeDirection::Buy,
            exact_in: true,
            global_config: Pubkey::default(),
            platform_config: Pubkey::default(),
            user_base_token: Pubkey::default(),
            user_quote_token: Pubkey::default(),
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            base_mint: Pubkey::default(),
            quote_mint,
            base_token_program: Pubkey::default(),
            quote_token_program: Pubkey::default(),
        })
    }

    #[test]
    fn launchlab_same_pool_occurrences_are_retained_and_enriched_in_order() {
        let pool = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let first_quote_mint = Pubkey::new_unique();
        let second_quote_mint = Pubkey::new_unique();
        let merged = dedupe_log_instruction_events(
            vec![
                launchlab_trade(pool, user, 100, Pubkey::default()),
                launchlab_trade(pool, user, 200, Pubkey::default()),
            ],
            vec![
                launchlab_trade(pool, user, 9_999, first_quote_mint),
                launchlab_trade(pool, user, 8_888, second_quote_mint),
            ],
        );

        assert_eq!(merged.len(), 2);
        let DexEvent::RaydiumLaunchlabTrade(first) = &merged[0] else {
            unreachable!();
        };
        let DexEvent::RaydiumLaunchlabTrade(second) = &merged[1] else {
            unreachable!();
        };
        assert_eq!(first.amount_in, 100);
        assert_eq!(first.quote_mint, first_quote_mint);
        assert_eq!(second.amount_in, 200);
        assert_eq!(second.quote_mint, second_quote_mint);
    }

    #[test]
    fn pumpfun_create_and_create_v2_same_mint_collapse() {
        let mint = Pubkey::new_unique();
        let bonding_curve = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let token_program = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let log_create = PumpFunCreateTokenEvent {
            metadata: dummy_meta(),
            name: "Token".to_string(),
            symbol: "TOK".to_string(),
            uri: "https://example.invalid/token.json".to_string(),
            mint,
            virtual_token_reserves: 1,
            ..Default::default()
        };
        let ix_create_v2 = PumpFunCreateV2TokenEvent {
            metadata: dummy_meta(),
            mint,
            bonding_curve,
            user,
            creator,
            token_program,
            quote_mint,
            virtual_quote_reserves: 2,
            is_mayhem_mode: true,
            is_cashback_enabled: true,
            ..Default::default()
        };

        let merged = dedupe_log_instruction_events(
            vec![DexEvent::PumpFunCreate(log_create)],
            vec![DexEvent::PumpFunCreateV2(ix_create_v2)],
        );

        assert_eq!(merged.len(), 1, "Create log + create_v2 ix for the same mint must emit once");
        match &merged[0] {
            DexEvent::PumpFunCreate(e) => {
                assert_eq!(e.mint, mint);
                assert_eq!(e.bonding_curve, bonding_curve);
                assert_eq!(e.user, user);
                assert_eq!(e.creator, creator);
                assert_eq!(e.token_program, token_program);
                assert_eq!(e.quote_mint, quote_mint);
                assert_eq!(e.virtual_token_reserves, 1);
                assert_eq!(e.virtual_quote_reserves, 2);
                assert!(e.is_mayhem_mode);
                assert!(e.is_cashback_enabled);
            }
            other => panic!("expected PumpFunCreate canonical event, got {other:?}"),
        }
    }

    #[test]
    fn pumpswap_create_pool_log_merge_keeps_instruction_cashback_flag() {
        let pool = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let coin_creator = Pubkey::new_unique();

        let log_create = PumpSwapCreatePoolEvent {
            metadata: dummy_meta(),
            pool,
            base_mint,
            quote_mint,
            is_cashback_coin: false,
            ..Default::default()
        };
        let ix_create = PumpSwapCreatePoolEvent {
            metadata: dummy_meta(),
            pool,
            base_mint,
            quote_mint,
            coin_creator,
            is_cashback_coin: true,
            ..Default::default()
        };

        let merged = dedupe_log_instruction_events(
            vec![DexEvent::PumpSwapCreatePool(log_create)],
            vec![DexEvent::PumpSwapCreatePool(ix_create)],
        );

        assert_eq!(merged.len(), 1);
        match &merged[0] {
            DexEvent::PumpSwapCreatePool(e) => {
                assert_eq!(e.pool, pool);
                assert_eq!(e.base_mint, base_mint);
                assert_eq!(e.quote_mint, quote_mint);
                assert_eq!(e.coin_creator, coin_creator);
                assert!(e.is_cashback_coin);
            }
            other => panic!("expected PumpSwapCreatePool, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_same_user_two_buys_log_ix_pairs_merge() {
        let mint = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let bc1 = Pubkey::new_unique();
        let bc2 = Pubkey::new_unique();

        let l1 = PumpFunTradeEvent {
            metadata: dummy_meta(),
            mint,
            user,
            creator,
            sol_amount: 100,
            token_amount: 200,
            is_buy: true,
            ix_name: "buy".to_string(),
            ..Default::default()
        };

        let mut l2 = l1.clone();
        l2.sol_amount = 300;
        l2.token_amount = 400;

        let mut i1 = l1.clone();
        i1.sol_amount = 9_999;
        i1.bonding_curve = bc1;
        let mut i2 = l2.clone();
        i2.sol_amount = 8_888;
        i2.bonding_curve = bc2;

        let merged = dedupe_log_instruction_events(
            vec![DexEvent::PumpFunTrade(l1), DexEvent::PumpFunTrade(l2)],
            vec![DexEvent::PumpFunBuy(i1), DexEvent::PumpFunBuy(i2)],
        );
        assert_eq!(merged.len(), 2);
        match (&merged[0], &merged[1]) {
            (DexEvent::PumpFunTrade(a), DexEvent::PumpFunTrade(b)) => {
                assert_eq!(a.sol_amount, 100);
                assert_eq!(a.bonding_curve, bc1);
                assert_eq!(b.sol_amount, 300);
                assert_eq!(b.bonding_curve, bc2);
            }
            e => panic!("expected two PumpFunTrade, got {:?}", e),
        }
    }

    #[test]
    fn pumpfun_same_user_two_buys_in_one_tx_both_kept() {
        let mint = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let first = PumpFunTradeEvent {
            metadata: dummy_meta(),
            mint,
            user,
            sol_amount: 1_000_000,
            token_amount: 100,
            is_buy: true,
            ix_name: "buy".to_string(),
            ..Default::default()
        };

        let mut second = first.clone();
        second.sol_amount = 2_000_000;
        second.token_amount = 150;

        let merged = dedupe_log_instruction_events(
            vec![DexEvent::PumpFunBuy(first), DexEvent::PumpFunBuy(second)],
            vec![],
        );
        assert_eq!(merged.len(), 2, "同钱包同 mint 连买两笔不得被压成一条");
    }

    #[test]
    fn pumpfun_two_distinct_users_same_amounts_not_merged() {
        let mint = Pubkey::new_unique();
        let u1 = Pubkey::new_unique();
        let u2 = Pubkey::new_unique();

        let a = PumpFunTradeEvent {
            metadata: dummy_meta(),
            mint,
            user: u1,
            sol_amount: 100,
            token_amount: 200,
            is_buy: true,
            ..Default::default()
        };

        let mut b = a.clone();
        b.user = u2;

        let merged = dedupe_log_instruction_events(
            vec![DexEvent::PumpFunBuy(a)],
            vec![DexEvent::PumpFunBuy(b)],
        );
        assert_eq!(merged.len(), 2, "不同 user 即使金额相同也不得合并");
    }
}
