use super::types::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;

// ====================== 统一的 DEX 事件枚举 ======================

/// 统一的 DEX 事件枚举 - 参考 sol-dex-shreds 的做法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DexEvent {
    // PumpFun 事件
    PumpFunCreate(PumpFunCreateTokenEvent),     // - 已对接
    PumpFunCreateV2(PumpFunCreateV2TokenEvent), // - 已对接 (CreateV2 / Mayhem)
    PumpFunTrade(PumpFunTradeEvent),            // - 已对接 (统一交易事件，包含所有交易类型)
    PumpFunBuy(PumpFunTradeEvent),              // - 已对接 (仅买入事件，用于过滤)
    PumpFunSell(PumpFunTradeEvent),             // - 已对接 (仅卖出事件，用于过滤)
    PumpFunBuyExactSolIn(PumpFunTradeEvent),    // - 已对接 (精确SOL买入事件，用于过滤)
    PumpFunMigrate(PumpFunMigrateEvent),        // - 已对接
    /// Pump fees：`CreateFeeSharingConfigEvent`（`pfeeUx...`，见 `idls/pump_fees.json`）
    PumpFeesCreateFeeSharingConfig(PumpFeesCreateFeeSharingConfigEvent),
    PumpFeesInitializeFeeConfig(PumpFeesInitializeFeeConfigEvent),
    PumpFeesResetFeeSharingConfig(PumpFeesResetFeeSharingConfigEvent),
    PumpFeesRevokeFeeSharingAuthority(PumpFeesRevokeFeeSharingAuthorityEvent),
    PumpFeesTransferFeeSharingAuthority(PumpFeesTransferFeeSharingAuthorityEvent),
    PumpFeesUpdateAdmin(PumpFeesUpdateAdminEvent),
    PumpFeesUpdateFeeConfig(PumpFeesUpdateFeeConfigEvent),
    PumpFeesUpdateFeeShares(PumpFeesUpdateFeeSharesEvent),
    PumpFeesUpsertFeeTiers(PumpFeesUpsertFeeTiersEvent),
    /// Pump.fun：曲线 creator 迁移（`migrateBondingCurveCreatorEvent`）
    PumpFunMigrateBondingCurveCreator(PumpFunMigrateBondingCurveCreatorEvent),
    PumpFunGlobalAccount(PumpFunGlobalAccountEvent), // - 已对接
    PumpFunBondingCurveAccount(PumpFunBondingCurveAccountEvent),
    PumpFunFeeConfigAccount(PumpFunFeeConfigAccountEvent),
    PumpFunSharingConfigAccount(PumpFunSharingConfigAccountEvent),
    PumpFunGlobalVolumeAccumulatorAccount(PumpFunGlobalVolumeAccumulatorAccountEvent),
    PumpFunUserVolumeAccumulatorAccount(PumpFunUserVolumeAccumulatorAccountEvent),
    PumpSwapTrade(PumpSwapTradeEvent), // - 已对接 (buy/sell/buy_exact_sol_in)
    PumpSwapBuy(PumpSwapBuyEvent),     // - 已对接 (legacy)
    PumpSwapSell(PumpSwapSellEvent),   // - 已对接 (legacy)
    PumpSwapCreatePool(PumpSwapCreatePoolEvent), // - 已对接
    PumpSwapLiquidityAdded(PumpSwapLiquidityAdded), // - 已对接
    PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved), // - 已对接

    // Meteora DAMM V2 事件
    MeteoraDammV2Swap(MeteoraDammV2SwapEvent), // - 已对接
    MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent), // - 已对接
    MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent), // - 已对接
    MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent), // - 已对接
    MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent), // - 已对接
    MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent), // - 已对接

    // Meteora DBC 事件
    MeteoraDbcSwap(MeteoraDbcSwapEvent),
    MeteoraDbcInitializePool(MeteoraDbcInitializePoolEvent),
    MeteoraDbcCurveComplete(MeteoraDbcCurveCompleteEvent),

    // RaydiumLaunchlab 事件
    RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent),
    RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent),
    RaydiumLaunchlabMigrateAmm(RaydiumLaunchlabMigrateAmmEvent),

    // Raydium CLMM 事件
    RaydiumClmmSwap(RaydiumClmmSwapEvent),
    RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent),
    RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent),
    RaydiumClmmOpenPositionWithTokenExtNft(RaydiumClmmOpenPositionWithTokenExtNftEvent),
    RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent),
    RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent),
    RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent),
    RaydiumClmmLiquidityChange(RaydiumClmmLiquidityChangeEvent),
    RaydiumClmmConfigChange(RaydiumClmmConfigChangeEvent),
    RaydiumClmmCreatePersonalPosition(RaydiumClmmCreatePersonalPositionEvent),
    RaydiumClmmLiquidityCalculate(RaydiumClmmLiquidityCalculateEvent),
    RaydiumClmmOpenLimitOrder(RaydiumClmmOpenLimitOrderEvent),
    RaydiumClmmIncreaseLimitOrder(RaydiumClmmIncreaseLimitOrderEvent),
    RaydiumClmmDecreaseLimitOrder(RaydiumClmmDecreaseLimitOrderEvent),
    RaydiumClmmSettleLimitOrder(RaydiumClmmSettleLimitOrderEvent),
    RaydiumClmmUpdateRewardInfos(RaydiumClmmUpdateRewardInfosEvent),
    RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent),
    RaydiumClmmAmmConfigAccount(Box<RaydiumClmmAmmConfigAccountEvent>),
    RaydiumClmmPoolStateAccount(Box<RaydiumClmmPoolStateAccountEvent>),
    RaydiumClmmTickArrayStateAccount(Box<RaydiumClmmTickArrayStateAccountEvent>),

    // Raydium CPMM 事件
    RaydiumCpmmSwap(RaydiumCpmmSwapEvent),
    RaydiumCpmmDeposit(RaydiumCpmmDepositEvent),
    RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent),
    RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent),
    RaydiumCpmmAmmConfigAccount(Box<RaydiumCpmmAmmConfigAccountEvent>),
    RaydiumCpmmPoolStateAccount(Box<RaydiumCpmmPoolStateAccountEvent>),

    // Raydium AMM V4 事件
    RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent),
    RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent),
    RaydiumAmmV4Initialize2(RaydiumAmmV4Initialize2Event),
    RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent),
    RaydiumAmmV4WithdrawPnl(RaydiumAmmV4WithdrawPnlEvent),

    // Orca Whirlpool 事件
    OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent),
    OrcaWhirlpoolLiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent),
    OrcaWhirlpoolLiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent),
    OrcaWhirlpoolPoolInitialized(OrcaWhirlpoolPoolInitializedEvent),
    OrcaWhirlpoolAccount(Box<OrcaWhirlpoolAccountEvent>),
    OrcaPositionAccount(Box<OrcaPositionAccountEvent>),
    OrcaTickArrayAccount(Box<OrcaTickArrayAccountEvent>),
    OrcaFeeTierAccount(Box<OrcaFeeTierAccountEvent>),
    OrcaWhirlpoolsConfigAccount(Box<OrcaWhirlpoolsConfigAccountEvent>),

    // Meteora Pools 事件
    MeteoraPoolsSwap(MeteoraPoolsSwapEvent),
    MeteoraPoolsAddLiquidity(MeteoraPoolsAddLiquidityEvent),
    MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent),
    MeteoraPoolsBootstrapLiquidity(MeteoraPoolsBootstrapLiquidityEvent),
    MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent),
    MeteoraPoolsSetPoolFees(MeteoraPoolsSetPoolFeesEvent),

    // Meteora DLMM 事件
    MeteoraDlmmSwap(MeteoraDlmmSwapEvent),
    MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent),
    MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent),
    MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent),
    MeteoraDlmmInitializeBinArray(MeteoraDlmmInitializeBinArrayEvent),
    MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent),
    MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent),
    MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent),

    // 账户事件
    TokenInfo(TokenInfoEvent),       // - 已对接
    TokenAccount(TokenAccountEvent), // - 已对接
    NonceAccount(NonceAccountEvent), // - 已对接
    PumpSwapGlobalConfigAccount(PumpSwapGlobalConfigAccountEvent), // - 已对接
    PumpSwapPoolAccount(PumpSwapPoolAccountEvent), // - 已对接

    // 区块元数据事件
    BlockMeta(BlockMetaEvent),

    // 错误事件
    Error(String),
}

