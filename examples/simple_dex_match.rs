use solana_streamer_sdk::{
    DexEvent,
    parse_transaction_events,
};

/// Simple example showing how to use DexEvent with regular match patterns
/// This demonstrates the pattern you requested without match_event! macro
fn main() {
    println!("Simple DexEvent Pattern Matching Example");

    // This shows the basic pattern - in a real application you would:
    // 1. Set up gRPC streaming to get transaction data
    // 2. Parse each transaction to get Vec<DexEvent>
    // 3. Call handle_dex_event for each event

    // Example: simulate parsing a transaction
    let sample_transaction_data = vec![]; // This would be real transaction bytes

    // Parse transaction to get events
    match parse_transaction_events(&sample_transaction_data) {
        Ok(events) => {
            if events.is_empty() {
                println!("No events found in sample transaction (expected - empty data)");

                // Show the pattern with a sample event
                println!("\\nDemonstrating event handling pattern:");
                demonstrate_event_handling();
            } else {
                println!("Found {} events", events.len());
                for event in events {
                    handle_dex_event(event);
                }
            }
        }
        Err(e) => {
            println!("Error parsing transaction: {:?} (expected - empty data)", e);

            // Show the pattern anyway
            println!("\\nDemonstrating event handling pattern:");
            demonstrate_event_handling();
        }
    }
}

/// Demonstrate the event handling pattern without needing real events
fn demonstrate_event_handling() {
    // This is the pattern you would use in your callback
    println!("Here's how you handle DexEvent with regular match patterns:");
    println!();
    println!("fn handle_dex_event(event: DexEvent) {{");
    println!("    match event {{");
    println!("        DexEvent::PumpFunTrade(e) => {{");
    println!("            println!(\"PumpFun trade: {{:?}}\", e);");
    println!("        }},");
    println!("        DexEvent::BonkTrade(e) => {{");
    println!("            println!(\"Bonk trade: {{:?}}\", e);");
    println!("        }},");
    println!("        DexEvent::RaydiumCpmmSwap(e) => {{");
    println!("            println!(\"Raydium CPMM swap: {{:?}}\", e);");
    println!("        }},");
    println!("        // ... handle all other event types");
    println!("        _ => {{");
    println!("            println!(\"Other event type\");");
    println!("        }}");
    println!("    }}");
    println!("}}");
}

