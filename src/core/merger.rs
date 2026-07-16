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
        (RaydiumClmmSwap(b), RaydiumClmmSwap(i)) => merge_generic(b, i),
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
        (RaydiumCpmmSwap(b), RaydiumCpmmSwap(i)) => merge_generic(b, i),
        (RaydiumCpmmDeposit(b), RaydiumCpmmDeposit(i)) => merge_generic(b, i),
        (RaydiumCpmmWithdraw(b), RaydiumCpmmWithdraw(i)) => merge_generic(b, i),
        (RaydiumCpmmInitialize(b), RaydiumCpmmInitialize(i)) => merge_generic(b, i),

        // ========== Raydium AMM V4 系列 ==========
        (RaydiumAmmV4Swap(b), RaydiumAmmV4Swap(i)) => merge_generic(b, i),
        (RaydiumAmmV4Deposit(b), RaydiumAmmV4Deposit(i)) => merge_generic(b, i),
        (RaydiumAmmV4Withdraw(b), RaydiumAmmV4Withdraw(i)) => merge_generic(b, i),
        (RaydiumAmmV4Initialize2(b), RaydiumAmmV4Initialize2(i)) => merge_generic(b, i),
        (RaydiumAmmV4WithdrawPnl(b), RaydiumAmmV4WithdrawPnl(i)) => merge_generic(b, i),

        // ========== Orca Whirlpool 系列 ==========
        (OrcaWhirlpoolSwap(b), OrcaWhirlpoolSwap(i)) => merge_generic(b, i),
        (OrcaWhirlpoolLiquidityIncreased(b), OrcaWhirlpoolLiquidityIncreased(i)) => {
            merge_generic(b, i)
        }
        (OrcaWhirlpoolLiquidityDecreased(b), OrcaWhirlpoolLiquidityDecreased(i)) => {
            merge_generic(b, i)
        }
        (OrcaWhirlpoolPoolInitialized(b), OrcaWhirlpoolPoolInitialized(i)) => merge_generic(b, i),

        // ========== Meteora Pools (AMM) 系列 ==========
        (MeteoraPoolsSwap(b), MeteoraPoolsSwap(i)) => merge_generic(b, i),
        (MeteoraPoolsAddLiquidity(b), MeteoraPoolsAddLiquidity(i)) => merge_generic(b, i),
        (MeteoraPoolsRemoveLiquidity(b), MeteoraPoolsRemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraPoolsBootstrapLiquidity(b), MeteoraPoolsBootstrapLiquidity(i)) => {
            merge_generic(b, i)
        }
        (MeteoraPoolsPoolCreated(b), MeteoraPoolsPoolCreated(i)) => merge_generic(b, i),
        (MeteoraPoolsSetPoolFees(b), MeteoraPoolsSetPoolFees(i)) => merge_generic(b, i),

        // ========== Meteora DAMM V2 系列 ==========
        (MeteoraDammV2Swap(b), MeteoraDammV2Swap(i)) => merge_generic(b, i),
        (MeteoraDammV2AddLiquidity(b), MeteoraDammV2AddLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2RemoveLiquidity(b), MeteoraDammV2RemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2InitializePool(b), MeteoraDammV2InitializePool(i)) => merge_generic(b, i),
        (MeteoraDammV2CreatePosition(b), MeteoraDammV2CreatePosition(i)) => merge_generic(b, i),
        (MeteoraDammV2ClosePosition(b), MeteoraDammV2ClosePosition(i)) => merge_generic(b, i),

        // ========== Meteora DLMM 系列 ==========
        (MeteoraDlmmSwap(b), MeteoraDlmmSwap(i)) => merge_generic(b, i),
        (MeteoraDlmmAddLiquidity(b), MeteoraDlmmAddLiquidity(i)) => merge_generic(b, i),
        (MeteoraDlmmRemoveLiquidity(b), MeteoraDlmmRemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraDlmmInitializePool(b), MeteoraDlmmInitializePool(i)) => merge_generic(b, i),
        (MeteoraDlmmInitializeBinArray(b), MeteoraDlmmInitializeBinArray(i)) => merge_generic(b, i),
        (MeteoraDlmmCreatePosition(b), MeteoraDlmmCreatePosition(i)) => merge_generic(b, i),
        (MeteoraDlmmClosePosition(b), MeteoraDlmmClosePosition(i)) => merge_generic(b, i),
        (MeteoraDlmmClaimFee(b), MeteoraDlmmClaimFee(i)) => merge_generic(b, i),

        // ========== RaydiumLaunchlab 系列 ==========
        (RaydiumLaunchlabTrade(b), RaydiumLaunchlabTrade(i)) => merge_generic(b, i),
        (RaydiumLaunchlabPoolCreate(b), RaydiumLaunchlabPoolCreate(i)) => merge_generic(b, i),
        (RaydiumLaunchlabMigrateAmm(b), RaydiumLaunchlabMigrateAmm(i)) => merge_generic(b, i),

        // 其他组合不需要合并（类型不匹配）
        _ => {}
    }
}