// 静态默认 EventMetadata，用于 Error 事件
static DEFAULT_METADATA: Lazy<EventMetadata> = Lazy::new(|| EventMetadata {
    signature: Signature::from([0u8; 64]),
    slot: 0,
    tx_index: 0,
    block_time_us: 0,
    grpc_recv_us: 0,
    recent_blockhash: None,
});

impl DexEvent {
    /// 获取事件的元数据
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            // PumpFun 事件
            DexEvent::PumpFunCreate(e) => &e.metadata,
            DexEvent::PumpFunCreateV2(e) => &e.metadata,
            DexEvent::PumpFunTrade(e) => &e.metadata,
            DexEvent::PumpFunBuy(e) => &e.metadata,
            DexEvent::PumpFunSell(e) => &e.metadata,
            DexEvent::PumpFunBuyExactSolIn(e) => &e.metadata,
            DexEvent::PumpFunMigrate(e) => &e.metadata,
            DexEvent::PumpFeesCreateFeeSharingConfig(e) => &e.metadata,
            DexEvent::PumpFeesInitializeFeeConfig(e) => &e.metadata,
            DexEvent::PumpFeesResetFeeSharingConfig(e) => &e.metadata,
            DexEvent::PumpFeesRevokeFeeSharingAuthority(e) => &e.metadata,
            DexEvent::PumpFeesTransferFeeSharingAuthority(e) => &e.metadata,
            DexEvent::PumpFeesUpdateAdmin(e) => &e.metadata,
            DexEvent::PumpFeesUpdateFeeConfig(e) => &e.metadata,
            DexEvent::PumpFeesUpdateFeeShares(e) => &e.metadata,
            DexEvent::PumpFeesUpsertFeeTiers(e) => &e.metadata,
            DexEvent::PumpFunMigrateBondingCurveCreator(e) => &e.metadata,
            DexEvent::PumpFunGlobalAccount(e) => &e.metadata,
            DexEvent::PumpFunBondingCurveAccount(e) => &e.metadata,
            DexEvent::PumpFunFeeConfigAccount(e) => &e.metadata,
            DexEvent::PumpFunSharingConfigAccount(e) => &e.metadata,
            DexEvent::PumpFunGlobalVolumeAccumulatorAccount(e) => &e.metadata,
            DexEvent::PumpFunUserVolumeAccumulatorAccount(e) => &e.metadata,

