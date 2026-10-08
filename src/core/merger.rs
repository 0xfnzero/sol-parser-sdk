//! 轻量级事件合并机制 - 零拷贝高性能实现
//!
//! 将 inner instruction 事件数据合并到主 instruction 事件中
//! 设计原则:
//! - 只合并必要的字段
//! - 保持零拷贝特性
//! - 内联优化，最小化开销
//!
//! **gRPC log + instruction 双路径**：见 [`merge_grpc_instruction_into_log`] —— **以程序日志为准**，
//! 指令解析仅补充账户等日志侧缺失字段。

use solana_sdk::pubkey::Pubkey;

use crate::core::events::*;

/// 合并 instruction 事件和 inner instruction 事件
///
/// # 设计
/// - Inner instruction 包含完整的交易数据（来自程序日志）
/// - Instruction 包含账户上下文（来自指令本身）
/// - 合并后的事件包含两者的完整信息
///
/// # 性能
/// - 内联优化，编译器会将其优化为直接赋值
/// - 零堆分配
/// - 预期开销 < 10ns
#[inline(always)]
pub fn merge_events(base: &mut DexEvent, inner: DexEvent) {
    let mut unmerged = None;
    let _ = try_merge_events(base, inner, &mut unmerged);
}

/// Try to merge `inner` into `base` without allocating or returning the large
/// event enum by value. On mismatch, the untouched event is written to
/// `unmerged` for the caller to recover.
#[inline(always)]
pub fn try_merge_events(
    base: &mut DexEvent,
    inner: DexEvent,
    unmerged: &mut Option<DexEvent>,
) -> bool {
    if pump_trade_identity_conflicts(base, &inner) {
        *unmerged = Some(inner);
        return false;
    }
    use DexEvent::*;

    match (base, inner) {
        // ========== PumpFun 系列 ==========
        (PumpFunTrade(b), PumpFunTrade(i))
        | (PumpFunTrade(b), PumpFunBuy(i))
        | (PumpFunTrade(b), PumpFunSell(i))
        | (PumpFunTrade(b), PumpFunBuyExactSolIn(i))
        | (PumpFunBuy(b), PumpFunTrade(i))
        | (PumpFunBuy(b), PumpFunBuy(i))
        | (PumpFunSell(b), PumpFunTrade(i))
        | (PumpFunSell(b), PumpFunSell(i))
        | (PumpFunBuyExactSolIn(b), PumpFunTrade(i))
        | (PumpFunBuyExactSolIn(b), PumpFunBuyExactSolIn(i)) => merge_pumpfun_trade(b, i),

        (PumpFunCreate(b), PumpFunCreate(i)) => merge_pumpfun_create(b, i),
        (PumpFunCreateV2(b), PumpFunCreateV2(i)) => merge_pumpfun_create_v2(b, i),
        (PumpFunMigrate(b), PumpFunMigrate(i)) => merge_pumpfun_migrate(b, i),
        (PumpFunMigrateBondingCurveCreator(b), PumpFunMigrateBondingCurveCreator(i)) => {
            merge_generic(b, i)
        }

        // ========== PumpFees 系列 ==========
        (PumpFeesCreateFeeSharingConfig(b), PumpFeesCreateFeeSharingConfig(i)) => {
            merge_generic(b, i)
        }
        (PumpFeesInitializeFeeConfig(b), PumpFeesInitializeFeeConfig(i)) => merge_generic(b, i),
        (PumpFeesResetFeeSharingConfig(b), PumpFeesResetFeeSharingConfig(i)) => merge_generic(b, i),
        (PumpFeesRevokeFeeSharingAuthority(b), PumpFeesRevokeFeeSharingAuthority(i)) => {
            merge_generic(b, i)
        }
        (PumpFeesTransferFeeSharingAuthority(b), PumpFeesTransferFeeSharingAuthority(i)) => {
            merge_generic(b, i)
        }
        (PumpFeesUpdateAdmin(b), PumpFeesUpdateAdmin(i)) => merge_generic(b, i),
        (PumpFeesUpdateFeeConfig(b), PumpFeesUpdateFeeConfig(i)) => merge_generic(b, i),
        (PumpFeesUpdateFeeShares(b), PumpFeesUpdateFeeShares(i)) => merge_generic(b, i),
        (PumpFeesUpsertFeeTiers(b), PumpFeesUpsertFeeTiers(i)) => merge_generic(b, i),

        // ========== PumpSwap 系列 ==========
        (PumpSwapTrade(b), PumpSwapTrade(i)) => merge_generic(b, i),
        (PumpSwapBuy(b), PumpSwapBuy(i)) => merge_pumpswap_buy(b, i),
        (PumpSwapSell(b), PumpSwapSell(i)) => merge_pumpswap_sell(b, i),
        (PumpSwapCreatePool(b), PumpSwapCreatePool(i)) => merge_generic(b, i),
        (PumpSwapLiquidityAdded(b), PumpSwapLiquidityAdded(i)) => merge_generic(b, i),
        (PumpSwapLiquidityRemoved(b), PumpSwapLiquidityRemoved(i)) => merge_generic(b, i),

        // ========== Raydium CLMM 系列 ==========
        (RaydiumClmmSwap(b), RaydiumClmmSwap(i)) => merge_clmm_swap(b, i),
        (RaydiumClmmIncreaseLiquidity(b), RaydiumClmmIncreaseLiquidity(i)) => merge_generic(b, i),
        (RaydiumClmmDecreaseLiquidity(b), RaydiumClmmDecreaseLiquidity(i)) => merge_generic(b, i),
        (RaydiumClmmLiquidityChange(b), RaydiumClmmLiquidityChange(i)) => merge_generic(b, i),
        (RaydiumClmmConfigChange(b), RaydiumClmmConfigChange(i)) => merge_generic(b, i),
        (RaydiumClmmCreatePersonalPosition(b), RaydiumClmmCreatePersonalPosition(i)) => {
            merge_generic(b, i)
        }
        (RaydiumClmmLiquidityCalculate(b), RaydiumClmmLiquidityCalculate(i)) => merge_generic(b, i),
        (RaydiumClmmOpenLimitOrder(b), RaydiumClmmOpenLimitOrder(i)) => merge_generic(b, i),
        (RaydiumClmmIncreaseLimitOrder(b), RaydiumClmmIncreaseLimitOrder(i)) => merge_generic(b, i),
        (RaydiumClmmDecreaseLimitOrder(b), RaydiumClmmDecreaseLimitOrder(i)) => merge_generic(b, i),
        (RaydiumClmmSettleLimitOrder(b), RaydiumClmmSettleLimitOrder(i)) => merge_generic(b, i),
        (RaydiumClmmUpdateRewardInfos(b), RaydiumClmmUpdateRewardInfos(i)) => merge_generic(b, i),
        (RaydiumClmmCreatePool(b), RaydiumClmmCreatePool(i)) => merge_generic(b, i),
        (RaydiumClmmOpenPosition(b), RaydiumClmmOpenPosition(i)) => merge_generic(b, i),
        (RaydiumClmmClosePosition(b), RaydiumClmmClosePosition(i)) => merge_generic(b, i),
        (RaydiumClmmOpenPositionWithTokenExtNft(b), RaydiumClmmOpenPositionWithTokenExtNft(i)) => {
            merge_generic(b, i)
        }
        (RaydiumClmmCollectFee(b), RaydiumClmmCollectFee(i)) => merge_generic(b, i),

        // ========== Raydium CPMM 系列 ==========
        (RaydiumCpmmSwap(b), RaydiumCpmmSwap(i)) => merge_cpmm_swap(b, i),
        (RaydiumCpmmDeposit(b), RaydiumCpmmDeposit(i)) => merge_generic(b, i),
        (RaydiumCpmmWithdraw(b), RaydiumCpmmWithdraw(i)) => merge_generic(b, i),
        (RaydiumCpmmInitialize(b), RaydiumCpmmInitialize(i)) => merge_generic(b, i),

        // ========== Raydium AMM V4 系列 ==========
        (RaydiumAmmV4Swap(b), RaydiumAmmV4Swap(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_raydium_amm_v4_swap_log_preferred(b, instruction);
        }
        (RaydiumAmmV4Deposit(b), RaydiumAmmV4Deposit(i)) => merge_generic(b, i),
        (RaydiumAmmV4Withdraw(b), RaydiumAmmV4Withdraw(i)) => merge_generic(b, i),
        (RaydiumAmmV4Initialize2(b), RaydiumAmmV4Initialize2(i)) => merge_generic(b, i),
        (RaydiumAmmV4WithdrawPnl(b), RaydiumAmmV4WithdrawPnl(i)) => merge_generic(b, i),

        // ========== Orca Whirlpool 系列 ==========
        (OrcaWhirlpoolSwap(b), OrcaWhirlpoolSwap(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_orca_swap_context(b, instruction);
        }
        (OrcaWhirlpoolLiquidityIncreased(b), OrcaWhirlpoolLiquidityIncreased(i)) => {
            merge_generic(b, i)
        }
        (OrcaWhirlpoolLiquidityDecreased(b), OrcaWhirlpoolLiquidityDecreased(i)) => {
            merge_generic(b, i)
        }
        (OrcaWhirlpoolPoolInitialized(b), OrcaWhirlpoolPoolInitialized(i)) => merge_generic(b, i),

        // ========== Meteora Pools (AMM) 系列 ==========
        (MeteoraPoolsSwap(b), MeteoraPoolsSwap(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_swap_context(b, instruction);
        }
        (MeteoraPoolsAddLiquidity(b), MeteoraPoolsAddLiquidity(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_add_context(b, instruction);
        }
        (MeteoraPoolsRemoveLiquidity(b), MeteoraPoolsRemoveLiquidity(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_remove_context(b, instruction);
        }
        (MeteoraPoolsBootstrapLiquidity(b), MeteoraPoolsBootstrapLiquidity(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_bootstrap_context(b, instruction);
        }
        (MeteoraPoolsPoolCreated(b), MeteoraPoolsPoolCreated(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_poolcreated_context(b, instruction);
        }
        (MeteoraPoolsSetPoolFees(b), MeteoraPoolsSetPoolFees(i)) => {
            let instruction = std::mem::replace(b, i);
            merge_meteora_pools_setpoolfees_context(b, instruction);
        }

        // ========== Meteora DAMM V2 系列 ==========
        (MeteoraDammV2Swap(b), MeteoraDammV2Swap(i)) => merge_generic(b, i),
        (MeteoraDammV2AddLiquidity(b), MeteoraDammV2AddLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2RemoveLiquidity(b), MeteoraDammV2RemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2InitializePool(b), MeteoraDammV2InitializePool(i)) => merge_generic(b, i),
        (MeteoraDammV2ClaimPositionFee(b), MeteoraDammV2ClaimPositionFee(i)) => merge_generic(b, i),
        (MeteoraDammV2ClaimReward(b), MeteoraDammV2ClaimReward(i)) => merge_generic(b, i),
        (MeteoraDlmmClaimReward(b), MeteoraDlmmClaimReward(i)) => merge_generic(b, i),
        (MeteoraDammV2CreatePosition(b), MeteoraDammV2CreatePosition(i)) => merge_generic(b, i),
        (MeteoraDammV2ClosePosition(b), MeteoraDammV2ClosePosition(i)) => merge_generic(b, i),
        (MeteoraDammV2UpdateDelegatePermission(b), MeteoraDammV2UpdateDelegatePermission(i)) => {
            merge_generic(b, i)
        }
        (
            MeteoraDammV2WithdrawDeadLiquidityReward(b),
            MeteoraDammV2WithdrawDeadLiquidityReward(i),
        ) => merge_generic(b, i),
        (MeteoraDammV2WithdrawIneligibleReward(b), MeteoraDammV2WithdrawIneligibleReward(i)) => {
            merge_generic(b, i)
        }
        (MeteoraDammV2UpdateRewardFunder(b), MeteoraDammV2UpdateRewardFunder(i)) => {
            merge_generic(b, i)
        }
        (MeteoraDammV2UpdateRewardDuration(b), MeteoraDammV2UpdateRewardDuration(i)) => {
            merge_generic(b, i)
        }
        (MeteoraDammV2InitializeReward(b), MeteoraDammV2InitializeReward(i)) => merge_generic(b, i),
        (MeteoraDammV2FundReward(b), MeteoraDammV2FundReward(i)) => merge_generic(b, i),
        (MeteoraDammV2CreateConfig(b), MeteoraDammV2CreateConfig(i)) => merge_generic(b, i),
        (MeteoraDammV2CreateDynamicConfig(b), MeteoraDammV2CreateDynamicConfig(i)) => {
            merge_generic(b, i)
        }

        // ========== Meteora DLMM 系列 ==========
        (MeteoraDlmmSwap(b), MeteoraDlmmSwap(i)) => merge_dlmm_swap(b, i),
        (MeteoraDlmmAddLiquidity(b), MeteoraDlmmAddLiquidity(i)) => merge_generic(b, i),
        (MeteoraDlmmRemoveLiquidity(b), MeteoraDlmmRemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraDlmmInitializePool(b), MeteoraDlmmInitializePool(i)) => {
            merge_dlmm_initialize_pool(b, i)
        }
        (MeteoraDlmmInitializeBinArray(b), MeteoraDlmmInitializeBinArray(i)) => merge_generic(b, i),
        (MeteoraDlmmCreatePosition(b), MeteoraDlmmCreatePosition(i)) => {
            merge_dlmm_create_position(b, i)
        }
        (MeteoraDlmmClosePosition(b), MeteoraDlmmClosePosition(i)) => {
            merge_dlmm_close_position(b, i)
        }
        (MeteoraDlmmClaimFee(b), MeteoraDlmmClaimFee(i)) => merge_generic(b, i),

        // ========== RaydiumLaunchlab 系列 ==========
        (RaydiumLaunchlabTrade(b), RaydiumLaunchlabTrade(i)) => merge_generic(b, i),
        (RaydiumLaunchlabPoolCreate(b), RaydiumLaunchlabPoolCreate(i)) => merge_generic(b, i),
        (RaydiumLaunchlabMigrateAmm(b), RaydiumLaunchlabMigrateAmm(i)) => merge_generic(b, i),

        // 其他组合不需要合并（类型不匹配）
        (_, event) => {
            *unmerged = Some(event);
            return false;
        }
    }

    true
}

/// 通用合并函数 - 对于大多数事件，inner instruction 包含完整数据
///
/// 这个函数简单地用 inner 的数据覆盖 base，因为：
/// - Inner instruction 来自程序日志，包含完整的交易数据
/// - Instruction 主要提供账户上下文
/// - 对于大多数协议，inner instruction 的数据已经足够完整
#[inline(always)]
fn merge_cpmm_swap(base: &mut RaydiumCpmmSwapEvent, inner: RaydiumCpmmSwapEvent) {
    let instruction = std::mem::replace(base, inner);
    merge_raydium_cpmm_swap_log_preferred(base, instruction);
}

#[inline]
fn merge_raydium_cpmm_swap_log_preferred(log: &mut RaydiumCpmmSwapEvent, ix: RaydiumCpmmSwapEvent) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.amount_in = ix.amount_in;
        log.minimum_amount_out = ix.minimum_amount_out;
        log.max_amount_in = ix.max_amount_in;
        log.amount_out = ix.amount_out;
    }
    fill_pk(&mut log.payer, ix.payer);
    fill_pk(&mut log.authority, ix.authority);
    fill_pk(&mut log.input_token_account, ix.input_token_account);
    fill_pk(&mut log.output_token_account, ix.output_token_account);
    fill_pk(&mut log.amm_config, ix.amm_config);
    fill_pk(&mut log.input_vault, ix.input_vault);
    fill_pk(&mut log.output_vault, ix.output_vault);
    fill_pk(&mut log.input_token_program, ix.input_token_program);
    fill_pk(&mut log.output_token_program, ix.output_token_program);
    fill_pk(&mut log.input_token_mint, ix.input_token_mint);
    fill_pk(&mut log.output_token_mint, ix.output_token_mint);
    fill_pk(&mut log.observation_state, ix.observation_state);
}

#[inline(always)]
fn merge_clmm_swap(base: &mut RaydiumClmmSwapEvent, inner: RaydiumClmmSwapEvent) {
    let instruction = std::mem::replace(base, inner);
    merge_raydium_clmm_swap_log_preferred(base, instruction);
}

#[inline(always)]
fn merge_generic<T>(base: &mut T, inner: T) {
    *base = inner;
}

#[inline(always)]
fn merge_dlmm_swap(base: &mut MeteoraDlmmSwapEvent, inner: MeteoraDlmmSwapEvent) {
    let min_amount_out =
        if inner.min_amount_out != 0 { inner.min_amount_out } else { base.min_amount_out };
    let user_token_in = if inner.user_token_in != Pubkey::default() {
        inner.user_token_in
    } else {
        base.user_token_in
    };
    let user_token_out = if inner.user_token_out != Pubkey::default() {
        inner.user_token_out
    } else {
        base.user_token_out
    };
    *base = inner;
    base.min_amount_out = min_amount_out;
    base.user_token_in = user_token_in;
    base.user_token_out = user_token_out;
}

#[inline(always)]
fn merge_dlmm_initialize_pool(
    base: &mut MeteoraDlmmInitializePoolEvent,
    inner: MeteoraDlmmInitializePoolEvent,
) {
    let creator = base.creator;
    let active_bin_id = base.active_bin_id;
    *base = inner;
    base.creator = creator;
    base.active_bin_id = active_bin_id;
}

#[inline(always)]
fn merge_dlmm_create_position(
    base: &mut MeteoraDlmmCreatePositionEvent,
    inner: MeteoraDlmmCreatePositionEvent,
) {
    let lower_bin_id = base.lower_bin_id;
    let width = base.width;
    *base = inner;
    base.lower_bin_id = lower_bin_id;
    base.width = width;
}

#[inline(always)]
fn merge_dlmm_close_position(
    base: &mut MeteoraDlmmClosePositionEvent,
    inner: MeteoraDlmmClosePositionEvent,
) {
    let pool = base.pool;
    *base = inner;
    base.pool = pool;
}

// ============================================================================
// PumpFun 事件合并实现
// ============================================================================

#[inline(always)]
fn put_pk_if_set(to: &mut Pubkey, from: Pubkey) {
    if from != Pubkey::default() {
        *to = from;
    }
}

#[inline(always)]
fn put_pumpfun_quote_mint_if_set(to: &mut Pubkey, from: Pubkey) {
    let from = normalize_pumpfun_quote_mint(from);
    if from != Pubkey::default()
        && (*to == Pubkey::default()
            || is_pumpfun_solscan_sol_quote_mint(*to)
            || !is_pumpfun_solscan_sol_quote_mint(from))
    {
        *to = from;
    }
}

#[inline(always)]
fn put_u64_if_nonzero(to: &mut u64, from: u64) {
    if from != 0 {
        *to = from;
    }
}

#[inline(always)]
fn put_i64_if_nonzero(to: &mut i64, from: i64) {
    if from != 0 {
        *to = from;
    }
}

/// 合并 PumpFun Trade 事件
///
/// 合并策略:
/// - Inner instruction 提供: 交易数据（amount, reserves, fees 等）
/// - Instruction 提供: 账户上下文（bonding_curve, associated_bonding_curve 等）
/// - 合并后: 完整的交易事件
///
/// 同一 outer 下多段 inner 链式合并时：若某段 inner 未带成交量（`sol_amount`/`token_amount` 均为 0），
/// 则不再用其覆盖金额与储备，避免把前一段已合并好的数据清空。
#[inline(always)]
fn merge_pumpfun_trade(base: &mut PumpFunTradeEvent, inner: PumpFunTradeEvent) {
    let leg = inner.sol_amount != 0 || inner.token_amount != 0;

    put_pk_if_set(&mut base.mint, inner.mint);
    put_pk_if_set(&mut base.user, inner.user);
    put_pk_if_set(&mut base.fee_recipient, inner.fee_recipient);
    put_pk_if_set(&mut base.creator, inner.creator);

    if leg {
        base.sol_amount = inner.sol_amount;
        base.token_amount = inner.token_amount;
        base.is_buy = inner.is_buy;
        base.timestamp = inner.timestamp;
        base.virtual_sol_reserves = inner.virtual_sol_reserves;
        base.virtual_token_reserves = inner.virtual_token_reserves;
        base.real_sol_reserves = inner.real_sol_reserves;
        base.real_token_reserves = inner.real_token_reserves;
        base.fee_basis_points = inner.fee_basis_points;
        base.fee = inner.fee;
        base.creator_fee_basis_points = inner.creator_fee_basis_points;
        base.creator_fee = inner.creator_fee;
        base.track_volume |= inner.track_volume;
        base.total_unclaimed_tokens = inner.total_unclaimed_tokens;
        base.total_claimed_tokens = inner.total_claimed_tokens;
        base.current_sol_volume = inner.current_sol_volume;
        base.last_update_timestamp = inner.last_update_timestamp;
        if !inner.ix_name.is_empty() {
            base.ix_name = inner.ix_name;
        }
        base.mayhem_mode |= inner.mayhem_mode;
        put_u64_if_nonzero(&mut base.cashback_fee_basis_points, inner.cashback_fee_basis_points);
        put_u64_if_nonzero(&mut base.cashback, inner.cashback);
        put_u64_if_nonzero(&mut base.buyback_fee_basis_points, inner.buyback_fee_basis_points);
        put_u64_if_nonzero(&mut base.buyback_fee, inner.buyback_fee);
        if base.shareholders.is_empty() && !inner.shareholders.is_empty() {
            base.shareholders = inner.shareholders;
        }
        put_pumpfun_quote_mint_if_set(&mut base.quote_mint, inner.quote_mint);
        put_u64_if_nonzero(&mut base.quote_amount, inner.quote_amount);
        put_u64_if_nonzero(&mut base.virtual_quote_reserves, inner.virtual_quote_reserves);
        put_u64_if_nonzero(&mut base.real_quote_reserves, inner.real_quote_reserves);
        base.is_cashback_coin |= inner.is_cashback_coin;
        base.holder_rewards_bps = inner.holder_rewards_bps;
        base.holder_rewards = inner.holder_rewards;
        base.creator_fee_unclaimed = inner.creator_fee_unclaimed;
    } else {
        put_u64_if_nonzero(&mut base.fee, inner.fee);
        put_u64_if_nonzero(&mut base.creator_fee, inner.creator_fee);
        put_u64_if_nonzero(&mut base.fee_basis_points, inner.fee_basis_points);
        put_u64_if_nonzero(&mut base.creator_fee_basis_points, inner.creator_fee_basis_points);
        put_u64_if_nonzero(&mut base.virtual_sol_reserves, inner.virtual_sol_reserves);
        put_u64_if_nonzero(&mut base.virtual_token_reserves, inner.virtual_token_reserves);
        put_u64_if_nonzero(&mut base.real_sol_reserves, inner.real_sol_reserves);
        put_u64_if_nonzero(&mut base.real_token_reserves, inner.real_token_reserves);
        put_u64_if_nonzero(&mut base.total_unclaimed_tokens, inner.total_unclaimed_tokens);
        put_u64_if_nonzero(&mut base.total_claimed_tokens, inner.total_claimed_tokens);
        put_u64_if_nonzero(&mut base.current_sol_volume, inner.current_sol_volume);
        put_u64_if_nonzero(&mut base.cashback_fee_basis_points, inner.cashback_fee_basis_points);
        put_u64_if_nonzero(&mut base.cashback, inner.cashback);
        put_u64_if_nonzero(&mut base.buyback_fee_basis_points, inner.buyback_fee_basis_points);
        put_u64_if_nonzero(&mut base.buyback_fee, inner.buyback_fee);
        if base.shareholders.is_empty() && !inner.shareholders.is_empty() {
            base.shareholders = inner.shareholders;
        }
        put_pumpfun_quote_mint_if_set(&mut base.quote_mint, inner.quote_mint);
        put_u64_if_nonzero(&mut base.quote_amount, inner.quote_amount);
        put_u64_if_nonzero(&mut base.virtual_quote_reserves, inner.virtual_quote_reserves);
        put_u64_if_nonzero(&mut base.real_quote_reserves, inner.real_quote_reserves);
        put_u64_if_nonzero(&mut base.holder_rewards_bps, inner.holder_rewards_bps);
        put_u64_if_nonzero(&mut base.holder_rewards, inner.holder_rewards);
        put_i64_if_nonzero(&mut base.timestamp, inner.timestamp);
        put_i64_if_nonzero(&mut base.last_update_timestamp, inner.last_update_timestamp);
        if !inner.ix_name.is_empty() {
            base.ix_name = inner.ix_name;
        }
        base.track_volume |= inner.track_volume;
        base.mayhem_mode |= inner.mayhem_mode;
        base.is_cashback_coin |= inner.is_cashback_coin;
    }
    put_u64_if_nonzero(&mut base.amount, inner.amount);
    put_u64_if_nonzero(&mut base.max_sol_cost, inner.max_sol_cost);
    put_u64_if_nonzero(&mut base.min_sol_output, inner.min_sol_output);
    put_u64_if_nonzero(&mut base.spendable_sol_in, inner.spendable_sol_in);
    put_u64_if_nonzero(&mut base.spendable_quote_in, inner.spendable_quote_in);
    put_u64_if_nonzero(&mut base.min_tokens_out, inner.min_tokens_out);
    put_pk_if_set(&mut base.global, inner.global);
    put_pk_if_set(&mut base.bonding_curve, inner.bonding_curve);
    put_pk_if_set(&mut base.bonding_curve_v2, inner.bonding_curve_v2);
    put_pk_if_set(&mut base.associated_bonding_curve, inner.associated_bonding_curve);
    put_pk_if_set(&mut base.associated_user, inner.associated_user);
    put_pk_if_set(&mut base.system_program, inner.system_program);
    put_pk_if_set(&mut base.token_program, inner.token_program);
    put_pk_if_set(&mut base.quote_token_program, inner.quote_token_program);
    put_pk_if_set(&mut base.associated_token_program, inner.associated_token_program);
    put_pk_if_set(&mut base.creator_vault, inner.creator_vault);
    put_pk_if_set(&mut base.associated_quote_fee_recipient, inner.associated_quote_fee_recipient);
    put_pk_if_set(&mut base.buyback_fee_recipient, inner.buyback_fee_recipient);
    put_pk_if_set(
        &mut base.associated_quote_buyback_fee_recipient,
        inner.associated_quote_buyback_fee_recipient,
    );
    put_pk_if_set(&mut base.associated_quote_bonding_curve, inner.associated_quote_bonding_curve);
    put_pk_if_set(&mut base.associated_quote_user, inner.associated_quote_user);
    put_pk_if_set(&mut base.associated_creator_vault, inner.associated_creator_vault);
    put_pk_if_set(&mut base.sharing_config, inner.sharing_config);
    put_pk_if_set(&mut base.event_authority, inner.event_authority);
    put_pk_if_set(&mut base.program, inner.program);
    put_pk_if_set(&mut base.global_volume_accumulator, inner.global_volume_accumulator);
    put_pk_if_set(&mut base.user_volume_accumulator, inner.user_volume_accumulator);
    put_pk_if_set(
        &mut base.associated_user_volume_accumulator,
        inner.associated_user_volume_accumulator,
    );
    put_pk_if_set(&mut base.fee_config, inner.fee_config);
    put_pk_if_set(&mut base.fee_program, inner.fee_program);
    if base.account.is_none() {
        base.account = inner.account;
    }

    base.is_created_buy |= inner.is_created_buy;
    // 保留 base 的账户上下文字段（bonding_curve, associated_bonding_curve 等）
}

/// 合并 PumpFun Create 事件
#[inline(always)]
fn merge_pumpfun_create(base: &mut PumpFunCreateTokenEvent, inner: PumpFunCreateTokenEvent) {
    // Inner instruction 包含完整的 create 数据
    base.name = inner.name;
    base.symbol = inner.symbol;
    base.uri = inner.uri;
    base.mint = inner.mint;
    base.bonding_curve = inner.bonding_curve;
    base.user = inner.user;
    base.creator = inner.creator;
    base.timestamp = inner.timestamp;
    base.virtual_token_reserves = inner.virtual_token_reserves;
    base.virtual_sol_reserves = inner.virtual_sol_reserves;
    base.real_token_reserves = inner.real_token_reserves;
    base.token_total_supply = inner.token_total_supply;
    base.token_program = inner.token_program;
    base.is_mayhem_mode = inner.is_mayhem_mode;
    base.is_cashback_enabled = inner.is_cashback_enabled;
    put_pumpfun_quote_mint_if_set(&mut base.quote_mint, inner.quote_mint);
    put_pk_if_set(&mut base.quote_vault, inner.quote_vault);
    put_pk_if_set(&mut base.quote_token_program, inner.quote_token_program);
    put_u64_if_nonzero(&mut base.virtual_quote_reserves, inner.virtual_quote_reserves);
    put_u64_if_nonzero(&mut base.creator_fee_bps, inner.creator_fee_bps);
    base.is_holder_reward |= inner.is_holder_reward;
}

/// 合并 PumpFun CreateV2 事件
#[inline(always)]
fn merge_pumpfun_create_v2(base: &mut PumpFunCreateV2TokenEvent, inner: PumpFunCreateV2TokenEvent) {
    fill_str_if_empty(&mut base.name, &inner.name);
    fill_str_if_empty(&mut base.symbol, &inner.symbol);
    fill_str_if_empty(&mut base.uri, &inner.uri);
    put_pk_if_set(&mut base.mint, inner.mint);
    put_pk_if_set(&mut base.bonding_curve, inner.bonding_curve);
    put_pk_if_set(&mut base.user, inner.user);
    put_pk_if_set(&mut base.creator, inner.creator);
    put_i64_if_nonzero(&mut base.timestamp, inner.timestamp);
    put_u64_if_nonzero(&mut base.virtual_token_reserves, inner.virtual_token_reserves);
    put_u64_if_nonzero(&mut base.virtual_sol_reserves, inner.virtual_sol_reserves);
    put_u64_if_nonzero(&mut base.real_token_reserves, inner.real_token_reserves);
    put_u64_if_nonzero(&mut base.token_total_supply, inner.token_total_supply);
    put_pk_if_set(&mut base.token_program, inner.token_program);
    base.is_mayhem_mode |= inner.is_mayhem_mode;
    base.is_cashback_enabled |= inner.is_cashback_enabled;
    put_pumpfun_quote_mint_if_set(&mut base.quote_mint, inner.quote_mint);
    put_pk_if_set(&mut base.quote_vault, inner.quote_vault);
    put_pk_if_set(&mut base.quote_token_program, inner.quote_token_program);
    put_u64_if_nonzero(&mut base.virtual_quote_reserves, inner.virtual_quote_reserves);
    put_pk_if_set(&mut base.mint_authority, inner.mint_authority);
    put_pk_if_set(&mut base.associated_bonding_curve, inner.associated_bonding_curve);
    put_pk_if_set(&mut base.global, inner.global);
    put_pk_if_set(&mut base.system_program, inner.system_program);
    put_pk_if_set(&mut base.associated_token_program, inner.associated_token_program);
    put_pk_if_set(&mut base.mayhem_program_id, inner.mayhem_program_id);
    put_pk_if_set(&mut base.global_params, inner.global_params);
    put_pk_if_set(&mut base.sol_vault, inner.sol_vault);
    put_pk_if_set(&mut base.mayhem_state, inner.mayhem_state);
    put_pk_if_set(&mut base.mayhem_token_vault, inner.mayhem_token_vault);
    put_pk_if_set(&mut base.event_authority, inner.event_authority);
    put_pk_if_set(&mut base.program, inner.program);
    put_pk_if_set(&mut base.observed_fee_recipient, inner.observed_fee_recipient);
    put_u64_if_nonzero(&mut base.creator_fee_bps, inner.creator_fee_bps);
    base.is_holder_reward |= inner.is_holder_reward;
}

/// 合并 PumpFun Migrate 事件
#[inline(always)]
fn merge_pumpfun_migrate(base: &mut PumpFunMigrateEvent, inner: PumpFunMigrateEvent) {
    // Inner instruction 包含完整的 migrate 数据
    base.user = inner.user;
    base.mint = inner.mint;
    base.mint_amount = inner.mint_amount;
    base.sol_amount = inner.sol_amount;
    base.pool_migration_fee = inner.pool_migration_fee;
    base.bonding_curve = inner.bonding_curve;
    base.timestamp = inner.timestamp;
    base.pool = inner.pool;
}

#[inline(always)]
fn merge_pumpswap_buy(base: &mut PumpSwapBuyEvent, inner: PumpSwapBuyEvent) {
    let ix = std::mem::take(base);
    *base = inner;
    merge_pumpswap_buy_log_preferred(base, ix);
}

#[inline(always)]
fn merge_pumpswap_sell(base: &mut PumpSwapSellEvent, inner: PumpSwapSellEvent) {
    let ix = std::mem::take(base);
    *base = inner;
    merge_pumpswap_sell_log_preferred(base, ix);
}

// ============================================================================
// 工具函数
// ============================================================================

/// 判断两个事件是否可以合并
///
/// 合并条件:
/// 1. 都是同一个协议的事件
/// 2. 事件类型兼容（例如 Trade 和 Buy 可以合并）
/// 3. 来自同一个交易（signature 相同）
#[inline(always)]
pub fn can_merge(base: &DexEvent, inner: &DexEvent) -> bool {
    if pump_trade_identity_conflicts(base, inner) {
        return false;
    }
    // 检查 signature 是否相同
    if base.metadata().signature != inner.metadata().signature {
        return false;
    }

    // 检查事件类型是否兼容
    match (base, inner) {
        // PumpFun Trade 系列事件可以互相合并
        (DexEvent::PumpFunTrade(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunBuy(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunSell(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunBuyExactSolIn(_))
        | (DexEvent::PumpFunBuy(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunBuy(_), DexEvent::PumpFunBuy(_))
        | (DexEvent::PumpFunSell(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunSell(_), DexEvent::PumpFunSell(_))
        | (DexEvent::PumpFunBuyExactSolIn(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunBuyExactSolIn(_), DexEvent::PumpFunBuyExactSolIn(_)) => true,

        // PumpFun Create / CreateV2 可以合并
        (DexEvent::PumpFunCreate(_), DexEvent::PumpFunCreate(_)) => true,
        (DexEvent::PumpFunCreateV2(_), DexEvent::PumpFunCreateV2(_)) => true,

        // PumpFun Migrate 可以合并
        (DexEvent::PumpFunMigrate(_), DexEvent::PumpFunMigrate(_)) => true,

        (DexEvent::PumpSwapBuy(_), DexEvent::PumpSwapBuy(_))
        | (DexEvent::PumpSwapSell(_), DexEvent::PumpSwapSell(_)) => true,

        // 其他组合不支持合并
        _ => false,
    }
}

// ============================================================================
// gRPC：日志优先 + 指令补充（Yellowstone 并行解析 log / ix）
// ============================================================================

#[inline(always)]
fn fill_pk(to: &mut Pubkey, from: Pubkey) {
    if *to == Pubkey::default() && from != Pubkey::default() {
        *to = from;
    }
}

#[inline(always)]
fn fill_pumpfun_quote_mint(to: &mut Pubkey, from: Pubkey) {
    let from = normalize_pumpfun_quote_mint(from);
    if (*to == Pubkey::default() || is_pumpfun_solscan_sol_quote_mint(*to))
        && from != Pubkey::default()
    {
        *to = from;
    }
}

#[inline(always)]
fn fill_str_if_empty(to: &mut String, from: &str) {
    if to.is_empty() && !from.is_empty() {
        to.push_str(from);
    }
}

/// PumpFun Trade：**保留 `log` 侧全部链上事件数值与标志**（与 `TradeEvent` 日志一致），
/// 仅用 `ix` 补齐默认的账户类字段；`is_created_buy` 若仅 ix 侧为 true 则置位（创建首买标记）。
#[inline]
fn merge_pumpfun_trade_log_preferred(log: &mut PumpFunTradeEvent, ix: PumpFunTradeEvent) {
    fill_pk(&mut log.global, ix.global);
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.bonding_curve_v2, ix.bonding_curve_v2);
    fill_pk(&mut log.associated_bonding_curve, ix.associated_bonding_curve);
    fill_pk(&mut log.associated_user, ix.associated_user);
    fill_pk(&mut log.system_program, ix.system_program);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    fill_pk(&mut log.associated_token_program, ix.associated_token_program);
    fill_pk(&mut log.creator_vault, ix.creator_vault);
    fill_pk(&mut log.fee_recipient, ix.fee_recipient);
    fill_pk(&mut log.creator, ix.creator);
    fill_pumpfun_quote_mint(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.associated_quote_fee_recipient, ix.associated_quote_fee_recipient);
    fill_pk(&mut log.buyback_fee_recipient, ix.buyback_fee_recipient);
    fill_pk(
        &mut log.associated_quote_buyback_fee_recipient,
        ix.associated_quote_buyback_fee_recipient,
    );
    fill_pk(&mut log.associated_quote_bonding_curve, ix.associated_quote_bonding_curve);
    fill_pk(&mut log.associated_quote_user, ix.associated_quote_user);
    fill_pk(&mut log.associated_creator_vault, ix.associated_creator_vault);
    fill_pk(&mut log.sharing_config, ix.sharing_config);
    fill_pk(&mut log.event_authority, ix.event_authority);
    fill_pk(&mut log.program, ix.program);
    fill_pk(&mut log.global_volume_accumulator, ix.global_volume_accumulator);
    fill_pk(&mut log.user_volume_accumulator, ix.user_volume_accumulator);
    fill_pk(&mut log.associated_user_volume_accumulator, ix.associated_user_volume_accumulator);
    fill_pk(&mut log.fee_config, ix.fee_config);
    fill_pk(&mut log.fee_program, ix.fee_program);
    if log.account.is_none() {
        log.account = ix.account;
    }
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    put_u64_if_nonzero(&mut log.amount, ix.amount);
    put_u64_if_nonzero(&mut log.max_sol_cost, ix.max_sol_cost);
    put_u64_if_nonzero(&mut log.min_sol_output, ix.min_sol_output);
    put_u64_if_nonzero(&mut log.spendable_sol_in, ix.spendable_sol_in);
    put_u64_if_nonzero(&mut log.spendable_quote_in, ix.spendable_quote_in);
    put_u64_if_nonzero(&mut log.min_tokens_out, ix.min_tokens_out);
    if log.quote_amount == 0 {
        put_u64_if_nonzero(&mut log.quote_amount, ix.quote_amount);
    }
    if log.virtual_quote_reserves == 0 {
        put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    }
    if log.real_quote_reserves == 0 {
        put_u64_if_nonzero(&mut log.real_quote_reserves, ix.real_quote_reserves);
    }
    if !log.is_created_buy && ix.is_created_buy {
        log.is_created_buy = true;
    }
}

#[inline]
fn merge_pumpfun_create_log_preferred(
    log: &mut PumpFunCreateTokenEvent,
    ix: PumpFunCreateTokenEvent,
) {
    fill_str_if_empty(&mut log.name, &ix.name);
    fill_str_if_empty(&mut log.symbol, &ix.symbol);
    fill_str_if_empty(&mut log.uri, &ix.uri);
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.creator, ix.creator);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pumpfun_quote_mint(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    if log.creator_fee_bps == 0 {
        log.creator_fee_bps = ix.creator_fee_bps;
    }
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
    log.is_holder_reward |= ix.is_holder_reward;
}

#[inline]
fn merge_pumpfun_create_v2_into_create_log_preferred(
    log: &mut PumpFunCreateTokenEvent,
    ix: PumpFunCreateV2TokenEvent,
) {
    fill_str_if_empty(&mut log.name, &ix.name);
    fill_str_if_empty(&mut log.symbol, &ix.symbol);
    fill_str_if_empty(&mut log.uri, &ix.uri);
    fill_pk(&mut log.mint, ix.mint);
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.creator, ix.creator);
    if log.timestamp == 0 && ix.timestamp != 0 {
        log.timestamp = ix.timestamp;
    }
    put_u64_if_nonzero(&mut log.virtual_token_reserves, ix.virtual_token_reserves);
    put_u64_if_nonzero(&mut log.virtual_sol_reserves, ix.virtual_sol_reserves);
    put_u64_if_nonzero(&mut log.real_token_reserves, ix.real_token_reserves);
    put_u64_if_nonzero(&mut log.token_total_supply, ix.token_total_supply);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pumpfun_quote_mint(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    if log.creator_fee_bps == 0 {
        log.creator_fee_bps = ix.creator_fee_bps;
    }
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
    log.is_holder_reward |= ix.is_holder_reward;
}

#[inline]
fn merge_pumpfun_create_into_create_v2_log_preferred(
    log: &mut PumpFunCreateV2TokenEvent,
    ix: PumpFunCreateTokenEvent,
) {
    fill_str_if_empty(&mut log.name, &ix.name);
    fill_str_if_empty(&mut log.symbol, &ix.symbol);
    fill_str_if_empty(&mut log.uri, &ix.uri);
    fill_pk(&mut log.mint, ix.mint);
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.creator, ix.creator);
    if log.timestamp == 0 && ix.timestamp != 0 {
        log.timestamp = ix.timestamp;
    }
    put_u64_if_nonzero(&mut log.virtual_token_reserves, ix.virtual_token_reserves);
    put_u64_if_nonzero(&mut log.virtual_sol_reserves, ix.virtual_sol_reserves);
    put_u64_if_nonzero(&mut log.real_token_reserves, ix.real_token_reserves);
    put_u64_if_nonzero(&mut log.token_total_supply, ix.token_total_supply);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pumpfun_quote_mint(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    if log.creator_fee_bps == 0 {
        log.creator_fee_bps = ix.creator_fee_bps;
    }
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
    log.is_holder_reward |= ix.is_holder_reward;
}

#[inline]
fn merge_pumpfun_create_v2_log_preferred(
    log: &mut PumpFunCreateV2TokenEvent,
    ix: PumpFunCreateV2TokenEvent,
) {
    fill_str_if_empty(&mut log.name, &ix.name);
    fill_str_if_empty(&mut log.symbol, &ix.symbol);
    fill_str_if_empty(&mut log.uri, &ix.uri);
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.creator, ix.creator);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pumpfun_quote_mint(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    if log.creator_fee_bps == 0 {
        log.creator_fee_bps = ix.creator_fee_bps;
    }
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    fill_pk(&mut log.mint_authority, ix.mint_authority);
    fill_pk(&mut log.associated_bonding_curve, ix.associated_bonding_curve);
    fill_pk(&mut log.global, ix.global);
    fill_pk(&mut log.system_program, ix.system_program);
    fill_pk(&mut log.associated_token_program, ix.associated_token_program);
    fill_pk(&mut log.mayhem_program_id, ix.mayhem_program_id);
    fill_pk(&mut log.global_params, ix.global_params);
    fill_pk(&mut log.sol_vault, ix.sol_vault);
    fill_pk(&mut log.mayhem_state, ix.mayhem_state);
    fill_pk(&mut log.mayhem_token_vault, ix.mayhem_token_vault);
    fill_pk(&mut log.event_authority, ix.event_authority);
    fill_pk(&mut log.program, ix.program);
    fill_pk(&mut log.observed_fee_recipient, ix.observed_fee_recipient);
    log.is_holder_reward |= ix.is_holder_reward;
}

#[inline]
fn merge_pumpfun_migrate_log_preferred(log: &mut PumpFunMigrateEvent, ix: PumpFunMigrateEvent) {
    fill_pk(&mut log.bonding_curve, ix.bonding_curve);
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.user, ix.user);
}

#[inline]
fn merge_pumpswap_trade_log_preferred(log: &mut PumpSwapTradeEvent, ix: PumpSwapTradeEvent) {
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
}

#[inline]
fn merge_pumpswap_buy_log_preferred(log: &mut PumpSwapBuyEvent, ix: PumpSwapBuyEvent) {
    fill_pk(&mut log.user_base_token_account, ix.user_base_token_account);
    fill_pk(&mut log.user_quote_token_account, ix.user_quote_token_account);
    fill_pk(&mut log.protocol_fee_recipient, ix.protocol_fee_recipient);
    fill_pk(&mut log.protocol_fee_recipient_token_account, ix.protocol_fee_recipient_token_account);
    fill_pk(&mut log.coin_creator, ix.coin_creator);
    fill_pk(&mut log.base_mint, ix.base_mint);
    fill_pk(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.pool_base_token_account, ix.pool_base_token_account);
    fill_pk(&mut log.pool_quote_token_account, ix.pool_quote_token_account);
    fill_pk(&mut log.coin_creator_vault_ata, ix.coin_creator_vault_ata);
    fill_pk(&mut log.coin_creator_vault_authority, ix.coin_creator_vault_authority);
    fill_pk(&mut log.base_token_program, ix.base_token_program);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    fill_pk(&mut log.pool_v2, ix.pool_v2);
    fill_pk(&mut log.fee_recipient, ix.fee_recipient);
    fill_pk(&mut log.fee_recipient_quote_token_account, ix.fee_recipient_quote_token_account);
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
}

#[inline]
fn merge_pumpswap_sell_log_preferred(log: &mut PumpSwapSellEvent, ix: PumpSwapSellEvent) {
    fill_pk(&mut log.user_base_token_account, ix.user_base_token_account);
    fill_pk(&mut log.user_quote_token_account, ix.user_quote_token_account);
    fill_pk(&mut log.protocol_fee_recipient, ix.protocol_fee_recipient);
    fill_pk(&mut log.protocol_fee_recipient_token_account, ix.protocol_fee_recipient_token_account);
    fill_pk(&mut log.coin_creator, ix.coin_creator);
    fill_pk(&mut log.base_mint, ix.base_mint);
    fill_pk(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.pool_base_token_account, ix.pool_base_token_account);
    fill_pk(&mut log.pool_quote_token_account, ix.pool_quote_token_account);
    fill_pk(&mut log.coin_creator_vault_ata, ix.coin_creator_vault_ata);
    fill_pk(&mut log.coin_creator_vault_authority, ix.coin_creator_vault_authority);
    fill_pk(&mut log.base_token_program, ix.base_token_program);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    fill_pk(&mut log.pool_v2, ix.pool_v2);
    fill_pk(&mut log.fee_recipient, ix.fee_recipient);
    fill_pk(&mut log.fee_recipient_quote_token_account, ix.fee_recipient_quote_token_account);
}

#[inline]
fn merge_raydium_clmm_swap_log_preferred(log: &mut RaydiumClmmSwapEvent, ix: RaydiumClmmSwapEvent) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.amount = ix.amount;
        log.other_amount_threshold = ix.other_amount_threshold;
        log.sqrt_price_limit_x64 = ix.sqrt_price_limit_x64;
        log.is_base_input = ix.is_base_input;
    }
    fill_pk(&mut log.token_account_0, ix.token_account_0);
    fill_pk(&mut log.token_account_1, ix.token_account_1);
    fill_pk(&mut log.input_token_account, ix.input_token_account);
    fill_pk(&mut log.output_token_account, ix.output_token_account);
    fill_pk(&mut log.sender, ix.sender);
    fill_pk(&mut log.amm_config, ix.amm_config);
    fill_pk(&mut log.input_vault, ix.input_vault);
    fill_pk(&mut log.output_vault, ix.output_vault);
    fill_pk(&mut log.observation_state, ix.observation_state);
    fill_pk(&mut log.input_mint, ix.input_mint);
    fill_pk(&mut log.output_mint, ix.output_mint);
    if log.tick_array_bitmap_extension.is_none() {
        log.tick_array_bitmap_extension = ix.tick_array_bitmap_extension;
    }
    if log.tick_arrays.is_empty() && !ix.tick_arrays.is_empty() {
        log.tick_arrays = ix.tick_arrays;
    }
}

#[inline]
fn merge_raydium_amm_v4_swap_log_preferred(
    log: &mut RaydiumAmmV4SwapEvent,
    ix: RaydiumAmmV4SwapEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.instruction_amount_in = ix.instruction_amount_in;
        log.instruction_amount_out = ix.instruction_amount_out;
        log.minimum_amount_out = ix.minimum_amount_out;
        log.max_amount_in = ix.max_amount_in;
    }
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pk(&mut log.amm_authority, ix.amm_authority);
    fill_pk(&mut log.amm_open_orders, ix.amm_open_orders);
    if let Some(ref o) = ix.amm_target_orders {
        if log.amm_target_orders.is_none() {
            log.amm_target_orders = Some(*o);
        }
    }
    fill_pk(&mut log.pool_coin_token_account, ix.pool_coin_token_account);
    fill_pk(&mut log.pool_pc_token_account, ix.pool_pc_token_account);
    fill_pk(&mut log.serum_program, ix.serum_program);
    fill_pk(&mut log.serum_market, ix.serum_market);
    fill_pk(&mut log.serum_bids, ix.serum_bids);
    fill_pk(&mut log.serum_asks, ix.serum_asks);
    fill_pk(&mut log.serum_event_queue, ix.serum_event_queue);
    fill_pk(&mut log.serum_coin_vault_account, ix.serum_coin_vault_account);
    fill_pk(&mut log.serum_pc_vault_account, ix.serum_pc_vault_account);
    fill_pk(&mut log.serum_vault_signer, ix.serum_vault_signer);
    fill_pk(&mut log.user_source_token_account, ix.user_source_token_account);
    fill_pk(&mut log.user_destination_token_account, ix.user_destination_token_account);
    fill_pk(&mut log.user_source_owner, ix.user_source_owner);
    fill_pk(&mut log.amm, ix.amm);
}

#[inline]
fn merge_pumpswap_create_pool_log_preferred(
    log: &mut PumpSwapCreatePoolEvent,
    ix: PumpSwapCreatePoolEvent,
) {
    fill_pk(&mut log.creator, ix.creator);
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.lp_mint, ix.lp_mint);
    fill_pk(&mut log.user_base_token_account, ix.user_base_token_account);
    fill_pk(&mut log.user_quote_token_account, ix.user_quote_token_account);
    fill_pk(&mut log.coin_creator, ix.coin_creator);
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_coin |= ix.is_cashback_coin;
    if log.creator_fee_bps == 0 {
        log.creator_fee_bps = ix.creator_fee_bps;
    }
    log.can_edit_creator_fee |= ix.can_edit_creator_fee;
    log.is_holder_reward |= ix.is_holder_reward;
}

#[inline]
fn merge_pumpswap_liquidity_added_log_preferred(
    log: &mut PumpSwapLiquidityAdded,
    ix: PumpSwapLiquidityAdded,
) {
    fill_pk(&mut log.user_base_token_account, ix.user_base_token_account);
    fill_pk(&mut log.user_quote_token_account, ix.user_quote_token_account);
    fill_pk(&mut log.user_pool_token_account, ix.user_pool_token_account);
}

#[inline]
fn merge_pumpswap_liquidity_removed_log_preferred(
    log: &mut PumpSwapLiquidityRemoved,
    ix: PumpSwapLiquidityRemoved,
) {
    fill_pk(&mut log.user_base_token_account, ix.user_base_token_account);
    fill_pk(&mut log.user_quote_token_account, ix.user_quote_token_account);
    fill_pk(&mut log.user_pool_token_account, ix.user_pool_token_account);
}

#[inline]
fn merge_raydium_launchlab_pool_create_log_preferred(
    log: &mut RaydiumLaunchlabPoolCreateEvent,
    ix: RaydiumLaunchlabPoolCreateEvent,
) {
    fill_pk(&mut log.payer, ix.payer);
    fill_pk(&mut log.creator, ix.creator);
    fill_pk(&mut log.global_config, ix.global_config);
    fill_pk(&mut log.platform_config, ix.platform_config);
    fill_pk(&mut log.base_mint, ix.base_mint);
    fill_pk(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.base_vault, ix.base_vault);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.base_token_program, ix.base_token_program);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    fill_str_if_empty(&mut log.base_mint_param.name, &ix.base_mint_param.name);
    fill_str_if_empty(&mut log.base_mint_param.symbol, &ix.base_mint_param.symbol);
    fill_str_if_empty(&mut log.base_mint_param.uri, &ix.base_mint_param.uri);
}

#[inline]
fn merge_raydium_launchlab_migrate_amm_log_preferred(
    log: &mut RaydiumLaunchlabMigrateAmmEvent,
    ix: RaydiumLaunchlabMigrateAmmEvent,
) {
    fill_pk(&mut log.old_pool, ix.old_pool);
    fill_pk(&mut log.new_pool, ix.new_pool);
    fill_pk(&mut log.user, ix.user);
}

#[inline]
fn merge_raydium_launchlab_trade_log_preferred(
    log: &mut RaydiumLaunchlabTradeEvent,
    ix: RaydiumLaunchlabTradeEvent,
) {
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.global_config, ix.global_config);
    fill_pk(&mut log.platform_config, ix.platform_config);
    fill_pk(&mut log.user_base_token, ix.user_base_token);
    fill_pk(&mut log.user_quote_token, ix.user_quote_token);
    fill_pk(&mut log.base_vault, ix.base_vault);
    fill_pk(&mut log.quote_vault, ix.quote_vault);
    fill_pk(&mut log.base_mint, ix.base_mint);
    fill_pk(&mut log.quote_mint, ix.quote_mint);
    fill_pk(&mut log.base_token_program, ix.base_token_program);
    fill_pk(&mut log.quote_token_program, ix.quote_token_program);
    fill_pk(&mut log.system_program, ix.system_program);
    fill_pk(&mut log.platform_associated_account, ix.platform_associated_account);
    fill_pk(&mut log.creator_associated_account, ix.creator_associated_account);
}

#[inline]
fn merge_meteora_dlmm_swap_log_preferred(log: &mut MeteoraDlmmSwapEvent, ix: MeteoraDlmmSwapEvent) {
    fill_pk(&mut log.user_token_in, ix.user_token_in);
    fill_pk(&mut log.user_token_out, ix.user_token_out);
    if log.min_amount_out == 0 {
        log.min_amount_out = ix.min_amount_out;
    }
}

/// 将 **instruction 路径**解析结果合并进 **log 路径**事件：`log` 保留链上日志权威数值，
/// `ix` 仅填补 `log` 中为默认值的账户等字段。**不替换** `log` 外层枚举变体。
///
/// 已覆盖与 [`crate::grpc::log_instr_dedup`] 去重键一致的主要类型：PumpFun 全系、PumpSwap
///（Trade/Buy/Sell/CreatePool/加减流动性）、RaydiumLaunchlab（Trade/PoolCreate/Migrate）、Raydium CLMM/AMM V4 Swap、Meteora DLMM Swap。
fn merge_meteora_pools_add_context(
    log: &mut MeteoraPoolsAddLiquidityEvent,
    ix: MeteoraPoolsAddLiquidityEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.pool_token_amount = ix.pool_token_amount;
        log.maximum_token_a_amount = ix.maximum_token_a_amount;
        log.maximum_token_b_amount = ix.maximum_token_b_amount;
        log.minimum_pool_token_amount = ix.minimum_pool_token_amount;
        log.token_a_in_amount = ix.token_a_in_amount;
        log.token_b_in_amount = ix.token_b_in_amount;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.lp_mint, ix.lp_mint);
    fill_pk(&mut log.user_pool_lp, ix.user_pool_lp);
    fill_pk(&mut log.a_vault_lp, ix.a_vault_lp);
    fill_pk(&mut log.b_vault_lp, ix.b_vault_lp);
    fill_pk(&mut log.a_vault, ix.a_vault);
    fill_pk(&mut log.b_vault, ix.b_vault);
    fill_pk(&mut log.a_vault_lp_mint, ix.a_vault_lp_mint);
    fill_pk(&mut log.b_vault_lp_mint, ix.b_vault_lp_mint);
    fill_pk(&mut log.a_token_vault, ix.a_token_vault);
    fill_pk(&mut log.b_token_vault, ix.b_token_vault);
    fill_pk(&mut log.user_a_token, ix.user_a_token);
    fill_pk(&mut log.user_b_token, ix.user_b_token);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.vault_program, ix.vault_program);
    fill_pk(&mut log.token_program, ix.token_program);
}

fn merge_meteora_pools_bootstrap_context(
    log: &mut MeteoraPoolsBootstrapLiquidityEvent,
    ix: MeteoraPoolsBootstrapLiquidityEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.token_a_in_amount = ix.token_a_in_amount;
        log.token_b_in_amount = ix.token_b_in_amount;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.lp_mint, ix.lp_mint);
    fill_pk(&mut log.user_pool_lp, ix.user_pool_lp);
    fill_pk(&mut log.a_vault_lp, ix.a_vault_lp);
    fill_pk(&mut log.b_vault_lp, ix.b_vault_lp);
    fill_pk(&mut log.a_vault, ix.a_vault);
    fill_pk(&mut log.b_vault, ix.b_vault);
    fill_pk(&mut log.a_vault_lp_mint, ix.a_vault_lp_mint);
    fill_pk(&mut log.b_vault_lp_mint, ix.b_vault_lp_mint);
    fill_pk(&mut log.a_token_vault, ix.a_token_vault);
    fill_pk(&mut log.b_token_vault, ix.b_token_vault);
    fill_pk(&mut log.user_a_token, ix.user_a_token);
    fill_pk(&mut log.user_b_token, ix.user_b_token);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.vault_program, ix.vault_program);
    fill_pk(&mut log.token_program, ix.token_program);
}

