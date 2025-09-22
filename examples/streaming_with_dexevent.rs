use solana_streamer_sdk::{
    DexEvent,
    parse_transaction_events,
    EventMetadata,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Example showing how to use DexEvent with regular match patterns
/// This demonstrates the pattern you requested without match_event! macro
fn main() {
    println!("DexEvent Pattern Matching Example");

    // Simulate processing some transactions (replace with actual grpc streaming)
    simulate_transaction_processing();
}

fn simulate_transaction_processing() {
    // In a real implementation, you would:
    // 1. Set up gRPC streaming
    // 2. Get transaction data from Yellowstone
    // 3. Parse each transaction to get DexEvent
    // 4. Call handle_dex_event for each event

    println!("Simulating transaction processing...");

    // Example: simulate parsing a transaction
    let sample_transaction_data = vec![]; // This would be real transaction bytes

    // Parse transaction to get events
    match parse_transaction_events(&sample_transaction_data) {
        Ok(events) => {
            for event in events {
                handle_dex_event(event);
            }
        }
        Err(e) => {
            println!("Error parsing transaction: {:?}", e);
        }
    }
}

/// This is the main event handler function that replaces the callback pattern
/// Use this pattern instead of match_event! macro
fn handle_dex_event(event: DexEvent) {
    println!("🎉 Event received!");

    match event {
        // -------------------------- block meta -----------------------
        DexEvent::BlockMeta(e) => {
            println!("BlockMetaEvent: handle_us={:?}", e.metadata.handle_us);
        },

        // -------------------------- bonk -----------------------
        DexEvent::BonkPoolCreate(e) => {
            // When using grpc, you can get block_time from each event
            println!("block_time: {:?}, block_time_ms: {:?}", e.metadata.block_time, e.metadata.block_time_ms);
            println!("BonkPoolCreateEvent: symbol={:?}", e.base_mint_param.symbol);
        },
        DexEvent::BonkTrade(e) => {
            println!("BonkTradeEvent: pool_state={}, user={}, amount_in={}, amount_out={}, is_buy={}",
                e.pool_state, e.user, e.amount_in, e.amount_out, e.is_buy);
        },
        DexEvent::BonkMigrateAmm(e) => {
            println!("BonkMigrateAmmEvent: old_pool={}, new_pool={}, user={}, liquidity_amount={}",
                e.old_pool, e.new_pool, e.user, e.liquidity_amount);
        },

        // -------------------------- pumpfun -----------------------
        DexEvent::PumpFunTrade(e) => {
            println!("PumpFunTradeEvent: mint={}, user={}, sol_amount={}, token_amount={}, is_buy={}",
                e.mint, e.user, e.sol_amount, e.token_amount, e.is_buy);
        },
        DexEvent::PumpFunMigrate(e) => {
            println!("PumpFunMigrateEvent: mint={}, bonding_curve={}, migration_target={}",
                e.mint, e.bonding_curve, e.migration_target);
        },
        DexEvent::PumpFunCreate(e) => {
            println!("PumpFunCreateTokenEvent: mint={}, bonding_curve={}, token_metadata={:?}",
                e.mint, e.bonding_curve, e.token_metadata);
        },
        DexEvent::PumpFunComplete(e) => {
            println!("PumpFunCompleteTokenEvent: mint={}, bonding_curve={}, migration_target={}",
                e.mint, e.bonding_curve, e.migration_target);
        },

        // -------------------------- pumpswap -----------------------
        DexEvent::PumpSwapBuy(e) => {
            println!("PumpSwapBuy: pool_id={}, user={}, sol_amount={}, token_amount={}",
                e.pool_id, e.user, e.sol_amount, e.token_amount);
        },
        DexEvent::PumpSwapSell(e) => {
            println!("PumpSwapSell: pool_id={}, user={}, token_amount={}, sol_amount={}",
                e.pool_id, e.user, e.token_amount, e.sol_amount);
        },
        DexEvent::PumpSwapCreatePool(e) => {
            println!("PumpSwapCreatePool: pool_id={}, token_mint={}, creator={}",
                e.pool_id, e.token_mint, e.creator);
        },
        DexEvent::PumpSwapPoolCreated(e) => {
            println!("PumpSwapPoolCreated: pool_id={}, token_mint={}, initial_liquidity={}",
                e.pool_id, e.token_mint, e.initial_liquidity);
        },
        DexEvent::PumpSwapTrade(e) => {
            println!("PumpSwapTrade: pool_id={}, user={}, amount_in={}, amount_out={}, is_buy={}",
                e.pool_id, e.user, e.amount_in, e.amount_out, e.is_buy);
        },
        DexEvent::PumpSwapLiquidityAdded(e) => {
            println!("PumpSwapLiquidityAdded: pool_id={}, provider={}, token_amount={}, sol_amount={}",
                e.pool_id, e.provider, e.token_amount, e.sol_amount);
        },
        DexEvent::PumpSwapLiquidityRemoved(e) => {
            println!("PumpSwapLiquidityRemoved: pool_id={}, provider={}, token_amount={}, sol_amount={}",
                e.pool_id, e.provider, e.token_amount, e.sol_amount);
        },
        DexEvent::PumpSwapPoolUpdated(e) => {
            println!("PumpSwapPoolUpdated: pool_id={}, new_fee={}, new_authority={:?}",
                e.pool_id, e.new_fee, e.new_authority);
        },
        DexEvent::PumpSwapFeesClaimed(e) => {
            println!("PumpSwapFeesClaimed: pool_id={}, claimer={}, token_fees={}, sol_fees={}",
                e.pool_id, e.claimer, e.token_fees, e.sol_fees);
        },

        // -------------------------- raydium_cpmm -----------------------
        DexEvent::RaydiumCpmmSwap(e) => {
            println!("RaydiumCpmmSwapEvent: pool={}, user={}, amount_in={}, amount_out={}",
                e.pool, e.user, e.amount_in, e.amount_out);
        },
        DexEvent::RaydiumCpmmDeposit(e) => {
            println!("RaydiumCpmmDepositEvent: pool={}, user={}, token_0_amount={}, token_1_amount={}, lp_amount={}",
                e.pool, e.user, e.token_0_amount, e.token_1_amount, e.lp_amount);
        },
        DexEvent::RaydiumCpmmInitialize(e) => {
            println!("RaydiumCpmmInitializeEvent: pool={}, token_0_mint={}, token_1_mint={}, initial_price={}",
                e.pool, e.token_0_mint, e.token_1_mint, e.initial_price);
        },
        DexEvent::RaydiumCpmmWithdraw(e) => {
            println!("RaydiumCpmmWithdrawEvent: pool={}, user={}, lp_amount={}, token_0_amount={}, token_1_amount={}",
                e.pool, e.user, e.lp_amount, e.token_0_amount, e.token_1_amount);
        },

        // -------------------------- raydium_clmm -----------------------
        DexEvent::RaydiumClmmSwap(e) => {
            println!("RaydiumClmmSwapEvent: pool={}, user={}, amount={}, other_amount_threshold={}, sqrt_price_limit={}, is_base_input={}",
                e.pool, e.user, e.amount, e.other_amount_threshold, e.sqrt_price_limit, e.is_base_input);
        },
        DexEvent::RaydiumClmmClosePosition(e) => {
            println!("RaydiumClmmClosePositionEvent: pool={}, position={}, owner={}",
                e.pool, e.position, e.owner);
        },
        DexEvent::RaydiumClmmDecreaseLiquidity(e) => {
            println!("RaydiumClmmDecreaseLiquidityEvent: pool={}, position={}, liquidity_amount={}, amount_0_min={}, amount_1_min={}",
                e.pool, e.position, e.liquidity_amount, e.amount_0_min, e.amount_1_min);
        },
        DexEvent::RaydiumClmmCreatePool(e) => {
            println!("RaydiumClmmCreatePoolEvent: pool={}, token_0_mint={}, token_1_mint={}, tick_spacing={}, sqrt_price_x64={}",
                e.pool, e.token_0_mint, e.token_1_mint, e.tick_spacing, e.sqrt_price_x64);
        },
        DexEvent::RaydiumClmmIncreaseLiquidity(e) => {
            println!("RaydiumClmmIncreaseLiquidityEvent: pool={}, position={}, liquidity_amount={}, amount_0_max={}, amount_1_max={}",
                e.pool, e.position, e.liquidity_amount, e.amount_0_max, e.amount_1_max);
        },
        DexEvent::RaydiumClmmOpenPosition(e) => {
            println!("RaydiumClmmOpenPositionEvent: pool={}, position={}, owner={}, tick_lower_index={}, tick_upper_index={}",
                e.pool, e.position, e.owner, e.tick_lower_index, e.tick_upper_index);
        },
        DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => {
            println!("RaydiumClmmOpenPositionWithTokenExtNftEvent: pool={}, position={}, owner={}, tick_lower_index={}, tick_upper_index={}",
                e.pool, e.position, e.owner, e.tick_lower_index, e.tick_upper_index);
        },
        DexEvent::RaydiumClmmCollectFee(e) => {
            println!("RaydiumClmmCollectFeeEvent: pool={}, position={}, owner={}, amount_0={}, amount_1={}",
                e.pool, e.position, e.owner, e.amount_0, e.amount_1);
        },

        // -------------------------- raydium_amm_v4 -----------------------
        DexEvent::RaydiumAmmV4Swap(e) => {
            println!("RaydiumAmmV4SwapEvent: amm={}, user={}, amount_in={}, amount_out={}, input_mint={}, output_mint={}",
                e.amm, e.user, e.amount_in, e.amount_out, e.input_mint, e.output_mint);
        },
        DexEvent::RaydiumAmmV4Deposit(e) => {
            println!("RaydiumAmmV4DepositEvent: amm={}, user={}, token_0_amount={}, token_1_amount={}, lp_amount={}",
                e.amm, e.user, e.token_0_amount, e.token_1_amount, e.lp_amount);
        },
        DexEvent::RaydiumAmmV4Initialize2(e) => {
            println!("RaydiumAmmV4Initialize2Event: amm={}, open_time={}, pc_amount={}, coin_amount={}",
                e.amm, e.open_time, e.pc_amount, e.coin_amount);
        },
        DexEvent::RaydiumAmmV4Withdraw(e) => {
            println!("RaydiumAmmV4WithdrawEvent: amm={}, user={}, lp_amount={}, token_0_amount={}, token_1_amount={}",
                e.amm, e.user, e.lp_amount, e.token_0_amount, e.token_1_amount);
        },
        DexEvent::RaydiumAmmV4WithdrawPnl(e) => {
            println!("RaydiumAmmV4WithdrawPnlEvent: amm={}, user={}, pnl_owner={}, pnl_token_amount={}, pnl_pc_amount={}",
                e.amm, e.user, e.pnl_owner, e.pnl_token_amount, e.pnl_pc_amount);
        },

        // -------------------------- orca whirlpool -----------------------
        DexEvent::OrcaWhirlpoolSwap(e) => {
            println!("OrcaWhirlpoolSwapEvent: whirlpool={}, a_to_b={}, pre_sqrt_price={}, post_sqrt_price={}",
                e.whirlpool, e.a_to_b, e.pre_sqrt_price, e.post_sqrt_price);
        },
        DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => {
            println!("OrcaWhirlpoolLiquidityIncreasedEvent: whirlpool={}, position={}, tick_lower_index={}, tick_upper_index={}",
                e.whirlpool, e.position, e.tick_lower_index, e.tick_upper_index);
        },
        DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => {
            println!("OrcaWhirlpoolLiquidityDecreasedEvent: whirlpool={}, position={}, tick_lower_index={}, tick_upper_index={}",
                e.whirlpool, e.position, e.tick_lower_index, e.tick_upper_index);
        },
        DexEvent::OrcaWhirlpoolPoolInitialized(e) => {
            println!("OrcaWhirlpoolPoolInitializedEvent: whirlpool={}, sqrt_price={}, tick_index={}",
                e.whirlpool, e.sqrt_price, e.tick_index);
        },

        // -------------------------- meteora pools -----------------------
        DexEvent::MeteoraPoolsSwap(e) => {
            println!("MeteoraPoolsSwapEvent: in_amount={}, out_amount={}, trade_fee={}, protocol_fee={}, host_fee={}",
                e.in_amount, e.out_amount, e.trade_fee, e.protocol_fee, e.host_fee);
        },
        DexEvent::MeteoraPoolsAddLiquidity(e) => {
            println!("MeteoraPoolsAddLiquidityEvent: lp_mint_amount={}, token_a_amount={}, token_b_amount={}",
                e.lp_mint_amount, e.token_a_amount, e.token_b_amount);
        },
        DexEvent::MeteoraPoolsRemoveLiquidity(e) => {
            println!("MeteoraPoolsRemoveLiquidityEvent: lp_unmint_amount={}, token_a_out_amount={}, token_b_out_amount={}",
                e.lp_unmint_amount, e.token_a_out_amount, e.token_b_out_amount);
        },
        DexEvent::MeteoraPoolsBootstrapLiquidity(e) => {
            println!("MeteoraPoolsBootstrapLiquidityEvent: pool={}, lp_mint_amount={}, token_a_amount={}, token_b_amount={}",
                e.pool, e.lp_mint_amount, e.token_a_amount, e.token_b_amount);
        },
        DexEvent::MeteoraPoolsPoolCreated(e) => {
            println!("MeteoraPoolsPoolCreatedEvent: pool={}, lp_mint={}, token_a_mint={}, token_b_mint={}, pool_type={}",
                e.pool, e.lp_mint, e.token_a_mint, e.token_b_mint, e.pool_type);
        },
        DexEvent::MeteoraPoolsSetPoolFees(e) => {
            println!("MeteoraPoolsSetPoolFeesEvent: pool={}, trade_fee_numerator={}, trade_fee_denominator={}, protocol_trade_fee_numerator={}, protocol_trade_fee_denominator={}",
                e.pool, e.trade_fee_numerator, e.trade_fee_denominator, e.protocol_trade_fee_numerator, e.protocol_trade_fee_denominator);
        },

        // -------------------------- meteora damm v2 -----------------------
        DexEvent::MeteoraDammV2Swap(e) => {
            println!("MeteoraDammV2SwapEvent: lb_pair={}, from={}, start_bin_id={}, end_bin_id={}",
                e.lb_pair, e.from, e.start_bin_id, e.end_bin_id);
        },
        DexEvent::MeteoraDammV2AddLiquidity(e) => {
            println!("MeteoraDammV2AddLiquidityEvent: lb_pair={}, from={}, position={}, amounts={:?}",
                e.lb_pair, e.from, e.position, e.amounts);
        },
        DexEvent::MeteoraDammV2RemoveLiquidity(e) => {
            println!("MeteoraDammV2RemoveLiquidityEvent: lb_pair={}, from={}, position={}, amounts={:?}",
                e.lb_pair, e.from, e.position, e.amounts);
        },
        DexEvent::MeteoraDammV2InitializePool(e) => {
            println!("MeteoraDammV2InitializePoolEvent: lb_pair={}, bin_step={}, token_x={}, token_y={}",
                e.lb_pair, e.bin_step, e.token_x, e.token_y);
        },
        DexEvent::MeteoraDammV2CreatePosition(e) => {
            println!("MeteoraDammV2CreatePositionEvent: lb_pair={}, position={}, owner={}",
                e.lb_pair, e.position, e.owner);
        },
        DexEvent::MeteoraDammV2ClosePosition(e) => {
            println!("MeteoraDammV2ClosePositionEvent: position={}, owner={}",
                e.position, e.owner);
        },
        DexEvent::MeteoraDammV2ClaimPositionFee(e) => {
            println!("MeteoraDammV2ClaimPositionFeeEvent: lb_pair={}, position={}, owner={}, fee_x={}, fee_y={}",
                e.lb_pair, e.position, e.owner, e.fee_x, e.fee_y);
        },
        DexEvent::MeteoraDammV2InitializeReward(e) => {
            println!("MeteoraDammV2InitializeRewardEvent: lb_pair={}, reward_mint={}, funder={}, reward_index={}, reward_duration={}",
                e.lb_pair, e.reward_mint, e.funder, e.reward_index, e.reward_duration);
        },
        DexEvent::MeteoraDammV2FundReward(e) => {
            println!("MeteoraDammV2FundRewardEvent: lb_pair={}, funder={}, reward_index={}, amount={}",
                e.lb_pair, e.funder, e.reward_index, e.amount);
        },
        DexEvent::MeteoraDammV2ClaimReward(e) => {
            println!("MeteoraDammV2ClaimRewardEvent: lb_pair={}, position={}, owner={}, reward_index={}, total_reward={}",
                e.lb_pair, e.position, e.owner, e.reward_index, e.total_reward);
        },

        // -------------------------- account events -----------------------
        DexEvent::TokenAccount(e) => {
            println!("TokenAccountEvent: mint={}, owner={}, amount={}",
                e.mint, e.owner, e.amount);
        },
        DexEvent::NonceAccount(e) => {
            println!("NonceAccountEvent: pubkey={}, authority={}, nonce={}",
                e.pubkey, e.authority, e.nonce);
        },
        DexEvent::TokenInfo(e) => {
            println!("TokenInfoEvent: mint={}, name={:?}, symbol={:?}, decimals={}",
                e.mint, e.name, e.symbol, e.decimals);
        },

        // -------------------------- error events -----------------------
        DexEvent::Error(e) => {
            println!("Error event: {}", e);
        },
    }
}

// Example: How you might set up streaming to get DexEvents
// This is a conceptual example - you would implement this based on your streaming infrastructure
#[allow(dead_code)]
async fn example_streaming_setup() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up gRPC client (not shown in this simplified example)
    // let grpc_client = YellowstoneGrpc::new(...)?;

    // 2. Set up transaction and account filters
    let program_ids = vec![
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", // PumpFun
        "PSwapMdSai8tjrEXcxFeQth87xC4rRsa4VA5mhGhXkP", // PumpSwap
        "BSwp6bEBihVLdqJRKS58NaebUBSDNjN7MdpFwNaR6gn3", // Bonk
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", // Raydium CPMM
        "CAMMCzo5YL8w4VFF8KVHrK22GGUQtcaMpgYqJPXBDvfE", // Raydium CLMM
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM V4
    ];

    // 3. For each transaction received:
    // let transaction_data = ...; // received from gRPC stream
    //
    // match parse_transaction_events(&transaction_data) {
    //     Ok(events) => {
    //         for event in events {
    //             handle_dex_event(event);
    //         }
    //     }
    //     Err(e) => {
    //         println!("Error parsing transaction: {:?}", e);
    //     }
    // }

    println!("Program IDs to monitor: {:?}", program_ids);
    Ok(())
}