/// 通用合并函数 - 对于大多数事件，inner instruction 包含完整数据
///
/// 这个函数简单地用 inner 的数据覆盖 base，因为：
/// - Inner instruction 来自程序日志，包含完整的交易数据
/// - Instruction 主要提供账户上下文
/// - 对于大多数协议，inner instruction 的数据已经足够完整
#[inline(always)]
fn merge_generic<T>(base: &mut T, inner: T) {
    *base = inner;
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
    put_u64_if_nonzero(&mut log.quote_amount, ix.quote_amount);
    put_u64_if_nonzero(&mut log.virtual_quote_reserves, ix.virtual_quote_reserves);
    put_u64_if_nonzero(&mut log.real_quote_reserves, ix.real_quote_reserves);
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
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
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
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
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
    if log.ix_name.is_empty() && !ix.ix_name.is_empty() {
        log.ix_name = ix.ix_name;
    }
    log.is_mayhem_mode |= ix.is_mayhem_mode;
    log.is_cashback_enabled |= ix.is_cashback_enabled;
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
    fill_pk(&mut log.token_account_0, ix.token_account_0);
    fill_pk(&mut log.token_account_1, ix.token_account_1);
    fill_pk(&mut log.sender, ix.sender);
}

#[inline]
fn merge_raydium_amm_v4_swap_log_preferred(
    log: &mut RaydiumAmmV4SwapEvent,
    ix: RaydiumAmmV4SwapEvent,
) {
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
    fill_pk(&mut log.creator, ix.creator);
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

/// RaydiumLaunchlabTrade 当前无独立「仅 ix 账户」字段；保留占位以便与 dedup 对齐，日后扩展。
#[inline]
fn merge_raydium_launchlab_trade_log_preferred(
    _log: &mut RaydiumLaunchlabTradeEvent,
    _ix: RaydiumLaunchlabTradeEvent,
) {
}

#[inline]
fn merge_meteora_dlmm_swap_log_preferred(
    _log: &mut MeteoraDlmmSwapEvent,
    _ix: MeteoraDlmmSwapEvent,
) {
}

/// 将 **instruction 路径**解析结果合并进 **log 路径**事件：`log` 保留链上日志权威数值，
/// `ix` 仅填补 `log` 中为默认值的账户等字段。**不替换** `log` 外层枚举变体。
///
/// 已覆盖与 [`crate::grpc::log_instr_dedup`] 去重键一致的主要类型：PumpFun 全系、PumpSwap
///（Trade/Buy/Sell/CreatePool/加减流动性）、RaydiumLaunchlab（Trade/PoolCreate/Migrate）、Raydium CLMM/AMM V4 Swap、Meteora DLMM Swap。
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
    use super::*;
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

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
            ..Default::default()
        });

        // 合并
        merge_events(&mut base, inner);

        // 验证合并结果
        if let DexEvent::PumpFunTrade(trade) = base {
            assert_eq!(trade.sol_amount, 1000);
            assert_eq!(trade.token_amount, 2000);
            assert!(trade.is_buy);
            // 账户上下文保留
            assert_ne!(trade.bonding_curve, Pubkey::default());
            assert_ne!(trade.associated_bonding_curve, Pubkey::default());
        } else {
            panic!("Expected PumpFunTrade event");
        }
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
}