/// The main event handling function that replaces the callback pattern
/// Use this pattern instead of match_event! macro and UnifiedEvent
fn handle_dex_event(event: DexEvent) {
    println!("🎉 Event received!");

    match event {
        // -------------------------- block meta -----------------------
        DexEvent::BlockMeta(e) => {
            println!("BlockMetaEvent: handle_us={:?}", e.metadata.handle_us);
        },

        // -------------------------- bonk -----------------------
        DexEvent::BonkPoolCreate(e) => {
            println!("BonkPoolCreateEvent: pool_state={}, creator={}", e.pool_state, e.creator);
        },
        DexEvent::BonkTrade(e) => {
            println!("BonkTradeEvent: pool_state={}, user={}, is_buy={}", e.pool_state, e.user, e.is_buy);
        },
        DexEvent::BonkMigrateAmm(e) => {
            println!("BonkMigrateAmmEvent: old_pool={}, new_pool={}", e.old_pool, e.new_pool);
        },

        // -------------------------- pumpfun -----------------------
        DexEvent::PumpFunTrade(e) => {
            println!("PumpFunTradeEvent: mint={}, user={}, is_buy={}", e.mint, e.user, e.is_buy);
        },
        DexEvent::PumpFunMigrate(e) => {
            println!("PumpFunMigrateEvent: mint={}, bonding_curve={}", e.mint, e.bonding_curve);
        },
        DexEvent::PumpFunCreate(e) => {
            println!("PumpFunCreateTokenEvent: mint={}, bonding_curve={}", e.mint, e.bonding_curve);
        },
        DexEvent::PumpFunComplete(e) => {
            println!("PumpFunCompleteTokenEvent: mint={}, bonding_curve={}", e.mint, e.bonding_curve);
        },

        // -------------------------- pumpswap -----------------------
        DexEvent::PumpSwapBuy(e) => {
            println!("PumpSwapBuyEvent: pool_id={}, user={}", e.pool_id, e.user);
        },
        DexEvent::PumpSwapSell(e) => {
            println!("PumpSwapSellEvent: pool_id={}, user={}", e.pool_id, e.user);
        },
        DexEvent::PumpSwapCreatePool(e) => {
            println!("PumpSwapCreatePoolEvent: pool_id={}, creator={}", e.pool_id, e.creator);
        },

        // -------------------------- raydium_cpmm -----------------------
        DexEvent::RaydiumCpmmSwap(e) => {
            println!("RaydiumCpmmSwapEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumCpmmDeposit(e) => {
            println!("RaydiumCpmmDepositEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumCpmmWithdraw(e) => {
            println!("RaydiumCpmmWithdrawEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumCpmmInitialize(e) => {
            println!("RaydiumCpmmInitializeEvent: pool={}", e.pool);
        },

        // -------------------------- raydium_clmm -----------------------
        DexEvent::RaydiumClmmSwap(e) => {
            println!("RaydiumClmmSwapEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumClmmCreatePool(e) => {
            println!("RaydiumClmmCreatePoolEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumClmmOpenPosition(e) => {
            println!("RaydiumClmmOpenPositionEvent: pool={}, position={}", e.pool, e.position);
        },
        DexEvent::RaydiumClmmClosePosition(e) => {
            println!("RaydiumClmmClosePositionEvent: pool={}, position={}", e.pool, e.position);
        },
        DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
            println!("RaydiumClmmIncreaseLiquidityEvent: pool={}, position={}", e.pool, e.position);
        },
        DexEvent::RaydiumClmmDecreaseLiquidity(e) => {
            println!("RaydiumClmmDecreaseLiquidityEvent: pool={}, position={}", e.pool, e.position);
        },

        // -------------------------- raydium_amm_v4 -----------------------
        DexEvent::RaydiumAmmV4Swap(e) => {
            println!("RaydiumAmmV4SwapEvent: amm={}", e.amm);
        },
        DexEvent::RaydiumAmmV4Deposit(e) => {
            println!("RaydiumAmmV4DepositEvent: amm={}", e.amm);
        },
        DexEvent::RaydiumAmmV4Withdraw(e) => {
            println!("RaydiumAmmV4WithdrawEvent: amm={}", e.amm);
        },
        DexEvent::RaydiumAmmV4Initialize2(e) => {
            println!("RaydiumAmmV4Initialize2Event: amm={}", e.amm);
        },
        DexEvent::RaydiumAmmV4WithdrawPnl(e) => {
            println!("RaydiumAmmV4WithdrawPnlEvent: amm={}", e.amm);
        },

        // -------------------------- orca whirlpool -----------------------
        DexEvent::OrcaWhirlpoolSwap(e) => {
            println!("OrcaWhirlpoolSwapEvent: whirlpool={}", e.whirlpool);
        },
        DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => {
            println!("OrcaWhirlpoolLiquidityIncreasedEvent: whirlpool={}, position={}", e.whirlpool, e.position);
        },
        DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => {
            println!("OrcaWhirlpoolLiquidityDecreasedEvent: whirlpool={}, position={}", e.whirlpool, e.position);
        },
        DexEvent::OrcaWhirlpoolPoolInitialized(e) => {
            println!("OrcaWhirlpoolPoolInitializedEvent: whirlpool={}", e.whirlpool);
        },

        // -------------------------- meteora -----------------------
        DexEvent::MeteoraPoolsSwap(e) => {
            println!("MeteoraPoolsSwapEvent: in_amount={}, out_amount={}", e.in_amount, e.out_amount);
        },
        DexEvent::MeteoraPoolsAddLiquidity(e) => {
            println!("MeteoraPoolsAddLiquidityEvent: lp_mint_amount={}", e.lp_mint_amount);
        },
        DexEvent::MeteoraPoolsRemoveLiquidity(e) => {
            println!("MeteoraPoolsRemoveLiquidityEvent: lp_unmint_amount={}", e.lp_unmint_amount);
        },
        DexEvent::MeteoraPoolsBootstrapLiquidity(e) => {
            println!("MeteoraPoolsBootstrapLiquidityEvent: pool={}", e.pool);
        },
        DexEvent::MeteoraPoolsPoolCreated(e) => {
            println!("MeteoraPoolsPoolCreatedEvent: pool={}", e.pool);
        },
        DexEvent::MeteoraPoolsSetPoolFees(e) => {
            println!("MeteoraPoolsSetPoolFeesEvent: pool={}", e.pool);
        },
        DexEvent::MeteoraDammV2Swap(e) => {
            println!("MeteoraDammV2SwapEvent: lb_pair={}", e.lb_pair);
        },
        DexEvent::MeteoraDammV2AddLiquidity(e) => {
            println!("MeteoraDammV2AddLiquidityEvent: lb_pair={}, position={}", e.lb_pair, e.position);
        },
        DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
            println!("MeteoraDammV2RemoveLiquidityEvent: lb_pair={}, position={}", e.lb_pair, e.position);
        },
        DexEvent::MeteoraDammV2InitializePool(e) => {
            println!("MeteoraDammV2InitializePoolEvent: lb_pair={}", e.lb_pair);
        },
        DexEvent::MeteoraDammV2CreatePosition(e) => {
            println!("MeteoraDammV2CreatePositionEvent: position={}", e.position);
        },
        DexEvent::MeteoraDammV2ClosePosition(e) => {
            println!("MeteoraDammV2ClosePositionEvent: position={}", e.position);
        },
        DexEvent::MeteoraDammV2ClaimPositionFee(e) => {
            println!("MeteoraDammV2ClaimPositionFeeEvent: position={}", e.position);
        },
        DexEvent::MeteoraDammV2InitializeReward(e) => {
            println!("MeteoraDammV2InitializeRewardEvent: lb_pair={}", e.lb_pair);
        },
        DexEvent::MeteoraDammV2FundReward(e) => {
            println!("MeteoraDammV2FundRewardEvent: lb_pair={}", e.lb_pair);
        },
        DexEvent::MeteoraDammV2ClaimReward(e) => {
            println!("MeteoraDammV2ClaimRewardEvent: position={}", e.position);
        },

        // -------------------------- other events -----------------------
        DexEvent::PumpSwapPoolCreated(e) => {
            println!("PumpSwapPoolCreated: pool_id={}", e.pool_id);
        },
        DexEvent::PumpSwapTrade(e) => {
            println!("PumpSwapTrade: pool_id={}", e.pool_id);
        },
        DexEvent::PumpSwapLiquidityAdded(e) => {
            println!("PumpSwapLiquidityAdded: pool_id={}", e.pool_id);
        },
        DexEvent::PumpSwapLiquidityRemoved(e) => {
            println!("PumpSwapLiquidityRemoved: pool_id={}", e.pool_id);
        },
        DexEvent::PumpSwapPoolUpdated(e) => {
            println!("PumpSwapPoolUpdated: pool_id={}", e.pool_id);
        },
        DexEvent::PumpSwapFeesClaimed(e) => {
            println!("PumpSwapFeesClaimed: pool_id={}", e.pool_id);
        },
        DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => {
            println!("RaydiumClmmOpenPositionWithTokenExtNftEvent: pool={}", e.pool);
        },
        DexEvent::RaydiumClmmCollectFee(e) => {
            println!("RaydiumClmmCollectFeeEvent: pool={}", e.pool);
        },

        // -------------------------- account events -----------------------
        DexEvent::TokenAccount(e) => {
            println!("TokenAccountEvent: mint={}, owner={}", e.mint, e.owner);
        },
        DexEvent::NonceAccount(e) => {
            println!("NonceAccountEvent: pubkey={}, authority={}", e.pubkey, e.authority);
        },
        DexEvent::TokenInfo(e) => {
            println!("TokenInfoEvent: mint={}", e.mint);
        },

        // -------------------------- error events -----------------------
        DexEvent::Error(e) => {
            println!("Error event: {}", e);
        },
    }
}

// How you might integrate this with a streaming setup
// (This is conceptual - you would implement based on your streaming infrastructure)
#[allow(dead_code)]
fn streaming_integration_example() {
    println!("\\nStreaming Integration Pattern:");
    println!("// In your streaming callback or event loop:");
    println!("let transaction_data = get_transaction_from_stream(); // Your gRPC data");
    println!("match parse_transaction_events(&transaction_data) {{");
    println!("    Ok(events) => {{");
    println!("        for event in events {{");
    println!("            handle_dex_event(event); // Use the function above");
    println!("        }}");
    println!("    }}");
    println!("    Err(e) => eprintln!(\"Parse error: {{:?}}\", e),");
    println!("}}");
}