            // PumpSwap 事件
            DexEvent::PumpSwapTrade(e) => &e.metadata,
            DexEvent::PumpSwapBuy(e) => &e.metadata,
            DexEvent::PumpSwapSell(e) => &e.metadata,
            DexEvent::PumpSwapCreatePool(e) => &e.metadata,
            DexEvent::PumpSwapLiquidityAdded(e) => &e.metadata,
            DexEvent::PumpSwapLiquidityRemoved(e) => &e.metadata,

            // Meteora DAMM V2 事件
            DexEvent::MeteoraDammV2Swap(e) => &e.metadata,
            DexEvent::MeteoraDammV2CreatePosition(e) => &e.metadata,
            DexEvent::MeteoraDammV2ClosePosition(e) => &e.metadata,
            DexEvent::MeteoraDammV2AddLiquidity(e) => &e.metadata,
            DexEvent::MeteoraDammV2RemoveLiquidity(e) => &e.metadata,
            DexEvent::MeteoraDammV2InitializePool(e) => &e.metadata,
            DexEvent::MeteoraDbcSwap(e) => &e.metadata,
            DexEvent::MeteoraDbcInitializePool(e) => &e.metadata,
            DexEvent::MeteoraDbcCurveComplete(e) => &e.metadata,

            // RaydiumLaunchlab 事件
            DexEvent::RaydiumLaunchlabTrade(e) => &e.metadata,
            DexEvent::RaydiumLaunchlabPoolCreate(e) => &e.metadata,
            DexEvent::RaydiumLaunchlabMigrateAmm(e) => &e.metadata,

            // Raydium CLMM 事件
            DexEvent::RaydiumClmmSwap(e) => &e.metadata,
            DexEvent::RaydiumClmmCreatePool(e) => &e.metadata,
            DexEvent::RaydiumClmmOpenPosition(e) => &e.metadata,
            DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => &e.metadata,
            DexEvent::RaydiumClmmClosePosition(e) => &e.metadata,
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => &e.metadata,
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => &e.metadata,
            DexEvent::RaydiumClmmLiquidityChange(e) => &e.metadata,
            DexEvent::RaydiumClmmConfigChange(e) => &e.metadata,
            DexEvent::RaydiumClmmCreatePersonalPosition(e) => &e.metadata,
            DexEvent::RaydiumClmmLiquidityCalculate(e) => &e.metadata,
            DexEvent::RaydiumClmmOpenLimitOrder(e) => &e.metadata,
            DexEvent::RaydiumClmmIncreaseLimitOrder(e) => &e.metadata,
            DexEvent::RaydiumClmmDecreaseLimitOrder(e) => &e.metadata,
            DexEvent::RaydiumClmmSettleLimitOrder(e) => &e.metadata,
            DexEvent::RaydiumClmmUpdateRewardInfos(e) => &e.metadata,
            DexEvent::RaydiumClmmCollectFee(e) => &e.metadata,
            DexEvent::RaydiumClmmAmmConfigAccount(e) => &e.metadata,
            DexEvent::RaydiumClmmPoolStateAccount(e) => &e.metadata,
            DexEvent::RaydiumClmmTickArrayStateAccount(e) => &e.metadata,