fn merge_meteora_pools_poolcreated_context(
    log: &mut MeteoraPoolsPoolCreatedEvent,
    ix: MeteoraPoolsPoolCreatedEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.token_a_in_amount = ix.token_a_in_amount;
        log.token_b_in_amount = ix.token_b_in_amount;
        log.activation_point = ix.activation_point;
        log.stable_curve = ix.stable_curve;
        log.trade_fee_bps = ix.trade_fee_bps;
        log.customizable_params = ix.customizable_params;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.config, ix.config);
    fill_pk(&mut log.lp_mint, ix.lp_mint);
    fill_pk(&mut log.token_a_mint, ix.token_a_mint);
    fill_pk(&mut log.token_b_mint, ix.token_b_mint);
    fill_pk(&mut log.a_vault, ix.a_vault);
    fill_pk(&mut log.b_vault, ix.b_vault);
    fill_pk(&mut log.a_token_vault, ix.a_token_vault);
    fill_pk(&mut log.b_token_vault, ix.b_token_vault);
    fill_pk(&mut log.a_vault_lp_mint, ix.a_vault_lp_mint);
    fill_pk(&mut log.b_vault_lp_mint, ix.b_vault_lp_mint);
    fill_pk(&mut log.a_vault_lp, ix.a_vault_lp);
    fill_pk(&mut log.b_vault_lp, ix.b_vault_lp);
    fill_pk(&mut log.payer_token_a, ix.payer_token_a);
    fill_pk(&mut log.payer_token_b, ix.payer_token_b);
    fill_pk(&mut log.payer_pool_lp, ix.payer_pool_lp);
    fill_pk(&mut log.protocol_token_a_fee, ix.protocol_token_a_fee);
    fill_pk(&mut log.protocol_token_b_fee, ix.protocol_token_b_fee);
    fill_pk(&mut log.payer, ix.payer);
    fill_pk(&mut log.rent, ix.rent);
    fill_pk(&mut log.mint_metadata, ix.mint_metadata);
    fill_pk(&mut log.metadata_program, ix.metadata_program);
    fill_pk(&mut log.vault_program, ix.vault_program);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pk(&mut log.associated_token_program, ix.associated_token_program);
    fill_pk(&mut log.system_program, ix.system_program);
    fill_pk(&mut log.admin_token_a, ix.admin_token_a);
    fill_pk(&mut log.admin_token_b, ix.admin_token_b);
    fill_pk(&mut log.admin_pool_lp, ix.admin_pool_lp);
    fill_pk(&mut log.admin, ix.admin);
    fill_pk(&mut log.fee_owner, ix.fee_owner);
}

