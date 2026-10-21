use sol_parser_sdk::{core::events::*, stock_prices::*, DexEvent};
use solana_sdk::pubkey::Pubkey;

const NOW: i64 = 1_000_000_000;

#[test]
fn confirmed_dlmm_instruction_limits_are_not_execution_prices() {
    let catalog = StockTokenCatalog::screened_defaults();
    let pool = catalog
        .pools
        .iter()
        .find(|p| p.dex_type == RawInstructionDexType::MeteoraDlmm)
        .unwrap()
        .clone();
    let mut tracker =
        StockPriceTracker::new(StockPriceConfig { catalog, ..Default::default() }).unwrap();
    let instruction = DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
        pool: pool.pool,
        token_x_mint: pool.mint,
        token_y_mint: pool.quote_mint,
        swap_for_y: true,
        amount_in: 100_000_000,
        amount_out: 20_000_000,
        // These are nonzero wire limits in legacy exact-out, even on success.
        event_version: 0,
        parse_status: TradeParseStatus::ExecutionConfirmed,
        metadata: EventMetadata { slot: 100, grpc_recv_us: NOW, ..Default::default() },
        ..Default::default()
    });
    assert!(tracker.process(&instruction, NOW).is_none());
}
fn setup(quote_mint: Pubkey) -> (StockPriceTracker, Pubkey, Pubkey) {
    let mint = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let tracker = StockPriceTracker::new(StockPriceConfig {
        catalog: StockTokenCatalog {
            tokens: vec![StockToken {
                mint,
                symbol: "TEST".into(),
                decimals: 8,
                catalog_source: "test".into(),
            }],
            pools: vec![StockPool {
                pool,
                mint,
                quote_mint,
                dex_type: RawInstructionDexType::RaydiumCpmm,
                catalog_fee_snapshot: None,
            }],
        },
        ..Default::default()
    })
    .unwrap();
    (tracker, mint, pool)
}
fn swap(pool: Pubkey, input: Pubkey, output: Pubkey, amount_in: u64, amount_out: u64) -> DexEvent {
    DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
        pool_id: pool,
        input_token_mint: input,
        output_token_mint: output,
        input_amount: amount_in,
        output_amount: amount_out,
        input_vault_before: 1_000_000_000,
        output_vault_before: 1_000_000_000,
        parse_status: TradeParseStatus::ExecutionConfirmed,
        metadata: EventMetadata { slot: 100, grpc_recv_us: NOW, ..Default::default() },
        ..Default::default()
    })
}
fn reference() -> DexEvent {
    DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
        whirlpool: DEFAULT_SOL_USDC_POOL,
        token_mint_a: WSOL,
        token_mint_b: USDC,
        a_to_b: true,
        post_sqrt_price: 1u128 << 64,
        input_amount: 1_000_000_000,
        output_amount: 150_000_000,
        parse_status: TradeParseStatus::ExecutionConfirmed,
        metadata: EventMetadata { slot: 99, grpc_recv_us: NOW, ..Default::default() },
        ..Default::default()
    })
}

#[test]
fn decimal_adjustment_both_directions_and_cross_conversion() {
    let (mut tracker, mint, pool) = setup(USDC);
    assert!(tracker.process(&reference(), NOW).is_none());
    for event in [
        swap(pool, USDC, mint, 300_000_000, 100_000_000),
        swap(pool, mint, USDC, 100_000_000, 300_000_000),
    ] {
        let price = tracker.process(&event, NOW).unwrap();
        assert_eq!(price.price_usdc, Some(300.0));
        assert_eq!(price.price_sol, Some(2.0));
        assert!(price.liquidity.is_none());
    }
    let (mut tracker, mint, pool) = setup(WSOL);
    tracker.process(&reference(), NOW);
    let price = tracker.process(&swap(pool, mint, WSOL, 100_000_000, 2_000_000_000), NOW).unwrap();
    assert_eq!(price.price_sol, Some(2.0));
    assert_eq!(price.price_usdc, Some(300.0));
}

#[test]
fn stale_missing_future_and_reset_reference_never_fabricate_conversion() {
    let (mut tracker, mint, pool) = setup(USDC);
    let trade = swap(pool, USDC, mint, 300_000_000, 100_000_000);
    assert!(tracker.process(&trade, NOW).unwrap().price_sol.is_none());
    let mut future = reference();
    future.metadata_mut().unwrap().slot = 101;
    tracker.process(&future, NOW);
    assert!(tracker.process(&trade, NOW).unwrap().price_sol.is_none());
    tracker.reset();
    tracker.process(&reference(), NOW);
    let mut later = trade.clone();
    later.metadata_mut().unwrap().grpc_recv_us += 31_000_000;
    assert!(tracker.process(&later, NOW + 31_000_000).unwrap().price_sol.is_none());
    tracker.reset();
    assert!(tracker.process(&trade, NOW).unwrap().price_sol.is_none());
}