            // Raydium CPMM 事件
            DexEvent::RaydiumCpmmSwap(e) => &e.metadata,
            DexEvent::RaydiumCpmmDeposit(e) => &e.metadata,
            DexEvent::RaydiumCpmmWithdraw(e) => &e.metadata,
            DexEvent::RaydiumCpmmInitialize(e) => &e.metadata,
            DexEvent::RaydiumCpmmAmmConfigAccount(e) => &e.metadata,
            DexEvent::RaydiumCpmmPoolStateAccount(e) => &e.metadata,

            // Raydium AMM V4 事件
            DexEvent::RaydiumAmmV4Swap(e) => &e.metadata,
            DexEvent::RaydiumAmmV4Deposit(e) => &e.metadata,
            DexEvent::RaydiumAmmV4Initialize2(e) => &e.metadata,
            DexEvent::RaydiumAmmV4Withdraw(e) => &e.metadata,
            DexEvent::RaydiumAmmV4WithdrawPnl(e) => &e.metadata,

            // Orca Whirlpool 事件
            DexEvent::OrcaWhirlpoolSwap(e) => &e.metadata,
            DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => &e.metadata,
            DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => &e.metadata,
            DexEvent::OrcaWhirlpoolPoolInitialized(e) => &e.metadata,
            DexEvent::OrcaWhirlpoolAccount(e) => &e.metadata,
            DexEvent::OrcaPositionAccount(e) => &e.metadata,
            DexEvent::OrcaTickArrayAccount(e) => &e.metadata,
            DexEvent::OrcaFeeTierAccount(e) => &e.metadata,
            DexEvent::OrcaWhirlpoolsConfigAccount(e) => &e.metadata,

            // Meteora Pools 事件
            DexEvent::MeteoraPoolsSwap(e) => &e.metadata,
            DexEvent::MeteoraPoolsAddLiquidity(e) => &e.metadata,
            DexEvent::MeteoraPoolsRemoveLiquidity(e) => &e.metadata,
            DexEvent::MeteoraPoolsBootstrapLiquidity(e) => &e.metadata,
            DexEvent::MeteoraPoolsPoolCreated(e) => &e.metadata,
            DexEvent::MeteoraPoolsSetPoolFees(e) => &e.metadata,

            // Meteora DLMM 事件
            DexEvent::MeteoraDlmmSwap(e) => &e.metadata,
            DexEvent::MeteoraDlmmAddLiquidity(e) => &e.metadata,
            DexEvent::MeteoraDlmmRemoveLiquidity(e) => &e.metadata,
            DexEvent::MeteoraDlmmInitializePool(e) => &e.metadata,
            DexEvent::MeteoraDlmmInitializeBinArray(e) => &e.metadata,
            DexEvent::MeteoraDlmmCreatePosition(e) => &e.metadata,
            DexEvent::MeteoraDlmmClosePosition(e) => &e.metadata,
            DexEvent::MeteoraDlmmClaimFee(e) => &e.metadata,

            // 账户事件
            DexEvent::TokenInfo(e) => &e.metadata,
            DexEvent::TokenAccount(e) => &e.metadata,
            DexEvent::NonceAccount(e) => &e.metadata,
            DexEvent::PumpSwapGlobalConfigAccount(e) => &e.metadata,
            DexEvent::PumpSwapPoolAccount(e) => &e.metadata,

            // 区块元数据事件
            DexEvent::BlockMeta(e) => &e.metadata,