fn merge_meteora_pools_setpoolfees_context(
    log: &mut MeteoraPoolsSetPoolFeesEvent,
    ix: MeteoraPoolsSetPoolFeesEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.new_partner_fee_numerator = ix.new_partner_fee_numerator;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.fee_operator, ix.fee_operator);
}

fn merge_meteora_pools_remove_context(
    log: &mut MeteoraPoolsRemoveLiquidityEvent,
    ix: MeteoraPoolsRemoveLiquidityEvent,
) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.pool_token_amount = ix.pool_token_amount;
        log.minimum_out_amount = ix.minimum_out_amount;
        log.minimum_a_token_out = ix.minimum_a_token_out;
        log.minimum_b_token_out = ix.minimum_b_token_out;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.lp_mint, ix.lp_mint);
    fill_pk(&mut log.user_pool_lp, ix.user_pool_lp);
    fill_pk(&mut log.a_vault_lp, ix.a_vault_lp);
    fill_pk(&mut log.b_vault_lp, ix.b_vault_lp);
    fill_pk(&mut log.a_vault, ix.a_vault);
    fill_pk(&mut log.b_vault, ix.b_vault);
    fill_pk(&mut log.a_vault_lp_mint, ix.a_vault_lp_mint);
    fill_pk(&mut log.b_vault_lp_mint, ix.b_vault_lp_mint);
    fill_pk(&mut log.a_token_vault, ix.a_token_vault);
    fill_pk(&mut log.b_token_vault, ix.b_token_vault);
    fill_pk(&mut log.user_a_token, ix.user_a_token);
    fill_pk(&mut log.user_b_token, ix.user_b_token);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.vault_program, ix.vault_program);
    fill_pk(&mut log.token_program, ix.token_program);
    fill_pk(&mut log.user_destination_token, ix.user_destination_token);
}