#[test]
fn same_transaction_reference_is_not_used_without_cpi_order() {
    let (mut tracker, mint, pool) = setup(USDC);
    let mut reference = reference();
    reference.metadata_mut().unwrap().slot = 100;
    tracker.process(&reference, NOW);
    let trade = swap(pool, USDC, mint, 300_000_000, 100_000_000);
    assert!(tracker.process(&trade, NOW).unwrap().price_sol.is_none());
}

#[test]
fn unconfirmed_unsupported_wrong_pool_zero_and_out_of_order_are_rejected() {
    let (mut tracker, mint, pool) = setup(USDC);
    let trade = swap(pool, USDC, mint, 300_000_000, 100_000_000);
    for status in [TradeParseStatus::Unresolved, TradeParseStatus::InstructionDecoded] {
        let mut event = trade.clone();
        if let DexEvent::RaydiumCpmmSwap(e) = &mut event {
            e.parse_status = status;
        }
        assert!(tracker.process(&event, NOW).is_none());
    }
    assert!(tracker.process(&swap(pool, USDC, mint, 0, 1), NOW).is_none());
    assert!(tracker.process(&swap(Pubkey::new_unique(), USDC, mint, 1, 1), NOW).is_none());
    assert!(tracker.process(&swap(pool, Pubkey::new_unique(), mint, 1, 1), NOW).is_none());
    assert!(tracker.process(&trade, NOW).is_some());
    let mut old = trade.clone();
    old.metadata_mut().unwrap().slot = 99;
    assert!(tracker.process(&old, NOW).is_none());
    assert!(tracker.process(&trade, NOW + 31_000_000).is_none());
}

#[test]
fn liquidity_requires_explicit_units_provenance_and_freshness() {
    let (mut tracker, mint, pool) = setup(USDC);
    let snapshot = StockPoolLiquidity {
        pool,
        slot: 100,
        observed_us: NOW,
        tvl_usdc: Some(20000.0),
        buy_depth_usdc: Some(1000.0),
        sell_depth_usdc: None,
        depth_max_price_impact_bps: Some(100),
        source: "coherent account snapshot + quote".into(),
    };
    tracker.update_liquidity(snapshot.clone()).unwrap();
    let trade = swap(pool, USDC, mint, 300_000_000, 100_000_000);
    assert_eq!(tracker.process(&trade, NOW).unwrap().liquidity.unwrap().tvl_usdc, Some(20000.0));
    let mut invalid = snapshot.clone();
    invalid.depth_max_price_impact_bps = None;
    assert!(tracker.update_liquidity(invalid).is_err());
    for impact in [10_000, u16::MAX] {
        let mut invalid = snapshot.clone();
        invalid.depth_max_price_impact_bps = Some(impact);
        assert!(tracker.update_liquidity(invalid).is_err());
    }
    let mut invalid = snapshot.clone();
    invalid.source = " \t\n".into();
    assert!(tracker.update_liquidity(invalid).is_err());
    // Rejected updates must not replace the last valid snapshot.
    assert_eq!(
        tracker.process(&trade, NOW).unwrap().liquidity.unwrap().depth_max_price_impact_bps,
        Some(100)
    );
    let mut later = trade;
    later.metadata_mut().unwrap().grpc_recv_us += 31_000_000;
    assert!(tracker.process(&later, NOW + 31_000_000).unwrap().liquidity.is_none());
}

#[test]
fn default_is_disabled_and_catalog_has_multiple_identity_sources() {
    assert!(StockPriceTracker::new(Default::default()).is_ok());
    assert!(StockPriceConfig::default().catalog.tokens.is_empty());
    let catalog = StockTokenCatalog::screened_defaults();
    assert_eq!(catalog.tokens.len(), 33);
    assert_eq!(catalog.pools.len(), 58);
    assert!(catalog.tokens.iter().any(|t| t.catalog_source.contains("xstocks")));
    assert!(catalog.tokens.iter().any(|t| t.catalog_source.contains("backpack")));
    assert!(catalog.tokens.iter().any(|t| t.symbol == "SPHR.US"));
    for symbol in ["BABA.US", "INTCx", "QQQx", "GMEx", "INTC.US", "SPCX.US"] {
        assert!(catalog.tokens.iter().any(|t| t.symbol == symbol));
    }
    for symbol in ["HIMS.US", "AMD.US", "RACE.US", "RDDT.US"] {
        assert!(catalog.tokens.iter().any(|t| t.symbol == symbol));
    }
    for token in &catalog.tokens {
        let selected = catalog.preferred_pool(&token.mint).unwrap();
        let fee = selected.catalog_fee_snapshot.as_ref().unwrap();
        assert_eq!(fee.stock_transfer_tax.basis_points, 0);
        for alternative in catalog.pools.iter().filter(|p| p.mint == token.mint) {
            let other = alternative.catalog_fee_snapshot.as_ref().unwrap();
            if let Some(other_rate) = other.snapshot_start_swap_fee_bps {
                assert!(fee.snapshot_start_swap_fee_bps.is_some_and(|rate| rate <= other_rate));
            }
        }
    }
    StockPriceTracker::new(StockPriceConfig { catalog, ..Default::default() }).unwrap();
}