            // 错误事件 - 返回默认元数据
            DexEvent::Error(_) => &DEFAULT_METADATA,
        }
    }

    /// Mutable metadata for filling shared fields (e.g. recent_blockhash). Returns None for Error variant.
    pub fn metadata_mut(&mut self) -> Option<&mut EventMetadata> {
        match self {
            DexEvent::PumpFunCreate(e) => Some(&mut e.metadata),
            DexEvent::PumpFunCreateV2(e) => Some(&mut e.metadata),
            DexEvent::PumpFunTrade(e) => Some(&mut e.metadata),
            DexEvent::PumpFunBuy(e) => Some(&mut e.metadata),
            DexEvent::PumpFunSell(e) => Some(&mut e.metadata),
            DexEvent::PumpFunBuyExactSolIn(e) => Some(&mut e.metadata),
            DexEvent::PumpFunMigrate(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesCreateFeeSharingConfig(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesInitializeFeeConfig(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesResetFeeSharingConfig(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesRevokeFeeSharingAuthority(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesTransferFeeSharingAuthority(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesUpdateAdmin(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesUpdateFeeConfig(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesUpdateFeeShares(e) => Some(&mut e.metadata),
            DexEvent::PumpFeesUpsertFeeTiers(e) => Some(&mut e.metadata),
            DexEvent::PumpFunMigrateBondingCurveCreator(e) => Some(&mut e.metadata),
            DexEvent::PumpFunGlobalAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpFunBondingCurveAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpFunFeeConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpFunSharingConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpFunGlobalVolumeAccumulatorAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpFunUserVolumeAccumulatorAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapTrade(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapBuy(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapSell(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapCreatePool(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapLiquidityAdded(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapLiquidityRemoved(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2Swap(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2CreatePosition(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2ClosePosition(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2AddLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2RemoveLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDammV2InitializePool(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDbcSwap(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDbcInitializePool(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDbcCurveComplete(e) => Some(&mut e.metadata),
            DexEvent::RaydiumLaunchlabTrade(e) => Some(&mut e.metadata),
            DexEvent::RaydiumLaunchlabPoolCreate(e) => Some(&mut e.metadata),
            DexEvent::RaydiumLaunchlabMigrateAmm(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmSwap(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmCreatePool(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmOpenPosition(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmClosePosition(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmIncreaseLiquidity(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmDecreaseLiquidity(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmLiquidityChange(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmConfigChange(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmCreatePersonalPosition(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmLiquidityCalculate(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmOpenLimitOrder(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmIncreaseLimitOrder(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmDecreaseLimitOrder(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmSettleLimitOrder(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmUpdateRewardInfos(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmCollectFee(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmAmmConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmPoolStateAccount(e) => Some(&mut e.metadata),
            DexEvent::RaydiumClmmTickArrayStateAccount(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmSwap(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmDeposit(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmWithdraw(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmInitialize(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmAmmConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::RaydiumCpmmPoolStateAccount(e) => Some(&mut e.metadata),
            DexEvent::RaydiumAmmV4Swap(e) => Some(&mut e.metadata),
            DexEvent::RaydiumAmmV4Deposit(e) => Some(&mut e.metadata),
            DexEvent::RaydiumAmmV4Initialize2(e) => Some(&mut e.metadata),
            DexEvent::RaydiumAmmV4Withdraw(e) => Some(&mut e.metadata),
            DexEvent::RaydiumAmmV4WithdrawPnl(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolSwap(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolLiquidityIncreased(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolLiquidityDecreased(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolPoolInitialized(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolAccount(e) => Some(&mut e.metadata),
            DexEvent::OrcaPositionAccount(e) => Some(&mut e.metadata),
            DexEvent::OrcaTickArrayAccount(e) => Some(&mut e.metadata),
            DexEvent::OrcaFeeTierAccount(e) => Some(&mut e.metadata),
            DexEvent::OrcaWhirlpoolsConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsSwap(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsAddLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsRemoveLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsBootstrapLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsPoolCreated(e) => Some(&mut e.metadata),
            DexEvent::MeteoraPoolsSetPoolFees(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmSwap(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmAddLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmRemoveLiquidity(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmInitializePool(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmInitializeBinArray(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmCreatePosition(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmClosePosition(e) => Some(&mut e.metadata),
            DexEvent::MeteoraDlmmClaimFee(e) => Some(&mut e.metadata),
            DexEvent::TokenInfo(e) => Some(&mut e.metadata),
            DexEvent::TokenAccount(e) => Some(&mut e.metadata),
            DexEvent::NonceAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapGlobalConfigAccount(e) => Some(&mut e.metadata),
            DexEvent::PumpSwapPoolAccount(e) => Some(&mut e.metadata),
            DexEvent::BlockMeta(e) => Some(&mut e.metadata),
            DexEvent::Error(_) => None,
        }
    }
}