fn merge_meteora_pools_swap_context(log: &mut MeteoraPoolsSwapEvent, ix: MeteoraPoolsSwapEvent) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.amount_in = ix.amount_in;
        log.minimum_out_amount = ix.minimum_out_amount;
    }
    fill_pk(&mut log.pool, ix.pool);
    fill_pk(&mut log.user_source_token, ix.user_source_token);
    fill_pk(&mut log.user_destination_token, ix.user_destination_token);
    fill_pk(&mut log.a_vault, ix.a_vault);
    fill_pk(&mut log.b_vault, ix.b_vault);
    fill_pk(&mut log.a_token_vault, ix.a_token_vault);
    fill_pk(&mut log.b_token_vault, ix.b_token_vault);
    fill_pk(&mut log.a_vault_lp_mint, ix.a_vault_lp_mint);
    fill_pk(&mut log.b_vault_lp_mint, ix.b_vault_lp_mint);
    fill_pk(&mut log.a_vault_lp, ix.a_vault_lp);
    fill_pk(&mut log.b_vault_lp, ix.b_vault_lp);
    fill_pk(&mut log.protocol_token_fee, ix.protocol_token_fee);
    fill_pk(&mut log.user, ix.user);
    fill_pk(&mut log.vault_program, ix.vault_program);
    fill_pk(&mut log.token_program, ix.token_program);
}

