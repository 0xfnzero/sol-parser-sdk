use sol_parser_sdk::{
    core::{
        events::{PumpFunTradeEvent, PumpSwapBuyEvent, PumpSwapSellEvent},
        merger::{can_merge, try_merge_events},
    },
    DexEvent,
};
use solana_sdk::pubkey::Pubkey;

#[test]
fn distinct_pump_trades_do_not_merge() {
    let venue = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    for kind in [0, 1, 2] {
        for mismatch in ["venue", "user", "direction", "none"] {
            let base = match kind {
                0 => DexEvent::PumpFunTrade(PumpFunTradeEvent {
                    mint: venue,
                    user,
                    is_buy: true,
                    ix_name: "buy_v3".into(),
                    ..Default::default()
                }),
                1 => DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                    pool: venue,
                    user,
                    ..Default::default()
                }),
                _ => DexEvent::PumpSwapSell(PumpSwapSellEvent {
                    pool: venue,
                    user,
                    ..Default::default()
                }),
            };
            let v = if mismatch == "venue" { Pubkey::new_unique() } else { venue };
            let u = if mismatch == "user" { Pubkey::new_unique() } else { user };
            let inner = match kind {
                0 => DexEvent::PumpFunTrade(PumpFunTradeEvent {
                    mint: v,
                    user: u,
                    is_buy: mismatch != "direction",
                    ix_name: if mismatch == "direction" { "sell_v3" } else { "buy_v3" }.into(),
                    ..Default::default()
                }),
                1 if mismatch != "direction" => DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                    pool: v,
                    user: u,
                    ..Default::default()
                }),
                2 if mismatch == "direction" => DexEvent::PumpSwapBuy(PumpSwapBuyEvent {
                    pool: v,
                    user: u,
                    ..Default::default()
                }),
                _ => DexEvent::PumpSwapSell(PumpSwapSellEvent {
                    pool: v,
                    user: u,
                    ..Default::default()
                }),
            };
            let before = serde_json::to_value(&base).unwrap();
            let mut actual = base;
            let mut unmerged = None;
            assert_eq!(can_merge(&actual, &inner), mismatch == "none", "kind={kind} {mismatch}");
            assert_eq!(
                try_merge_events(&mut actual, inner.clone(), &mut unmerged),
                mismatch == "none",
                "kind={kind} {mismatch}"
            );
            if mismatch != "none" {
                assert_eq!(serde_json::to_value(&actual).unwrap(), before);
                assert_eq!(
                    serde_json::to_value(unmerged.unwrap()).unwrap(),
                    serde_json::to_value(inner).unwrap()
                );
            }
        }
    }
}