#[test]
fn same_slot_later_reference_cannot_price_an_earlier_transaction() {
    let (mut tracker, mint, pool) = setup(USDC);
    let mut reference = reference();
    let meta = reference.metadata_mut().unwrap();
    meta.slot = 100;
    meta.tx_index = 2;
    tracker.process(&reference, NOW);
    let trade = swap(pool, USDC, mint, 300_000_000, 100_000_000);
    assert!(tracker.process(&trade, NOW).unwrap().price_sol.is_none());
}

#[test]
fn signed_mainnet_stock_prices_match_cpi_transfers_and_rpc_grpc_agree() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use solana_sdk::transaction::VersionedTransaction;
    use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
    use yellowstone_grpc_proto::prelude::{
        SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
    };
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/stock_price_replay_20261020.json")).unwrap();
    let mut total = 0;
    for case in capture["cases"].as_array().unwrap() {
        let wire = STANDARD.decode(case["rpc"]["transaction"][0].as_str().unwrap()).unwrap();
        let signed: VersionedTransaction = wincode::deserialize(&wire).unwrap();
        assert!(signed.verify_with_results().into_iter().all(|valid| valid));
        assert_eq!(signed.signatures[0].to_string(), case["signature"].as_str().unwrap());
        let rpc: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(case["rpc"].clone()).unwrap();
        let (meta, tx) = sol_parser_sdk::convert_rpc_to_grpc(&rpc).unwrap();
        let route = sol_parser_sdk::analyze_yellowstone_transaction_routes(&tx, &meta, &[]);
        assert!(route.succeeded);
        let update = SubscribeUpdateTransaction {
            slot: rpc.slot,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: tx.signatures[0].clone(),
                transaction: Some(tx),
                meta: Some(meta),
                ..Default::default()
            }),
        };
        let grpc = sol_parser_sdk::grpc::parse_subscribe_update_transaction_low_latency(
            &update, NOW, None, None,
        );
        let mut rpc_events = sol_parser_sdk::parse_rpc_transaction(&rpc, None).unwrap();
        for e in &mut rpc_events {
            e.metadata_mut().unwrap().grpc_recv_us = NOW;
        }
        let prices = |events: &[DexEvent]| {
            let mut tracker = StockPriceTracker::new(StockPriceConfig {
                catalog: StockTokenCatalog::screened_defaults(),
                ..Default::default()
            })
            .unwrap();
            events.iter().filter_map(|e| tracker.process(e, NOW)).collect::<Vec<_>>()
        };
        let grpc_prices = prices(&grpc);
        // The historical ABNB pool was removed after a large withdrawal; its
        // raw trade still parses but must not produce a default stock price.
        let expected = usize::from(case["pool"] != "52PqYh77gmA6AbM5Hs6gHe9mhGBey5CwmD7sDybaYj27");
        assert_eq!(grpc_prices.len(), expected, "{}", case["name"]);
        let rpc_prices = prices(&rpc_events);
        assert_eq!(rpc_prices.len(), grpc_prices.len());
        for (price, other) in grpc_prices.iter().zip(&rpc_prices) {
            assert_eq!(price.price_in_quote, other.price_in_quote);
            assert_eq!(price.pool, other.pool);
            // Independent SPL transfer instructions corroborate the emitted raw
            // amounts. Route-leg amount association can be unknown for legacy
            // account layouts even though these transfer facts are available.
            assert!(route.legs.iter().any(|leg| leg.pool == price.pool));
            assert!(
                route
                    .transfers
                    .iter()
                    .any(|t| t.mint == Some(price.mint) && t.amount == price.stock_amount_raw),
                "stock {}",
                case["name"]
            );
            assert!(route.transfers.iter().any(|t|
                t.mint == Some(price.quote_mint) && t.amount == price.quote_amount_raw), "quote {}", case["name"]);
            assert!(price.price_sol.is_none());
            assert!(price.liquidity.is_none());
            total += 1;
        }
    }
    assert_eq!(total, 2);
}