fn merge_orca_swap_context(log: &mut OrcaWhirlpoolSwapEvent, ix: OrcaWhirlpoolSwapEvent) {
    if !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
        log.amount = ix.amount;
        log.other_amount_threshold = ix.other_amount_threshold;
        log.sqrt_price_limit = ix.sqrt_price_limit;
        log.amount_specified_is_input = ix.amount_specified_is_input;
    }
    fill_pk(&mut log.token_authority, ix.token_authority);
    fill_pk(&mut log.token_owner_account_a, ix.token_owner_account_a);
    fill_pk(&mut log.token_owner_account_b, ix.token_owner_account_b);
    fill_pk(&mut log.token_program_a, ix.token_program_a);
    fill_pk(&mut log.token_program_b, ix.token_program_b);
    fill_pk(&mut log.token_mint_a, ix.token_mint_a);
    fill_pk(&mut log.token_mint_b, ix.token_mint_b);
    fill_pk(&mut log.token_vault_a, ix.token_vault_a);
    fill_pk(&mut log.token_vault_b, ix.token_vault_b);
    fill_pk(&mut log.tick_array_0, ix.tick_array_0);
    fill_pk(&mut log.tick_array_1, ix.tick_array_1);
    fill_pk(&mut log.tick_array_2, ix.tick_array_2);
    fill_pk(&mut log.oracle, ix.oracle);
}

pub fn merge_grpc_instruction_into_log(log: &mut DexEvent, ix: DexEvent) {
    use DexEvent::*;
    match log {
        PumpFunTrade(l) => {
            if let Some(i) = pumpfun_trade_from_ix_variant(ix) {
                merge_pumpfun_trade_log_preferred(l, i);
            }
        }
        PumpFunBuy(l) => {
            if let Some(i) = pumpfun_trade_from_ix_variant(ix) {
                merge_pumpfun_trade_log_preferred(l, i);
            }
        }
        PumpFunSell(l) => {
            if let Some(i) = pumpfun_trade_from_ix_variant(ix) {
                merge_pumpfun_trade_log_preferred(l, i);
            }
        }
        PumpFunBuyExactSolIn(l) => {
            if let Some(i) = pumpfun_trade_from_ix_variant(ix) {
                merge_pumpfun_trade_log_preferred(l, i);
            }
        }
        PumpFunCreate(l) => match ix {
            DexEvent::PumpFunCreate(i) => merge_pumpfun_create_log_preferred(l, i),
            DexEvent::PumpFunCreateV2(i) => merge_pumpfun_create_v2_into_create_log_preferred(l, i),
            _ => {}
        },
        PumpFunCreateV2(l) => match ix {
            DexEvent::PumpFunCreate(i) => merge_pumpfun_create_into_create_v2_log_preferred(l, i),
            DexEvent::PumpFunCreateV2(i) => merge_pumpfun_create_v2_log_preferred(l, i),
            _ => {}
        },
        PumpFunMigrate(l) => {
            if let DexEvent::PumpFunMigrate(i) = ix {
                merge_pumpfun_migrate_log_preferred(l, i);
            }
        }
        PumpSwapTrade(l) => {
            if let PumpSwapTrade(i) = ix {
                merge_pumpswap_trade_log_preferred(l, i);
            }
        }
        PumpSwapBuy(l) => {
            if let PumpSwapBuy(i) = ix {
                merge_pumpswap_buy_log_preferred(l, i);
            }
        }
        PumpSwapSell(l) => {
            if let PumpSwapSell(i) = ix {
                merge_pumpswap_sell_log_preferred(l, i);
            }
        }
        RaydiumCpmmSwap(l) => {
            if let RaydiumCpmmSwap(i) = ix {
                merge_raydium_cpmm_swap_log_preferred(l, i);
            }
        }
        MeteoraPoolsAddLiquidity(l) => {
            if let MeteoraPoolsAddLiquidity(i) = ix {
                merge_meteora_pools_add_context(l, i);
            }
        }
        MeteoraPoolsRemoveLiquidity(l) => {
            if let MeteoraPoolsRemoveLiquidity(i) = ix {
                merge_meteora_pools_remove_context(l, i);
            }
        }
        MeteoraPoolsBootstrapLiquidity(l) => {
            if let MeteoraPoolsBootstrapLiquidity(i) = ix {
                merge_meteora_pools_bootstrap_context(l, i);
            }
        }
        MeteoraPoolsPoolCreated(l) => {
            if let MeteoraPoolsPoolCreated(i) = ix {
                merge_meteora_pools_poolcreated_context(l, i);
            }
        }
        MeteoraPoolsSetPoolFees(l) => {
            if let MeteoraPoolsSetPoolFees(i) = ix {
                merge_meteora_pools_setpoolfees_context(l, i);
            }
        }
        MeteoraPoolsSwap(l) => {
            if let MeteoraPoolsSwap(i) = ix {
                merge_meteora_pools_swap_context(l, i);
            }
        }
        OrcaWhirlpoolSwap(l) => {
            if let OrcaWhirlpoolSwap(i) = ix {
                merge_orca_swap_context(l, i);
            }
        }
        RaydiumClmmIncreaseLiquidity(l) => {
            if let RaydiumClmmIncreaseLiquidity(i) = ix {
                fill_pk(&mut l.pool, i.pool);
                fill_pk(&mut l.user, i.user);
                fill_pk(&mut l.personal_position, i.personal_position);
                l.amount0_max = i.amount0_max;
                l.amount1_max = i.amount1_max;
            }
        }
        RaydiumClmmDecreaseLiquidity(l) => {
            if let RaydiumClmmDecreaseLiquidity(i) = ix {
                fill_pk(&mut l.pool, i.pool);
                fill_pk(&mut l.user, i.user);
                fill_pk(&mut l.personal_position, i.personal_position);
                l.amount0_min = i.amount0_min;
                l.amount1_min = i.amount1_min;
            }
        }
        RaydiumClmmSwap(l) => {
            if let RaydiumClmmSwap(i) = ix {
                merge_raydium_clmm_swap_log_preferred(l, i);
            }
        }
        RaydiumAmmV4Swap(l) => {
            if let RaydiumAmmV4Swap(i) = ix {
                merge_raydium_amm_v4_swap_log_preferred(l, i);
            }
        }
        RaydiumLaunchlabTrade(l) => {
            if let RaydiumLaunchlabTrade(i) = ix {
                merge_raydium_launchlab_trade_log_preferred(l, i);
            }
        }
        RaydiumLaunchlabPoolCreate(l) => {
            if let RaydiumLaunchlabPoolCreate(i) = ix {
                merge_raydium_launchlab_pool_create_log_preferred(l, i);
            }
        }
        RaydiumLaunchlabMigrateAmm(l) => {
            if let RaydiumLaunchlabMigrateAmm(i) = ix {
                merge_raydium_launchlab_migrate_amm_log_preferred(l, i);
            }
        }
        PumpSwapCreatePool(l) => {
            if let PumpSwapCreatePool(i) = ix {
                merge_pumpswap_create_pool_log_preferred(l, i);
            }
        }
        PumpSwapLiquidityAdded(l) => {
            if let PumpSwapLiquidityAdded(i) = ix {
                merge_pumpswap_liquidity_added_log_preferred(l, i);
            }
        }
        PumpSwapLiquidityRemoved(l) => {
            if let PumpSwapLiquidityRemoved(i) = ix {
                merge_pumpswap_liquidity_removed_log_preferred(l, i);
            }
        }
        MeteoraDlmmSwap(l) => {
            if let MeteoraDlmmSwap(i) = ix {
                merge_meteora_dlmm_swap_log_preferred(l, i);
            }
        }
        _ => {}
    }
}

