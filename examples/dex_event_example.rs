use solana_streamer_sdk::{
    DexEvent,
    // Note: This example shows the pattern for handling DexEvent with match
    // The actual streaming implementation would need to be added separately
};

fn main() {
    println!("DexEvent Pattern Matching Example");

    // This example demonstrates how to handle DexEvent using match patterns
    // instead of the match_event! macro

    // Example: simulate receiving some events
    let sample_events = create_sample_events();

    for event in sample_events {
        handle_dex_event(event);
    }

    println!("Example completed!");
}

// Create some sample events for demonstration
fn create_sample_events() -> Vec<DexEvent> {
    // Note: In a real implementation, you would get these events from
    // your streaming/parsing infrastructure
    vec![
        // You would create actual events here based on parsed transaction data
    ]
}

// The main event handling function - replaces the callback pattern
fn handle_dex_event(event: DexEvent) {
    println!("🎉 Event received!");

    match event {
            // -------------------------- block meta -----------------------
            DexEvent::BlockMeta(e) => {
                println!("BlockMetaEvent: {:?}", e.metadata.handle_us);
            },

            // -------------------------- bonk -----------------------
            DexEvent::BonkPoolCreate(e) => {
                // When using grpc, you can get block_time from each event
                println!("block_time: {:?}, block_time_ms: {:?}", e.metadata.block_time, e.metadata.block_time_ms);
                println!("BonkPoolCreateEvent: {:?}", e.base_mint_param.symbol);
            },
            DexEvent::BonkTrade(e) => {
                println!("BonkTradeEvent: {e:?}");
            },
            DexEvent::BonkMigrateAmm(e) => {
                println!("BonkMigrateAmmEvent: {e:?}");
            },

            // -------------------------- pumpfun -----------------------
            DexEvent::PumpFunTrade(e) => {
                println!("PumpFunTradeEvent: {e:?}");
            },
            DexEvent::PumpFunMigrate(e) => {
                println!("PumpFunMigrateEvent: {e:?}");
            },
            DexEvent::PumpFunCreate(e) => {
                println!("PumpFunCreateTokenEvent: {e:?}");
            },
            DexEvent::PumpFunComplete(e) => {
                println!("PumpFunCompleteTokenEvent: {e:?}");
            },

            // -------------------------- pumpswap -----------------------
            DexEvent::PumpSwapBuy(e) => {
                println!("PumpSwapBuy event: {e:?}");
            },
            DexEvent::PumpSwapSell(e) => {
                println!("PumpSwapSell event: {e:?}");
            },
            DexEvent::PumpSwapCreatePool(e) => {
                println!("PumpSwapCreatePool event: {e:?}");
            },
            DexEvent::PumpSwapPoolCreated(e) => {
                println!("PumpSwapPoolCreated event: {e:?}");
            },
            DexEvent::PumpSwapTrade(e) => {
                println!("PumpSwapTrade event: {e:?}");
            },
            DexEvent::PumpSwapLiquidityAdded(e) => {
                println!("PumpSwapLiquidityAdded event: {e:?}");
            },
            DexEvent::PumpSwapLiquidityRemoved(e) => {
                println!("PumpSwapLiquidityRemoved event: {e:?}");
            },
            DexEvent::PumpSwapPoolUpdated(e) => {
                println!("PumpSwapPoolUpdated event: {e:?}");
            },
            DexEvent::PumpSwapFeesClaimed(e) => {
                println!("PumpSwapFeesClaimed event: {e:?}");
            },

            // -------------------------- raydium_cpmm -----------------------
            DexEvent::RaydiumCpmmSwap(e) => {
                println!("RaydiumCpmmSwapEvent: {e:?}");
            },
            DexEvent::RaydiumCpmmDeposit(e) => {
                println!("RaydiumCpmmDepositEvent: {e:?}");
            },
            DexEvent::RaydiumCpmmInitialize(e) => {
                println!("RaydiumCpmmInitializeEvent: {e:?}");
            },
            DexEvent::RaydiumCpmmWithdraw(e) => {
                println!("RaydiumCpmmWithdrawEvent: {e:?}");
            },

            // -------------------------- raydium_clmm -----------------------
            DexEvent::RaydiumClmmSwap(e) => {
                println!("RaydiumClmmSwapEvent: {e:?}");
            },
            DexEvent::RaydiumClmmClosePosition(e) => {
                println!("RaydiumClmmClosePositionEvent: {e:?}");
            },
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => {
                println!("RaydiumClmmDecreaseLiquidityEvent: {e:?}");
            },
            DexEvent::RaydiumClmmCreatePool(e) => {
                println!("RaydiumClmmCreatePoolEvent: {e:?}");
            },
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
                println!("RaydiumClmmIncreaseLiquidityEvent: {e:?}");
            },
            DexEvent::RaydiumClmmOpenPosition(e) => {
                println!("RaydiumClmmOpenPositionEvent: {e:?}");
            },
            DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => {
                println!("RaydiumClmmOpenPositionWithTokenExtNftEvent: {e:?}");
            },
            DexEvent::RaydiumClmmCollectFee(e) => {
                println!("RaydiumClmmCollectFeeEvent: {e:?}");
            },

            // -------------------------- raydium_amm_v4 -----------------------
            DexEvent::RaydiumAmmV4Swap(e) => {
                println!("RaydiumAmmV4SwapEvent: {e:?}");
            },
            DexEvent::RaydiumAmmV4Deposit(e) => {
                println!("RaydiumAmmV4DepositEvent: {e:?}");
            },
            DexEvent::RaydiumAmmV4Initialize2(e) => {
                println!("RaydiumAmmV4Initialize2Event: {e:?}");
            },
            DexEvent::RaydiumAmmV4Withdraw(e) => {
                println!("RaydiumAmmV4WithdrawEvent: {e:?}");
            },
            DexEvent::RaydiumAmmV4WithdrawPnl(e) => {
                println!("RaydiumAmmV4WithdrawPnlEvent: {e:?}");
            },

            // -------------------------- orca whirlpool -----------------------
            DexEvent::OrcaWhirlpoolSwap(e) => {
                println!("OrcaWhirlpoolSwapEvent: {e:?}");
            },
            DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => {
                println!("OrcaWhirlpoolLiquidityIncreasedEvent: {e:?}");
            },
            DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => {
                println!("OrcaWhirlpoolLiquidityDecreasedEvent: {e:?}");
            },
            DexEvent::OrcaWhirlpoolPoolInitialized(e) => {
                println!("OrcaWhirlpoolPoolInitializedEvent: {e:?}");
            },

            // -------------------------- meteora pools -----------------------
            DexEvent::MeteoraPoolsSwap(e) => {
                println!("MeteoraPoolsSwapEvent: {e:?}");
            },
            DexEvent::MeteoraPoolsAddLiquidity(e) => {
                println!("MeteoraPoolsAddLiquidityEvent: {e:?}");
            },
            DexEvent::MeteoraPoolsRemoveLiquidity(e) => {
                println!("MeteoraPoolsRemoveLiquidityEvent: {e:?}");
            },
            DexEvent::MeteoraPoolsBootstrapLiquidity(e) => {
                println!("MeteoraPoolsBootstrapLiquidityEvent: {e:?}");
            },
            DexEvent::MeteoraPoolsPoolCreated(e) => {
                println!("MeteoraPoolsPoolCreatedEvent: {e:?}");
            },
            DexEvent::MeteoraPoolsSetPoolFees(e) => {
                println!("MeteoraPoolsSetPoolFeesEvent: {e:?}");
            },

            // -------------------------- meteora damm v2 -----------------------
            DexEvent::MeteoraDammV2Swap(e) => {
                println!("MeteoraDammV2SwapEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2AddLiquidity(e) => {
                println!("MeteoraDammV2AddLiquidityEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
                println!("MeteoraDammV2RemoveLiquidityEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2InitializePool(e) => {
                println!("MeteoraDammV2InitializePoolEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2CreatePosition(e) => {
                println!("MeteoraDammV2CreatePositionEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2ClosePosition(e) => {
                println!("MeteoraDammV2ClosePositionEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2ClaimPositionFee(e) => {
                println!("MeteoraDammV2ClaimPositionFeeEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2InitializeReward(e) => {
                println!("MeteoraDammV2InitializeRewardEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2FundReward(e) => {
                println!("MeteoraDammV2FundRewardEvent: {e:?}");
            },
            DexEvent::MeteoraDammV2ClaimReward(e) => {
                println!("MeteoraDammV2ClaimRewardEvent: {e:?}");
            },

            // -------------------------- account events -----------------------
            DexEvent::TokenAccount(e) => {
                println!("TokenAccountEvent: {e:?}");
            },
            DexEvent::NonceAccount(e) => {
                println!("NonceAccountEvent: {e:?}");
            },
            DexEvent::TokenInfo(e) => {
                println!("TokenInfoEvent: {e:?}");
            },

            // -------------------------- error events -----------------------
            DexEvent::Error(e) => {
                println!("Error event: {e:?}");
            },
        }
}