#[inline]
fn pumpfun_trade_from_ix_variant(ix: DexEvent) -> Option<PumpFunTradeEvent> {
    match ix {
        DexEvent::PumpFunTrade(t)
        | DexEvent::PumpFunBuy(t)
        | DexEvent::PumpFunSell(t)
        | DexEvent::PumpFunBuyExactSolIn(t) => Some(t),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn clmm_merge_retains_limits_and_mode_without_overwriting_executed_price_or_direction() {
        use crate::core::events::{DexEvent, RaydiumClmmSwapEvent};
        for input_mode in [false, true] {
            let instruction = RaydiumClmmSwapEvent {
                ix_name: "swap_v2".to_string(),
                amount: 0,
                other_amount_threshold: u64::MAX,
                sqrt_price_limit_x64: 0,
                is_base_input: input_mode,
                input_mint: solana_sdk::pubkey::Pubkey::new_unique(),
                tick_arrays: vec![solana_sdk::pubkey::Pubkey::new_unique()],
                ..Default::default()
            };
            let log = RaydiumClmmSwapEvent {
                amount_0: 456,
                amount_1: 123,
                sqrt_price_x64: 789,
                zero_for_one: !input_mode,
                tick: -7,
                liquidity: 1011,
                amount: 99,
                sqrt_price_limit_x64: 99,
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut merged = DexEvent::RaydiumClmmSwap(if log_first {
                    log.clone()
                } else {
                    instruction.clone()
                });
                if log_first {
                    super::merge_grpc_instruction_into_log(
                        &mut merged,
                        DexEvent::RaydiumClmmSwap(instruction.clone()),
                    );
                } else {
                    super::merge_events(&mut merged, DexEvent::RaydiumClmmSwap(log.clone()));
                }
                let DexEvent::RaydiumClmmSwap(e) = merged else { panic!("swap") };
                assert_eq!(
                    (e.amount_0, e.amount_1, e.sqrt_price_x64, e.tick, e.liquidity),
                    (456, 123, 789, -7, 1011)
                );
                assert_eq!(e.zero_for_one, !input_mode);
                assert_eq!(
                    (e.amount, e.other_amount_threshold, e.sqrt_price_limit_x64),
                    (0, u64::MAX, 0)
                );
                assert_eq!(e.is_base_input, input_mode);
                assert_eq!(e.ix_name, "swap_v2");
                assert_eq!(e.input_mint, instruction.input_mint);
                assert_eq!(e.tick_arrays, instruction.tick_arrays);
            }
        }
    }

    #[test]
    fn cpmm_merge_preserves_zero_limits_and_execution_values_in_both_paths() {
        use crate::core::events::{DexEvent, RaydiumCpmmSwapEvent};
        for exact_input in [false, true] {
            let payer = solana_sdk::pubkey::Pubkey::new_unique();
            let input_account = solana_sdk::pubkey::Pubkey::new_unique();
            let instruction = RaydiumCpmmSwapEvent {
                ix_name: if exact_input { "swap_base_input" } else { "swap_base_output" }
                    .to_string(),
                base_input: exact_input,
                amount_in: 0,
                minimum_amount_out: if exact_input { u64::MAX } else { 0 },
                max_amount_in: if exact_input { 0 } else { u64::MAX },
                amount_out: 0,
                payer,
                input_token_account: input_account,
                ..Default::default()
            };
            let log = RaydiumCpmmSwapEvent {
                base_input: exact_input,
                input_amount: 456,
                output_amount: 123,
                input_transfer_fee: 7,
                amount_in: 99,
                amount_out: 99,
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut merged = if log_first {
                    DexEvent::RaydiumCpmmSwap(log.clone())
                } else {
                    DexEvent::RaydiumCpmmSwap(instruction.clone())
                };
                if log_first {
                    super::merge_grpc_instruction_into_log(
                        &mut merged,
                        DexEvent::RaydiumCpmmSwap(instruction.clone()),
                    );
                } else {
                    super::merge_events(&mut merged, DexEvent::RaydiumCpmmSwap(log.clone()));
                }
                let DexEvent::RaydiumCpmmSwap(e) = merged else { panic!("swap") };
                assert_eq!((e.input_amount, e.output_amount, e.input_transfer_fee), (456, 123, 7));
                assert_eq!(e.ix_name, instruction.ix_name);
                assert_eq!(
                    (e.amount_in, e.minimum_amount_out, e.max_amount_in, e.amount_out),
                    (
                        instruction.amount_in,
                        instruction.minimum_amount_out,
                        instruction.max_amount_in,
                        0
                    )
                );
                assert_eq!(e.payer, payer);
                assert_eq!(e.input_token_account, input_account);
                assert_eq!(e.base_input, exact_input);
            }
        }
    }
    use super::*;

    #[test]
    fn incompatible_merge_leaves_both_events_unchanged() {
        let mut base = DexEvent::Error("base".to_string());
        let inner = DexEvent::Error("inner".to_string());
        let mut unmerged = None;

        assert!(!try_merge_events(&mut base, inner, &mut unmerged));
        assert!(matches!(base, DexEvent::Error(ref message) if message == "base"));
        assert!(matches!(unmerged, Some(DexEvent::Error(ref message)) if message == "inner"));
    }
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    fn dlmm_swap(min_amount_out: u64, amount_out: u64) -> MeteoraDlmmSwapEvent {
        MeteoraDlmmSwapEvent {
            metadata: EventMetadata::default(),
            token_x_mint: Pubkey::default(),
            token_y_mint: Pubkey::default(),
            user_token_in: Pubkey::new_unique(),
            user_token_out: Pubkey::new_unique(),
            min_amount_out,
            pool: Pubkey::new_unique(),
            from: Pubkey::new_unique(),
            start_bin_id: 0,
            end_bin_id: 0,
            amount_in: 200,
            amount_out,
            swap_for_y: true,
            fee: 1,
            protocol_fee: 0,
            fee_bps: 25,
            host_fee: 0,
            ..Default::default()
        }
    }

    #[test]
    fn dlmm_event_merge_keeps_instruction_threshold_and_executed_output() {
        let base_event = dlmm_swap(100, 0);
        let expected_user_token_in = base_event.user_token_in;
        let expected_user_token_out = base_event.user_token_out;
        let mut inner_event = dlmm_swap(0, 125);
        inner_event.user_token_in = Pubkey::default();
        inner_event.user_token_out = Pubkey::default();
        let mut base = DexEvent::MeteoraDlmmSwap(base_event);
        let inner = DexEvent::MeteoraDlmmSwap(inner_event);

        assert!(try_merge_events(&mut base, inner, &mut None));
        let DexEvent::MeteoraDlmmSwap(event) = base else { panic!("swap") };
        assert_eq!(event.min_amount_out, 100);
        assert_eq!(event.amount_out, 125);
        assert_eq!(event.user_token_in, expected_user_token_in);
        assert_eq!(event.user_token_out, expected_user_token_out);
    }

    #[test]
    fn grpc_dlmm_merge_keeps_log_output_and_adds_instruction_threshold() {
        let mut log_event = dlmm_swap(0, 125);
        log_event.user_token_in = Pubkey::default();
        log_event.user_token_out = Pubkey::default();
        let instruction_event = dlmm_swap(100, 0);
        let expected_user_token_in = instruction_event.user_token_in;
        let expected_user_token_out = instruction_event.user_token_out;
        let mut log = DexEvent::MeteoraDlmmSwap(log_event);
        let instruction = DexEvent::MeteoraDlmmSwap(instruction_event);

        merge_grpc_instruction_into_log(&mut log, instruction);
        let DexEvent::MeteoraDlmmSwap(event) = log else { panic!("swap") };
        assert_eq!(event.min_amount_out, 100);
        assert_eq!(event.amount_out, 125);
        assert_eq!(event.user_token_in, expected_user_token_in);
        assert_eq!(event.user_token_out, expected_user_token_out);
    }

    #[test]
    fn test_merge_pumpfun_trade() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        // Base event 来自 instruction（包含账户上下文）
        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            bonding_curve: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            ..Default::default()
        });

        // Inner event 来自 inner instruction（包含交易数据）
        let inner = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            mint: Pubkey::new_unique(),
            sol_amount: 1000,
            token_amount: 2000,
            is_buy: true,
            user: Pubkey::new_unique(),
            holder_rewards_bps: 300,
            holder_rewards: 400,
            ..Default::default()
        });

        // 合并
        merge_events(&mut base, inner);

        // 验证合并结果
        if let DexEvent::PumpFunTrade(trade) = base {
            assert_eq!(trade.sol_amount, 1000);
            assert_eq!(trade.token_amount, 2000);
            assert!(trade.is_buy);
            assert_eq!(trade.holder_rewards_bps, 300);
            assert_eq!(trade.holder_rewards, 400);
            // 账户上下文保留
            assert_ne!(trade.bonding_curve, Pubkey::default());
            assert_ne!(trade.associated_bonding_curve, Pubkey::default());
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn merge_pumpfun_trade_non_leg_propagates_holder_rewards() {
        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent::default());
        let inner = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            holder_rewards_bps: 300,
            holder_rewards: 400,
            ..Default::default()
        });

        merge_events(&mut base, inner);

        let DexEvent::PumpFunTrade(trade) = base else {
            panic!("expected PumpFunTrade event");
        };
        assert_eq!(trade.holder_rewards_bps, 300);
        assert_eq!(trade.holder_rewards, 400);
    }

    #[test]
    fn merge_preserves_instruction_context_when_log_tail_is_absent() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };
        let quote_mint = Pubkey::new_unique();
        let associated_quote_user = Pubkey::new_unique();

        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ix_name: "buy_exact_quote_in".to_string(),
            is_buy: true,
            quote_mint,
            spendable_quote_in: 1_000,
            min_tokens_out: 2_000,
            associated_quote_user,
            ..Default::default()
        });

        let inner = DexEvent::PumpFunBuy(PumpFunTradeEvent {
            metadata,
            sol_amount: 1_000,
            token_amount: 2_000,
            is_buy: true,
            ..Default::default()
        });

        merge_events(&mut base, inner);

        if let DexEvent::PumpFunTrade(t) = base {
            assert_eq!(t.sol_amount, 1_000);
            assert_eq!(t.token_amount, 2_000);
            assert_eq!(t.ix_name, "buy_exact_quote_in");
            assert_eq!(t.quote_mint, quote_mint);
            assert_eq!(t.spendable_quote_in, 1_000);
            assert_eq!(t.min_tokens_out, 2_000);
            assert_eq!(t.associated_quote_user, associated_quote_user);
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn merge_replaces_sol_quote_sentinel_with_real_quote_mint() {
        let quote_mint = Pubkey::new_unique();
        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        });

        let inner = DexEvent::PumpFunBuy(PumpFunTradeEvent { quote_mint, ..Default::default() });

        merge_events(&mut base, inner);

        if let DexEvent::PumpFunTrade(t) = base {
            assert_eq!(t.quote_mint, quote_mint);
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn merge_replaces_sol_quote_sentinel_with_wsol_mint() {
        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
            ..Default::default()
        });

        let inner = DexEvent::PumpFunBuy(PumpFunTradeEvent {
            quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
            ..Default::default()
        });

        merge_events(&mut base, inner);

        if let DexEvent::PumpFunTrade(t) = base {
            assert_eq!(t.quote_mint, PUMPFUN_WSOL_QUOTE_MINT);
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn test_can_merge() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        let base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ..Default::default()
        });

        let inner = DexEvent::PumpFunBuy(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ..Default::default()
        });

        // 应该可以合并（同一个 signature，兼容类型）
        assert!(can_merge(&base, &inner));

        // 不同 signature 不能合并
        let different_sig = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: EventMetadata { signature: Signature::new_unique(), ..metadata },
            ..Default::default()
        });

        assert!(!can_merge(&base, &different_sig));
    }

    #[test]
    fn dlmm_position_event_keeps_instruction_only_fields() {
        let pool = Pubkey::new_unique();
        let position = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut base = DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
            metadata: EventMetadata::default(),
            pool,
            position,
            owner,
            lower_bin_id: -42,
            width: 70,
        });
        let inner = DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
            metadata: EventMetadata::default(),
            pool,
            position,
            owner,
            lower_bin_id: 0,
            width: 0,
        });

        assert!(try_merge_events(&mut base, inner, &mut None));
        let DexEvent::MeteoraDlmmCreatePosition(event) = base else { panic!("position") };
        assert_eq!(event.lower_bin_id, -42);
        assert_eq!(event.width, 70);
    }

    #[test]
    fn grpc_merge_fills_fee_recipient_from_ix_when_log_default() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let fr = Pubkey::new_unique();
        let log_t =
            PumpFunTradeEvent { metadata: metadata.clone(), sol_amount: 50, ..Default::default() };
        let mut ix_t = log_t.clone();
        ix_t.fee_recipient = fr;
        ix_t.sol_amount = 777;
        let mut log_ev = DexEvent::PumpFunTrade(log_t);
        merge_grpc_instruction_into_log(&mut log_ev, DexEvent::PumpFunBuy(ix_t));
        match log_ev {
            DexEvent::PumpFunTrade(t) => {
                assert_eq!(t.fee_recipient, fr);
                assert_eq!(t.sol_amount, 50);
            }
            _ => panic!("expected trade"),
        }
    }

    #[test]
    fn grpc_merge_keeps_log_trade_fields() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 1,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };
        let log_t = PumpFunTradeEvent {
            metadata: metadata.clone(),
            mayhem_mode: true,
            sol_amount: 100,
            ..Default::default()
        };
        let mut ix_t = log_t.clone();
        ix_t.mayhem_mode = false;
        ix_t.sol_amount = 999;

        let mut log_ev = DexEvent::PumpFunTrade(log_t);
        merge_grpc_instruction_into_log(&mut log_ev, DexEvent::PumpFunBuy(ix_t));
        match log_ev {
            DexEvent::PumpFunTrade(t) => {
                assert!(t.mayhem_mode);
                assert_eq!(t.sol_amount, 100);
            }
            _ => panic!("variant preserved"),
        }
    }

    #[test]
    fn pumpfun_create_merge_propagates_holder_rewards_fields() {
        let mut base = DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            creator_fee_bps: 125,
            ..Default::default()
        });
        let inner = DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            creator_fee_bps: 300,
            is_holder_reward: true,
            ..Default::default()
        });

        merge_events(&mut base, inner);

        let DexEvent::PumpFunCreate(event) = base else { panic!("create") };
        assert_eq!(event.creator_fee_bps, 300);
        assert!(event.is_holder_reward);
    }

    #[test]
    fn grpc_create_merge_keeps_log_fee_and_adds_instruction_holder_reward() {
        let mut log = DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            creator_fee_bps: 300,
            ..Default::default()
        });
        let instruction = DexEvent::PumpFunCreateV2(PumpFunCreateV2TokenEvent {
            creator_fee_bps: 500,
            is_holder_reward: true,
            ..Default::default()
        });

        merge_grpc_instruction_into_log(&mut log, instruction);

        let DexEvent::PumpFunCreate(event) = log else { panic!("create") };
        assert_eq!(event.creator_fee_bps, 300);
        assert!(event.is_holder_reward);
    }
}

#[cfg(test)]
mod orca_swap_context_tests {
    use super::*;

    #[test]
    fn both_merge_paths_preserve_zero_wire_values_and_authoritative_execution() {
        for input_mode in [false, true] {
            let ix = OrcaWhirlpoolSwapEvent {
                ix_name: "swap_v2".into(),
                amount: 0,
                other_amount_threshold: u64::MAX,
                sqrt_price_limit: 0,
                amount_specified_is_input: input_mode,
                token_authority: Pubkey::new_unique(),
                token_owner_account_a: Pubkey::new_unique(),
                token_owner_account_b: Pubkey::new_unique(),
                ..Default::default()
            };
            let log = OrcaWhirlpoolSwapEvent {
                amount: 99,
                sqrt_price_limit: 99,
                input_amount: 123,
                output_amount: 456,
                pre_sqrt_price: 789,
                post_sqrt_price: 987,
                input_transfer_fee: 1,
                output_transfer_fee: 2,
                lp_fee: 3,
                protocol_fee: 4,
                a_to_b: true,
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut result = if log_first {
                    DexEvent::OrcaWhirlpoolSwap(log.clone())
                } else {
                    DexEvent::OrcaWhirlpoolSwap(ix.clone())
                };
                if log_first {
                    merge_grpc_instruction_into_log(
                        &mut result,
                        DexEvent::OrcaWhirlpoolSwap(ix.clone()),
                    );
                } else {
                    merge_events(&mut result, DexEvent::OrcaWhirlpoolSwap(log.clone()));
                }
                let DexEvent::OrcaWhirlpoolSwap(e) = result else { panic!("swap") };
                assert_eq!(
                    (e.amount, e.other_amount_threshold, e.sqrt_price_limit),
                    (0, u64::MAX, 0)
                );
                assert_eq!(e.amount_specified_is_input, input_mode);
                assert_eq!(e.ix_name, "swap_v2");
                assert_eq!(
                    (e.input_amount, e.output_amount, e.pre_sqrt_price, e.post_sqrt_price),
                    (123, 456, 789, 987)
                );
                assert_eq!(
                    (e.input_transfer_fee, e.output_transfer_fee, e.lp_fee, e.protocol_fee),
                    (1, 2, 3, 4)
                );
                assert!(e.a_to_b);
                assert_eq!(
                    (e.token_authority, e.token_owner_account_a, e.token_owner_account_b),
                    (ix.token_authority, ix.token_owner_account_a, ix.token_owner_account_b)
                );
            }
        }
    }
}

#[cfg(test)]
mod pools_swap_merge_tests {
    use super::*;

    #[test]
    fn both_paths_keep_instruction_zero_values_and_log_execution_values() {
        let ix = MeteoraPoolsSwapEvent {
            ix_name: "swap".into(),
            amount_in: 0,
            minimum_out_amount: u64::MAX,
            pool: Pubkey::new_unique(),
            user_source_token: Pubkey::new_unique(),
            user_destination_token: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            protocol_token_fee: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsSwapEvent {
            amount_in: 99,
            minimum_out_amount: 99,
            in_amount: 123,
            out_amount: 456,
            trade_fee: 1,
            admin_fee: 2,
            host_fee: 3,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result =
                DexEvent::MeteoraPoolsSwap(if log_first { log.clone() } else { ix.clone() });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsSwap(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsSwap(log.clone()));
            }
            let DexEvent::MeteoraPoolsSwap(e) = result else { panic!("swap") };
            assert_eq!(e.ix_name, "swap");
            assert_eq!((e.amount_in, e.minimum_out_amount), (0, u64::MAX));
            assert_eq!(
                (e.in_amount, e.out_amount, e.trade_fee, e.admin_fee, e.host_fee),
                (123, 456, 1, 2, 3)
            );
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.user_source_token, ix.user_source_token);
            assert_eq!(e.user_destination_token, ix.user_destination_token);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.protocol_token_fee, ix.protocol_token_fee);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.token_program, log.token_program);
        }
    }
}

#[cfg(test)]
mod pools_liquidity_merge_tests {
    use super::*;
    #[test]
    fn add_balance_liquidity_both_merge_paths_preserve_limits_and_execution() {
        let ix = MeteoraPoolsAddLiquidityEvent {
            ix_name: "add_balance_liquidity".into(),
            pool_token_amount: 0,
            maximum_token_a_amount: u64::MAX,
            maximum_token_b_amount: 0,
            pool: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            user_pool_lp: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            user_a_token: Pubkey::new_unique(),
            user_b_token: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsAddLiquidityEvent {
            pool_token_amount: 99,
            maximum_token_a_amount: 99,
            maximum_token_b_amount: 99,
            lp_mint_amount: 123,
            token_a_amount: 124,
            token_b_amount: 125,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result = DexEvent::MeteoraPoolsAddLiquidity(if log_first {
                log.clone()
            } else {
                ix.clone()
            });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsAddLiquidity(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsAddLiquidity(log.clone()));
            }
            let DexEvent::MeteoraPoolsAddLiquidity(e) = result else { panic!("liquidity") };
            assert_eq!(e.ix_name, "add_balance_liquidity");
            assert_eq!(e.pool_token_amount, ix.pool_token_amount);
            assert_eq!(e.maximum_token_a_amount, ix.maximum_token_a_amount);
            assert_eq!(e.maximum_token_b_amount, ix.maximum_token_b_amount);
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.lp_mint, ix.lp_mint);
            assert_eq!(e.user_pool_lp, ix.user_pool_lp);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.user_a_token, ix.user_a_token);
            assert_eq!(e.user_b_token, ix.user_b_token);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.lp_mint_amount, log.lp_mint_amount);
            assert_eq!(e.token_a_amount, log.token_a_amount);
            assert_eq!(e.token_b_amount, log.token_b_amount);
            assert_eq!(e.token_program, log.token_program);
        }
    }
    #[test]
    fn add_imbalance_liquidity_both_merge_paths_preserve_limits_and_execution() {
        let ix = MeteoraPoolsAddLiquidityEvent {
            ix_name: "add_imbalance_liquidity".into(),
            minimum_pool_token_amount: 0,
            token_a_in_amount: u64::MAX,
            token_b_in_amount: 0,
            pool: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            user_pool_lp: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            user_a_token: Pubkey::new_unique(),
            user_b_token: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsAddLiquidityEvent {
            minimum_pool_token_amount: 99,
            token_a_in_amount: 99,
            token_b_in_amount: 99,
            lp_mint_amount: 123,
            token_a_amount: 124,
            token_b_amount: 125,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result = DexEvent::MeteoraPoolsAddLiquidity(if log_first {
                log.clone()
            } else {
                ix.clone()
            });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsAddLiquidity(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsAddLiquidity(log.clone()));
            }
            let DexEvent::MeteoraPoolsAddLiquidity(e) = result else { panic!("liquidity") };
            assert_eq!(e.ix_name, "add_imbalance_liquidity");
            assert_eq!(e.minimum_pool_token_amount, ix.minimum_pool_token_amount);
            assert_eq!(e.token_a_in_amount, ix.token_a_in_amount);
            assert_eq!(e.token_b_in_amount, ix.token_b_in_amount);
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.lp_mint, ix.lp_mint);
            assert_eq!(e.user_pool_lp, ix.user_pool_lp);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.user_a_token, ix.user_a_token);
            assert_eq!(e.user_b_token, ix.user_b_token);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.lp_mint_amount, log.lp_mint_amount);
            assert_eq!(e.token_a_amount, log.token_a_amount);
            assert_eq!(e.token_b_amount, log.token_b_amount);
            assert_eq!(e.token_program, log.token_program);
        }
    }
    #[test]
    fn remove_balance_liquidity_both_merge_paths_preserve_limits_and_execution() {
        let ix = MeteoraPoolsRemoveLiquidityEvent {
            ix_name: "remove_balance_liquidity".into(),
            pool_token_amount: 0,
            minimum_a_token_out: u64::MAX,
            minimum_b_token_out: 0,
            pool: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            user_pool_lp: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            user_a_token: Pubkey::new_unique(),
            user_b_token: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsRemoveLiquidityEvent {
            pool_token_amount: 99,
            minimum_a_token_out: 99,
            minimum_b_token_out: 99,
            lp_unmint_amount: 123,
            token_a_out_amount: 124,
            token_b_out_amount: 125,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result = DexEvent::MeteoraPoolsRemoveLiquidity(if log_first {
                log.clone()
            } else {
                ix.clone()
            });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsRemoveLiquidity(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsRemoveLiquidity(log.clone()));
            }
            let DexEvent::MeteoraPoolsRemoveLiquidity(e) = result else { panic!("liquidity") };
            assert_eq!(e.ix_name, "remove_balance_liquidity");
            assert_eq!(e.pool_token_amount, ix.pool_token_amount);
            assert_eq!(e.minimum_a_token_out, ix.minimum_a_token_out);
            assert_eq!(e.minimum_b_token_out, ix.minimum_b_token_out);
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.lp_mint, ix.lp_mint);
            assert_eq!(e.user_pool_lp, ix.user_pool_lp);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.user_a_token, ix.user_a_token);
            assert_eq!(e.user_b_token, ix.user_b_token);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.lp_unmint_amount, log.lp_unmint_amount);
            assert_eq!(e.token_a_out_amount, log.token_a_out_amount);
            assert_eq!(e.token_b_out_amount, log.token_b_out_amount);
            assert_eq!(e.token_program, log.token_program);
        }
    }
}

#[cfg(test)]
mod remaining_liquidity_merge_tests {
    use super::*;
    #[test]
    fn remove_liquidity_single_side_both_mergers_keep_zero_inputs_and_log_execution() {
        let ix = MeteoraPoolsRemoveLiquidityEvent {
            ix_name: "remove_liquidity_single_side".into(),
            pool_token_amount: 0,
            minimum_out_amount: u64::MAX,
            pool: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            user_pool_lp: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            user_destination_token: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsRemoveLiquidityEvent {
            pool_token_amount: 99,
            minimum_out_amount: 99,
            lp_unmint_amount: 123,
            token_a_out_amount: 124,
            token_b_out_amount: 125,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result = DexEvent::MeteoraPoolsRemoveLiquidity(if log_first {
                log.clone()
            } else {
                ix.clone()
            });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsRemoveLiquidity(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsRemoveLiquidity(log.clone()));
            }
            let DexEvent::MeteoraPoolsRemoveLiquidity(e) = result else { panic!("liquidity") };
            assert_eq!(e.ix_name, "remove_liquidity_single_side");
            assert_eq!(e.pool_token_amount, ix.pool_token_amount);
            assert_eq!(e.minimum_out_amount, ix.minimum_out_amount);
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.lp_mint, ix.lp_mint);
            assert_eq!(e.user_pool_lp, ix.user_pool_lp);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.user_destination_token, ix.user_destination_token);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.lp_unmint_amount, log.lp_unmint_amount);
            assert_eq!(e.token_a_out_amount, log.token_a_out_amount);
            assert_eq!(e.token_b_out_amount, log.token_b_out_amount);
            assert_eq!(e.token_program, log.token_program);
        }
    }
    #[test]
    fn bootstrap_liquidity_both_mergers_keep_zero_inputs_and_log_execution() {
        let ix = MeteoraPoolsBootstrapLiquidityEvent {
            ix_name: "bootstrap_liquidity".into(),
            token_a_in_amount: 0,
            token_b_in_amount: u64::MAX,
            pool: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            user_pool_lp: Pubkey::new_unique(),
            a_vault_lp: Pubkey::new_unique(),
            b_vault_lp: Pubkey::new_unique(),
            a_vault: Pubkey::new_unique(),
            b_vault: Pubkey::new_unique(),
            a_vault_lp_mint: Pubkey::new_unique(),
            b_vault_lp_mint: Pubkey::new_unique(),
            a_token_vault: Pubkey::new_unique(),
            b_token_vault: Pubkey::new_unique(),
            user_a_token: Pubkey::new_unique(),
            user_b_token: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            vault_program: Pubkey::new_unique(),
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsBootstrapLiquidityEvent {
            token_a_in_amount: 99,
            token_b_in_amount: 99,
            lp_mint_amount: 123,
            token_a_amount: 124,
            token_b_amount: 125,
            token_program: Pubkey::new_unique(),
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result = DexEvent::MeteoraPoolsBootstrapLiquidity(if log_first {
                log.clone()
            } else {
                ix.clone()
            });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsBootstrapLiquidity(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsBootstrapLiquidity(log.clone()));
            }
            let DexEvent::MeteoraPoolsBootstrapLiquidity(e) = result else { panic!("liquidity") };
            assert_eq!(e.ix_name, "bootstrap_liquidity");
            assert_eq!(e.token_a_in_amount, ix.token_a_in_amount);
            assert_eq!(e.token_b_in_amount, ix.token_b_in_amount);
            assert_eq!(e.pool, ix.pool);
            assert_eq!(e.lp_mint, ix.lp_mint);
            assert_eq!(e.user_pool_lp, ix.user_pool_lp);
            assert_eq!(e.a_vault_lp, ix.a_vault_lp);
            assert_eq!(e.b_vault_lp, ix.b_vault_lp);
            assert_eq!(e.a_vault, ix.a_vault);
            assert_eq!(e.b_vault, ix.b_vault);
            assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
            assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
            assert_eq!(e.a_token_vault, ix.a_token_vault);
            assert_eq!(e.b_token_vault, ix.b_token_vault);
            assert_eq!(e.user_a_token, ix.user_a_token);
            assert_eq!(e.user_b_token, ix.user_b_token);
            assert_eq!(e.user, ix.user);
            assert_eq!(e.vault_program, ix.vault_program);
            assert_eq!(e.lp_mint_amount, log.lp_mint_amount);
            assert_eq!(e.token_a_amount, log.token_a_amount);
            assert_eq!(e.token_b_amount, log.token_b_amount);
            assert_eq!(e.token_program, log.token_program);
        }
    }
}

#[cfg(test)]
mod pools_management_merge_tests {
    use super::*;
    #[test]
    fn creation_both_paths_keep_zero_inputs_optional_activation_and_accounts() {
        for activation_point in [None, Some(0), Some(u64::MAX)] {
            let ix = MeteoraPoolsPoolCreatedEvent {
                ix_name: "initialize_permissionless_constant_product_pool_with_config2".into(),
                token_a_in_amount: 0,
                token_b_in_amount: u64::MAX,
                activation_point,
                pool: Pubkey::new_unique(),
                config: Pubkey::new_unique(),
                lp_mint: Pubkey::new_unique(),
                token_a_mint: Pubkey::new_unique(),
                token_b_mint: Pubkey::new_unique(),
                a_vault: Pubkey::new_unique(),
                b_vault: Pubkey::new_unique(),
                a_token_vault: Pubkey::new_unique(),
                b_token_vault: Pubkey::new_unique(),
                a_vault_lp_mint: Pubkey::new_unique(),
                b_vault_lp_mint: Pubkey::new_unique(),
                a_vault_lp: Pubkey::new_unique(),
                b_vault_lp: Pubkey::new_unique(),
                payer_token_a: Pubkey::new_unique(),
                payer_token_b: Pubkey::new_unique(),
                payer_pool_lp: Pubkey::new_unique(),
                protocol_token_a_fee: Pubkey::new_unique(),
                protocol_token_b_fee: Pubkey::new_unique(),
                payer: Pubkey::new_unique(),
                rent: Pubkey::new_unique(),
                mint_metadata: Pubkey::new_unique(),
                metadata_program: Pubkey::new_unique(),
                vault_program: Pubkey::new_unique(),
                token_program: Pubkey::new_unique(),
                associated_token_program: Pubkey::new_unique(),
                system_program: Pubkey::new_unique(),
                ..Default::default()
            };
            let log = MeteoraPoolsPoolCreatedEvent {
                token_a_in_amount: 99,
                activation_point: Some(99),
                pool_type: 1,
                token_program: Pubkey::new_unique(),
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut result = DexEvent::MeteoraPoolsPoolCreated(if log_first {
                    log.clone()
                } else {
                    ix.clone()
                });
                if log_first {
                    merge_grpc_instruction_into_log(
                        &mut result,
                        DexEvent::MeteoraPoolsPoolCreated(ix.clone()),
                    );
                } else {
                    merge_events(&mut result, DexEvent::MeteoraPoolsPoolCreated(log.clone()));
                }
                let DexEvent::MeteoraPoolsPoolCreated(e) = result else { panic!("create") };
                assert_eq!(
                    (e.token_a_in_amount, e.token_b_in_amount, e.activation_point),
                    (0, u64::MAX, activation_point)
                );
                assert_eq!(e.pool_type, 1);
                assert_eq!(e.pool, ix.pool);
                assert_eq!(e.config, ix.config);
                assert_eq!(e.lp_mint, ix.lp_mint);
                assert_eq!(e.token_a_mint, ix.token_a_mint);
                assert_eq!(e.token_b_mint, ix.token_b_mint);
                assert_eq!(e.a_vault, ix.a_vault);
                assert_eq!(e.b_vault, ix.b_vault);
                assert_eq!(e.a_token_vault, ix.a_token_vault);
                assert_eq!(e.b_token_vault, ix.b_token_vault);
                assert_eq!(e.a_vault_lp_mint, ix.a_vault_lp_mint);
                assert_eq!(e.b_vault_lp_mint, ix.b_vault_lp_mint);
                assert_eq!(e.a_vault_lp, ix.a_vault_lp);
                assert_eq!(e.b_vault_lp, ix.b_vault_lp);
                assert_eq!(e.payer_token_a, ix.payer_token_a);
                assert_eq!(e.payer_token_b, ix.payer_token_b);
                assert_eq!(e.payer_pool_lp, ix.payer_pool_lp);
                assert_eq!(e.protocol_token_a_fee, ix.protocol_token_a_fee);
                assert_eq!(e.protocol_token_b_fee, ix.protocol_token_b_fee);
                assert_eq!(e.payer, ix.payer);
                assert_eq!(e.rent, ix.rent);
                assert_eq!(e.mint_metadata, ix.mint_metadata);
                assert_eq!(e.metadata_program, ix.metadata_program);
                assert_eq!(e.vault_program, ix.vault_program);
                assert_eq!(e.associated_token_program, ix.associated_token_program);
                assert_eq!(e.system_program, ix.system_program);
                assert_eq!(e.token_program, log.token_program);
            }
        }
    }
    #[test]
    fn fees_both_paths_keep_partner_parameter_and_log_fee_values() {
        let ix = MeteoraPoolsSetPoolFeesEvent {
            ix_name: "set_pool_fees".into(),
            new_partner_fee_numerator: 0,
            fee_operator: Pubkey::new_unique(),
            pool: Pubkey::new_unique(),
            ..Default::default()
        };
        let log = MeteoraPoolsSetPoolFeesEvent {
            new_partner_fee_numerator: 99,
            trade_fee_numerator: 1,
            trade_fee_denominator: 2,
            protocol_trade_fee_numerator: 3,
            protocol_trade_fee_denominator: 4,
            owner_trade_fee_numerator: 3,
            owner_trade_fee_denominator: 4,
            ..Default::default()
        };
        for log_first in [false, true] {
            let mut result =
                DexEvent::MeteoraPoolsSetPoolFees(if log_first { log.clone() } else { ix.clone() });
            if log_first {
                merge_grpc_instruction_into_log(
                    &mut result,
                    DexEvent::MeteoraPoolsSetPoolFees(ix.clone()),
                );
            } else {
                merge_events(&mut result, DexEvent::MeteoraPoolsSetPoolFees(log.clone()));
            }
            let DexEvent::MeteoraPoolsSetPoolFees(e) = result else { panic!("fees") };
            assert_eq!(e.new_partner_fee_numerator, 0);
            assert_eq!((e.pool, e.fee_operator), (ix.pool, ix.fee_operator));
            assert_eq!(
                (
                    e.trade_fee_numerator,
                    e.trade_fee_denominator,
                    e.protocol_trade_fee_numerator,
                    e.protocol_trade_fee_denominator
                ),
                (1, 2, 3, 4)
            );
        }
    }
}

#[cfg(test)]
mod remaining_creation_merge_tests {
    use super::*;
    #[test]
    fn both_paths_preserve_curve_custom_parameters_and_clear_absent_options() {
        for populated in [false, true] {
            let ix = MeteoraPoolsPoolCreatedEvent {
                ix_name: "initialize_customizable_permissionless_constant_product_pool".into(),
                stable_curve: populated
                    .then(|| MeteoraPoolsStableCurveParams { amp: u64::MAX, ..Default::default() }),
                trade_fee_bps: populated.then_some(0),
                customizable_params: populated.then(|| MeteoraPoolsCustomizableParams {
                    trade_fee_numerator: u32::MAX,
                    padding: vec![255; 90],
                    ..Default::default()
                }),
                admin: Pubkey::new_unique(),
                admin_token_a: Pubkey::new_unique(),
                admin_token_b: Pubkey::new_unique(),
                admin_pool_lp: Pubkey::new_unique(),
                fee_owner: Pubkey::new_unique(),
                ..Default::default()
            };
            let log = MeteoraPoolsPoolCreatedEvent {
                pool_type: 1,
                stable_curve: Some(MeteoraPoolsStableCurveParams::default()),
                trade_fee_bps: Some(99),
                customizable_params: Some(MeteoraPoolsCustomizableParams::default()),
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut merged = DexEvent::MeteoraPoolsPoolCreated(if log_first {
                    log.clone()
                } else {
                    ix.clone()
                });
                if log_first {
                    merge_grpc_instruction_into_log(
                        &mut merged,
                        DexEvent::MeteoraPoolsPoolCreated(ix.clone()),
                    );
                } else {
                    merge_events(&mut merged, DexEvent::MeteoraPoolsPoolCreated(log.clone()));
                }
                let DexEvent::MeteoraPoolsPoolCreated(e) = merged else { panic!("create") };
                assert_eq!(e.stable_curve, ix.stable_curve);
                assert_eq!(e.trade_fee_bps, ix.trade_fee_bps);
                assert_eq!(e.customizable_params, ix.customizable_params);
                assert_eq!(
                    (e.admin, e.admin_token_a, e.admin_token_b, e.admin_pool_lp, e.fee_owner),
                    (ix.admin, ix.admin_token_a, ix.admin_token_b, ix.admin_pool_lp, ix.fee_owner)
                );
                assert_eq!(e.pool_type, 1);
            }
        }
    }
}

#[cfg(test)]
mod amm_swap_parameter_merge_tests {
    use super::*;
    #[test]
    fn both_paths_preserve_zero_parameters_execution_and_account_context() {
        for name in ["swap_base_in", "swap_base_out", "swap_base_in_v2", "swap_base_out_v2"] {
            let input = name.starts_with("swap_base_in");
            let ix = RaydiumAmmV4SwapEvent {
                ix_name: name.into(),
                instruction_amount_in: 0,
                instruction_amount_out: 0,
                minimum_amount_out: if input { u64::MAX } else { 0 },
                max_amount_in: if input { 0 } else { u64::MAX },
                user_source_owner: Pubkey::new_unique(),
                user_source_token_account: Pubkey::new_unique(),
                user_destination_token_account: Pubkey::new_unique(),
                amm_target_orders: Some(Pubkey::new_unique()),
                ..Default::default()
            };
            let log = RaydiumAmmV4SwapEvent {
                amount_in: 123,
                amount_out: 456,
                instruction_amount_in: 99,
                instruction_amount_out: 99,
                minimum_amount_out: 99,
                max_amount_in: 99,
                token_program: Pubkey::new_unique(),
                ..Default::default()
            };
            for log_first in [false, true] {
                let mut merged =
                    DexEvent::RaydiumAmmV4Swap(if log_first { log.clone() } else { ix.clone() });
                if log_first {
                    merge_grpc_instruction_into_log(
                        &mut merged,
                        DexEvent::RaydiumAmmV4Swap(ix.clone()),
                    );
                } else {
                    merge_events(&mut merged, DexEvent::RaydiumAmmV4Swap(log.clone()));
                }
                let DexEvent::RaydiumAmmV4Swap(e) = merged else { panic!("swap") };
                assert_eq!(e.ix_name, name);
                assert_eq!((e.amount_in, e.amount_out), (123, 456));
                assert_eq!((e.instruction_amount_in, e.instruction_amount_out), (0, 0));
                assert_eq!(
                    (e.minimum_amount_out, e.max_amount_in),
                    (ix.minimum_amount_out, ix.max_amount_in)
                );
                assert_eq!(
                    (
                        e.user_source_owner,
                        e.user_source_token_account,
                        e.user_destination_token_account
                    ),
                    (
                        ix.user_source_owner,
                        ix.user_source_token_account,
                        ix.user_destination_token_account
                    )
                );
                assert_eq!(e.amm_target_orders, ix.amm_target_orders);
                assert_eq!(e.token_program, log.token_program);
            }
        }
    }
}

// Instruction/CPI events sharing an outer index may still be distinct trades.
fn pump_trade_identity(event: &DexEvent) -> Option<(u8, Pubkey, Pubkey, Option<bool>)> {
    use DexEvent::*;
    match event {
        PumpFunTrade(e) | PumpFunBuy(e) | PumpFunSell(e) | PumpFunBuyExactSolIn(e) => Some((
            0,
            e.mint,
            e.user,
            (e.mint != Pubkey::default()
                || !e.ix_name.is_empty()
                || e.sol_amount != 0
                || e.token_amount != 0)
                .then_some(e.is_buy),
        )),
        PumpSwapBuy(e) => Some((1, e.pool, e.user, Some(true))),
        PumpSwapSell(e) => Some((1, e.pool, e.user, Some(false))),
        _ => None,
    }
}

fn pump_trade_identity_conflicts(base: &DexEvent, inner: &DexEvent) -> bool {
    let (Some((kind, venue, user, buy)), Some((other, other_venue, other_user, other_buy))) =
        (pump_trade_identity(base), pump_trade_identity(inner))
    else {
        return false;
    };
    kind == other
        && ((buy.is_some() && other_buy.is_some() && buy != other_buy)
            || (venue != Pubkey::default()
                && other_venue != Pubkey::default()
                && venue != other_venue)
            || (user != Pubkey::default() && other_user != Pubkey::default() && user != other_user))
}
