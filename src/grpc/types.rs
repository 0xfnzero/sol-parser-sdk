use serde::{Deserialize, Serialize};
use yellowstone_grpc_proto::geyser::{
    subscribe_request_filter_accounts_filter::Filter as AccountsFilterOneof,
    subscribe_request_filter_accounts_filter_memcmp::Data as MemcmpDataOneof,
    SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
};

/// 事件输出顺序模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OrderMode {
    /// 无序模式：收到即输出，超低延迟 (10-20μs)
    #[default]
    Unordered,
    /// 有序模式：按 slot + tx_index 排序后输出
    /// 同一 slot 内的交易会等待收齐后按 tx_index 排序
    /// 延迟增加约 1-50ms（取决于 slot 内交易数量）
    Ordered,
    /// 流式有序模式：连续序列立即释放，低延迟 + 顺序保证
    /// 只要收到从 0 开始的连续 tx_index 序列，立即释放
    /// 延迟约 0.1-5ms，比 Ordered 低 5-50 倍
    StreamingOrdered,
    /// 微批次模式：极短时间窗口内收集事件，窗口结束后排序释放
    /// 窗口大小由 micro_batch_us 配置（默认 100μs）
    /// 延迟约 50-200μs，接近 Unordered 但保证顺序
    MicroBatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 是否启用性能监控
    pub enable_metrics: bool,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 请求超时时间（毫秒）
    pub request_timeout_ms: u64,
    /// 是否启用TLS
    pub enable_tls: bool,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_concurrent_streams: u32,
    pub keep_alive_interval_ms: u64,
    pub keep_alive_timeout_ms: u64,
    pub buffer_size: usize,
    /// 事件输出顺序模式
    pub order_mode: OrderMode,
    /// 有序模式下，slot 超时时间（毫秒）
    /// 超过此时间未收到新 slot 信号，强制输出当前缓冲的事件
    pub order_timeout_ms: u64,
    /// MicroBatch 模式下的时间窗口大小（微秒）
    /// 默认 100μs，可根据网络状况调整
    pub micro_batch_us: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            connection_timeout_ms: 8000,
            request_timeout_ms: 15000,
            enable_tls: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            max_concurrent_streams: 100,
            keep_alive_interval_ms: 30000,
            keep_alive_timeout_ms: 5000,
            buffer_size: 100_000,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 100,
            micro_batch_us: 100, // 100μs 默认窗口
        }
    }
}

impl ClientConfig {
    pub fn low_latency() -> Self {
        Self {
            enable_metrics: false,
            connection_timeout_ms: 5000,
            request_timeout_ms: 10000,
            enable_tls: true,
            max_retries: 1,
            retry_delay_ms: 100,
            max_concurrent_streams: 200,
            keep_alive_interval_ms: 10000,
            keep_alive_timeout_ms: 2000,
            buffer_size: 100_000,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 50,
            micro_batch_us: 50, // 50μs 更激进的窗口
        }
    }

    pub fn high_throughput() -> Self {
        Self {
            enable_metrics: true,
            connection_timeout_ms: 10000,
            request_timeout_ms: 30000,
            enable_tls: true,
            max_retries: 5,
            retry_delay_ms: 2000,
            max_concurrent_streams: 500,
            keep_alive_interval_ms: 60000,
            keep_alive_timeout_ms: 10000,
            buffer_size: 200_000,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 200,
            micro_batch_us: 200, // 200μs 高吞吐模式
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionFilter {
    pub account_include: Vec<String>,
    pub account_exclude: Vec<String>,
    pub account_required: Vec<String>,
}

impl TransactionFilter {
    pub fn new() -> Self {
        Self {
            account_include: Vec::new(),
            account_exclude: Vec::new(),
            account_required: Vec::new(),
        }
    }

    pub fn include_account(mut self, account: impl Into<String>) -> Self {
        self.account_include.push(account.into());
        self
    }

    pub fn exclude_account(mut self, account: impl Into<String>) -> Self {
        self.account_exclude.push(account.into());
        self
    }

    pub fn require_account(mut self, account: impl Into<String>) -> Self {
        self.account_required.push(account.into());
        self
    }

    /// 从程序ID列表创建过滤器
    pub fn from_program_ids(program_ids: Vec<String>) -> Self {
        Self {
            account_include: program_ids,
            account_exclude: Vec::new(),
            account_required: Vec::new(),
        }
    }
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AccountFilter {
    pub account: Vec<String>,
    pub owner: Vec<String>,
    pub filters: Vec<SubscribeRequestFilterAccountsFilter>,
}

impl AccountFilter {
    pub fn new() -> Self {
        Self { account: Vec::new(), owner: Vec::new(), filters: Vec::new() }
    }

    pub fn add_account(mut self, account: impl Into<String>) -> Self {
        self.account.push(account.into());
        self
    }

    pub fn add_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner.push(owner.into());
        self
    }

    pub fn add_filter(mut self, filter: SubscribeRequestFilterAccountsFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// 从程序ID列表创建所有者过滤器
    pub fn from_program_owners(program_ids: Vec<String>) -> Self {
        Self { account: Vec::new(), owner: program_ids, filters: Vec::new() }
    }
}

impl Default for AccountFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a memcmp account filter for use in `AccountFilter::filters`.
/// ATA accounts have mint at offset 0; PumpSwap pool accounts often use offset 32 for mint/pubkey.
#[inline]
pub fn account_filter_memcmp(offset: u64, bytes: Vec<u8>) -> SubscribeRequestFilterAccountsFilter {
    SubscribeRequestFilterAccountsFilter {
        filter: Some(AccountsFilterOneof::Memcmp(SubscribeRequestFilterAccountsFilterMemcmp {
            offset,
            data: Some(MemcmpDataOneof::Bytes(bytes)),
        })),
    }
}

#[derive(Debug, Clone)]
pub struct AccountFilterData {
    pub memcmp: Option<AccountFilterMemcmp>,
    pub datasize: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AccountFilterMemcmp {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    PumpFun,
    PumpSwap,
    PumpFees,
    RaydiumLaunchlab,
    RaydiumCpmm,
    RaydiumClmm,
    RaydiumAmmV4,
    OrcaWhirlpool,
    MeteoraPools,
    MeteoraDammV2,
    MeteoraDlmm,
    MeteoraDbc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventType {
    // Block events
    BlockMeta,

    // RaydiumLaunchlab events
    RaydiumLaunchlabTrade,
    RaydiumLaunchlabPoolCreate,
    RaydiumLaunchlabMigrateAmm,

    // PumpFun events
    PumpFunTrade,         // All trade events (backward compatible)
    PumpFunBuy,           // Buy events only (filter by ix_name)
    PumpFunSell,          // Sell events only (filter by ix_name)
    PumpFunBuyExactSolIn, // BuyExactSolIn events only (filter by ix_name)
    PumpFunCreate,
    PumpFunCreateV2, // SPL-22 / Mayhem create
    PumpFunComplete,
    PumpFunMigrate,
    /// Pump fees（`pfeeUx...`，`idls/pump_fees.json` Program data events）
    PumpFeesCreateFeeSharingConfig,
    PumpFeesInitializeFeeConfig,
    PumpFeesResetFeeSharingConfig,
    PumpFeesRevokeFeeSharingAuthority,
    PumpFeesTransferFeeSharingAuthority,
    PumpFeesUpdateAdmin,
    PumpFeesUpdateFeeConfig,
    PumpFeesUpdateFeeShares,
    PumpFeesUpsertFeeTiers,
    /// Pump.fun：`migrateBondingCurveCreatorEvent`
    PumpFunMigrateBondingCurveCreator,

    // PumpSwap events
    PumpSwapTrade,
    PumpSwapBuy,
    PumpSwapSell,
    PumpSwapCreatePool,
    PumpSwapLiquidityAdded,
    PumpSwapLiquidityRemoved,
    // PumpSwapPoolUpdated,
    // PumpSwapFeesClaimed,

    // Raydium CPMM events
    RaydiumCpmmSwap,
    RaydiumCpmmDeposit,
    RaydiumCpmmWithdraw,
    RaydiumCpmmInitialize,

    // Raydium CLMM events
    RaydiumClmmSwap,
    RaydiumClmmCreatePool,
    RaydiumClmmOpenPosition,
    RaydiumClmmClosePosition,
    RaydiumClmmIncreaseLiquidity,
    RaydiumClmmDecreaseLiquidity,
    RaydiumClmmLiquidityChange,
    RaydiumClmmConfigChange,
    RaydiumClmmCreatePersonalPosition,
    RaydiumClmmLiquidityCalculate,
    RaydiumClmmOpenLimitOrder,
    RaydiumClmmIncreaseLimitOrder,
    RaydiumClmmDecreaseLimitOrder,
    RaydiumClmmSettleLimitOrder,
    RaydiumClmmUpdateRewardInfos,
    RaydiumClmmOpenPositionWithTokenExtNft,
    RaydiumClmmCollectFee,

    // Raydium AMM V4 events
    RaydiumAmmV4Swap,
    RaydiumAmmV4Deposit,
    RaydiumAmmV4Withdraw,
    RaydiumAmmV4Initialize2,
    RaydiumAmmV4WithdrawPnl,

    // Orca Whirlpool events
    OrcaWhirlpoolSwap,
    OrcaWhirlpoolLiquidityIncreased,
    OrcaWhirlpoolLiquidityDecreased,
    OrcaWhirlpoolPoolInitialized,

    // Meteora events
    MeteoraPoolsSwap,
    MeteoraPoolsAddLiquidity,
    MeteoraPoolsRemoveLiquidity,
    MeteoraPoolsBootstrapLiquidity,
    MeteoraPoolsPoolCreated,
    MeteoraPoolsSetPoolFees,

    // Meteora DAMM V2 events
    MeteoraDammV2Swap,
    MeteoraDammV2AddLiquidity,
    MeteoraDammV2RemoveLiquidity,
    MeteoraDammV2InitializePool,
    MeteoraDammV2CreatePosition,
    MeteoraDammV2ClosePosition,
    // MeteoraDammV2ClaimPositionFee,
    // MeteoraDammV2InitializeReward,
    // MeteoraDammV2FundReward,
    // MeteoraDammV2ClaimReward,

    // Meteora DBC events
    MeteoraDbcSwap,
    MeteoraDbcInitializePool,
    MeteoraDbcCurveComplete,

    // Meteora DLMM events
    MeteoraDlmmSwap,
    MeteoraDlmmAddLiquidity,
    MeteoraDlmmRemoveLiquidity,
    MeteoraDlmmInitializePool,
    MeteoraDlmmInitializeBinArray,
    MeteoraDlmmCreatePosition,
    MeteoraDlmmClosePosition,
    MeteoraDlmmClaimFee,

    // Account events
    TokenAccount,
    TokenInfo,
    NonceAccount,
    AccountPumpFunGlobal,
    AccountPumpFunBondingCurve,
    AccountPumpFunFeeConfig,
    AccountPumpFunSharingConfig,
    AccountPumpFunGlobalVolumeAccumulator,
    AccountPumpFunUserVolumeAccumulator,

    AccountPumpSwapGlobalConfig,
    AccountPumpSwapPool,
    AccountRaydiumClmmAmmConfig,
    AccountRaydiumClmmPoolState,
    AccountRaydiumClmmTickArrayState,
    AccountRaydiumCpmmAmmConfig,
    AccountRaydiumCpmmPoolState,
    AccountOrcaWhirlpool,
    AccountOrcaPosition,
    AccountOrcaTickArray,
    AccountOrcaFeeTier,
    AccountOrcaWhirlpoolsConfig,
}

#[derive(Debug, Clone)]
pub struct EventTypeFilter {
    pub include_only: Option<Vec<EventType>>,
    pub exclude_types: Option<Vec<EventType>>,
}

impl EventTypeFilter {
    pub fn include_only(types: Vec<EventType>) -> Self {
        Self { include_only: Some(types), exclude_types: None }
    }

    pub fn exclude_types(types: Vec<EventType>) -> Self {
        Self { include_only: None, exclude_types: Some(types) }
    }

    #[inline]
    fn includes_any(&self, event_types: &[EventType]) -> bool {
        event_types.iter().any(|event_type| self.should_include(*event_type))
    }

    pub fn should_include(&self, event_type: EventType) -> bool {
        if let Some(ref include_only) = self.include_only {
            // Direct match
            if include_only.contains(&event_type) {
                return true;
            }
            if matches!(
                event_type,
                EventType::PumpFunBuy | EventType::PumpFunSell | EventType::PumpFunBuyExactSolIn
            ) {
                if pumpfun_trade_filter_is_generic(include_only) {
                    return true;
                }
                if event_type == EventType::PumpFunBuyExactSolIn
                    && pumpfun_buy_filter_is_generic(include_only)
                {
                    return true;
                }
                return false;
            }
            if is_pumpfun_create_family(event_type) {
                return include_only.iter().any(|t| is_pumpfun_create_family(*t));
            }
            if matches!(event_type, EventType::PumpSwapBuy | EventType::PumpSwapSell) {
                return include_only.contains(&EventType::PumpSwapTrade);
            }
            return false;
        }

        if let Some(ref exclude_types) = self.exclude_types {
            if exclude_types.contains(&event_type) {
                return false;
            }
            if matches!(
                event_type,
                EventType::PumpFunBuy | EventType::PumpFunSell | EventType::PumpFunBuyExactSolIn
            ) && exclude_types.contains(&EventType::PumpFunTrade)
            {
                return false;
            }
            if event_type == EventType::PumpFunBuyExactSolIn
                && exclude_types.contains(&EventType::PumpFunBuy)
            {
                return false;
            }
            if is_pumpfun_create_family(event_type)
                && exclude_types.iter().any(|t| is_pumpfun_create_family(*t))
            {
                return false;
            }
            if matches!(event_type, EventType::PumpSwapBuy | EventType::PumpSwapSell)
                && exclude_types.contains(&EventType::PumpSwapTrade)
            {
                return false;
            }
            return true;
        }

        true
    }

    pub fn should_include_dex_event(&self, event: &crate::core::events::DexEvent) -> bool {
        let Some(event_type) = event_type_from_dex_event(event) else { return true };
        self.should_include(event_type)
    }

    #[inline]
    pub fn includes_block_meta(&self) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.contains(&EventType::BlockMeta);
        }
        false
    }

    #[inline]
    pub fn normalize_dex_event(
        &self,
        event: crate::core::events::DexEvent,
    ) -> crate::core::events::DexEvent {
        use crate::core::events::DexEvent;

        let Some(ref include_only) = self.include_only else { return event };
        if pumpfun_trade_filter_is_generic(include_only) {
            return match event {
                DexEvent::PumpFunBuy(t)
                | DexEvent::PumpFunSell(t)
                | DexEvent::PumpFunBuyExactSolIn(t) => DexEvent::PumpFunTrade(t),
                other => other,
            };
        }
        if pumpfun_buy_filter_is_generic(include_only) {
            return match event {
                DexEvent::PumpFunBuyExactSolIn(t) => DexEvent::PumpFunBuy(t),
                other => other,
            };
        }

        event
    }

    #[inline]
    pub fn includes_pumpfun(&self) -> bool {
        self.includes_any(&[
            EventType::PumpFunTrade,
            EventType::PumpFunBuy,
            EventType::PumpFunSell,
            EventType::PumpFunBuyExactSolIn,
            EventType::PumpFunCreate,
            EventType::PumpFunCreateV2,
            EventType::PumpFunComplete,
            EventType::PumpFunMigrate,
            EventType::PumpFunMigrateBondingCurveCreator,
        ])
    }

    #[inline]
    pub fn includes_meteora_damm_v2(&self) -> bool {
        self.includes_any(&[
            EventType::MeteoraDammV2Swap,
            EventType::MeteoraDammV2AddLiquidity,
            EventType::MeteoraDammV2CreatePosition,
            EventType::MeteoraDammV2ClosePosition,
            EventType::MeteoraDammV2InitializePool,
            EventType::MeteoraDammV2RemoveLiquidity,
        ])
    }

    #[inline]
    pub fn includes_pump_fees(&self) -> bool {
        self.includes_any(&[
            EventType::PumpFeesCreateFeeSharingConfig,
            EventType::PumpFeesInitializeFeeConfig,
            EventType::PumpFeesResetFeeSharingConfig,
            EventType::PumpFeesRevokeFeeSharingAuthority,
            EventType::PumpFeesTransferFeeSharingAuthority,
            EventType::PumpFeesUpdateAdmin,
            EventType::PumpFeesUpdateFeeConfig,
            EventType::PumpFeesUpdateFeeShares,
            EventType::PumpFeesUpsertFeeTiers,
        ])
    }

    /// Check if PumpSwap protocol events are included in the filter
    #[inline]
    pub fn includes_pumpswap(&self) -> bool {
        self.includes_any(&[
            EventType::PumpSwapTrade,
            EventType::PumpSwapBuy,
            EventType::PumpSwapSell,
            EventType::PumpSwapCreatePool,
            EventType::PumpSwapLiquidityAdded,
            EventType::PumpSwapLiquidityRemoved,
        ])
    }

    /// Check if Raydium LaunchLab events are included in the filter.
    #[inline]
    pub fn includes_raydium_launchlab(&self) -> bool {
        self.includes_any(&[
            EventType::RaydiumLaunchlabTrade,
            EventType::RaydiumLaunchlabPoolCreate,
            EventType::RaydiumLaunchlabMigrateAmm,
        ])
    }

    #[inline]
    pub fn includes_raydium_cpmm(&self) -> bool {
        self.includes_any(&[
            EventType::RaydiumCpmmSwap,
            EventType::RaydiumCpmmDeposit,
            EventType::RaydiumCpmmWithdraw,
            EventType::RaydiumCpmmInitialize,
        ])
    }

    #[inline]
    pub fn includes_raydium_clmm(&self) -> bool {
        self.includes_any(&[
            EventType::RaydiumClmmSwap,
            EventType::RaydiumClmmCreatePool,
            EventType::RaydiumClmmOpenPosition,
            EventType::RaydiumClmmClosePosition,
            EventType::RaydiumClmmIncreaseLiquidity,
            EventType::RaydiumClmmDecreaseLiquidity,
            EventType::RaydiumClmmLiquidityChange,
            EventType::RaydiumClmmConfigChange,
            EventType::RaydiumClmmCreatePersonalPosition,
            EventType::RaydiumClmmLiquidityCalculate,
            EventType::RaydiumClmmOpenLimitOrder,
            EventType::RaydiumClmmIncreaseLimitOrder,
            EventType::RaydiumClmmDecreaseLimitOrder,
            EventType::RaydiumClmmSettleLimitOrder,
            EventType::RaydiumClmmUpdateRewardInfos,
            EventType::RaydiumClmmOpenPositionWithTokenExtNft,
            EventType::RaydiumClmmCollectFee,
        ])
    }

    #[inline]
    pub fn includes_raydium_amm_v4(&self) -> bool {
        self.includes_any(&[
            EventType::RaydiumAmmV4Swap,
            EventType::RaydiumAmmV4Deposit,
            EventType::RaydiumAmmV4Withdraw,
            EventType::RaydiumAmmV4Initialize2,
            EventType::RaydiumAmmV4WithdrawPnl,
        ])
    }

    #[inline]
    pub fn includes_orca_whirlpool(&self) -> bool {
        self.includes_any(&[
            EventType::OrcaWhirlpoolSwap,
            EventType::OrcaWhirlpoolLiquidityIncreased,
            EventType::OrcaWhirlpoolLiquidityDecreased,
            EventType::OrcaWhirlpoolPoolInitialized,
        ])
    }

    #[inline]
    pub fn includes_meteora_pools(&self) -> bool {
        self.includes_any(&[
            EventType::MeteoraPoolsSwap,
            EventType::MeteoraPoolsAddLiquidity,
            EventType::MeteoraPoolsRemoveLiquidity,
            EventType::MeteoraPoolsBootstrapLiquidity,
            EventType::MeteoraPoolsPoolCreated,
            EventType::MeteoraPoolsSetPoolFees,
        ])
    }

    #[inline]
    pub fn includes_meteora_dlmm(&self) -> bool {
        self.includes_any(&[
            EventType::MeteoraDlmmSwap,
            EventType::MeteoraDlmmAddLiquidity,
            EventType::MeteoraDlmmRemoveLiquidity,
            EventType::MeteoraDlmmInitializePool,
            EventType::MeteoraDlmmInitializeBinArray,
            EventType::MeteoraDlmmCreatePosition,
            EventType::MeteoraDlmmClosePosition,
            EventType::MeteoraDlmmClaimFee,
        ])
    }

    #[inline]
    pub fn includes_meteora_dbc(&self) -> bool {
        self.includes_any(&[
            EventType::MeteoraDbcSwap,
            EventType::MeteoraDbcInitializePool,
            EventType::MeteoraDbcCurveComplete,
        ])
    }
}

#[inline]
fn pumpfun_trade_filter_is_generic(include_only: &[EventType]) -> bool {
    include_only.contains(&EventType::PumpFunTrade)
        && !include_only.iter().any(|t| {
            matches!(
                t,
                EventType::PumpFunBuy | EventType::PumpFunSell | EventType::PumpFunBuyExactSolIn
            )
        })
}

#[inline]
fn pumpfun_buy_filter_is_generic(include_only: &[EventType]) -> bool {
    include_only.contains(&EventType::PumpFunBuy)
        && !include_only.contains(&EventType::PumpFunBuyExactSolIn)
}

#[inline]
fn is_pumpfun_create_family(event_type: EventType) -> bool {
    matches!(event_type, EventType::PumpFunCreate | EventType::PumpFunCreateV2)
}

#[inline]
pub fn event_type_from_dex_event(event: &crate::core::events::DexEvent) -> Option<EventType> {
    use crate::core::events::DexEvent;
    match event {
        DexEvent::PumpFunCreate(_) => Some(EventType::PumpFunCreate),
        DexEvent::PumpFunCreateV2(_) => Some(EventType::PumpFunCreateV2),
        DexEvent::PumpFunTrade(_) => Some(EventType::PumpFunTrade),
        DexEvent::PumpFunBuy(_) => Some(EventType::PumpFunBuy),
        DexEvent::PumpFunSell(_) => Some(EventType::PumpFunSell),
        DexEvent::PumpFunBuyExactSolIn(_) => Some(EventType::PumpFunBuyExactSolIn),
        DexEvent::PumpFunMigrate(_) => Some(EventType::PumpFunMigrate),
        DexEvent::PumpFeesCreateFeeSharingConfig(_) => {
            Some(EventType::PumpFeesCreateFeeSharingConfig)
        }
        DexEvent::PumpFeesInitializeFeeConfig(_) => Some(EventType::PumpFeesInitializeFeeConfig),
        DexEvent::PumpFeesResetFeeSharingConfig(_) => {
            Some(EventType::PumpFeesResetFeeSharingConfig)
        }
        DexEvent::PumpFeesRevokeFeeSharingAuthority(_) => {
            Some(EventType::PumpFeesRevokeFeeSharingAuthority)
        }
        DexEvent::PumpFeesTransferFeeSharingAuthority(_) => {
            Some(EventType::PumpFeesTransferFeeSharingAuthority)
        }
        DexEvent::PumpFeesUpdateAdmin(_) => Some(EventType::PumpFeesUpdateAdmin),
        DexEvent::PumpFeesUpdateFeeConfig(_) => Some(EventType::PumpFeesUpdateFeeConfig),
        DexEvent::PumpFeesUpdateFeeShares(_) => Some(EventType::PumpFeesUpdateFeeShares),
        DexEvent::PumpFeesUpsertFeeTiers(_) => Some(EventType::PumpFeesUpsertFeeTiers),
        DexEvent::PumpFunMigrateBondingCurveCreator(_) => {
            Some(EventType::PumpFunMigrateBondingCurveCreator)
        }
        DexEvent::PumpFunGlobalAccount(_) => Some(EventType::AccountPumpFunGlobal),
        DexEvent::PumpFunBondingCurveAccount(_) => Some(EventType::AccountPumpFunBondingCurve),
        DexEvent::PumpFunFeeConfigAccount(_) => Some(EventType::AccountPumpFunFeeConfig),
        DexEvent::PumpFunSharingConfigAccount(_) => Some(EventType::AccountPumpFunSharingConfig),
        DexEvent::PumpFunGlobalVolumeAccumulatorAccount(_) => {
            Some(EventType::AccountPumpFunGlobalVolumeAccumulator)
        }
        DexEvent::PumpFunUserVolumeAccumulatorAccount(_) => {
            Some(EventType::AccountPumpFunUserVolumeAccumulator)
        }
        DexEvent::PumpSwapTrade(_) => Some(EventType::PumpSwapTrade),
        DexEvent::PumpSwapBuy(_) => Some(EventType::PumpSwapBuy),
        DexEvent::PumpSwapSell(_) => Some(EventType::PumpSwapSell),
        DexEvent::PumpSwapCreatePool(_) => Some(EventType::PumpSwapCreatePool),
        DexEvent::PumpSwapLiquidityAdded(_) => Some(EventType::PumpSwapLiquidityAdded),
        DexEvent::PumpSwapLiquidityRemoved(_) => Some(EventType::PumpSwapLiquidityRemoved),
        DexEvent::MeteoraDammV2Swap(_) => Some(EventType::MeteoraDammV2Swap),
        DexEvent::MeteoraDammV2CreatePosition(_) => Some(EventType::MeteoraDammV2CreatePosition),
        DexEvent::MeteoraDammV2ClosePosition(_) => Some(EventType::MeteoraDammV2ClosePosition),
        DexEvent::MeteoraDammV2AddLiquidity(_) => Some(EventType::MeteoraDammV2AddLiquidity),
        DexEvent::MeteoraDammV2RemoveLiquidity(_) => Some(EventType::MeteoraDammV2RemoveLiquidity),
        DexEvent::MeteoraDammV2InitializePool(_) => Some(EventType::MeteoraDammV2InitializePool),
        DexEvent::MeteoraDbcSwap(_) => Some(EventType::MeteoraDbcSwap),
        DexEvent::MeteoraDbcInitializePool(_) => Some(EventType::MeteoraDbcInitializePool),
        DexEvent::MeteoraDbcCurveComplete(_) => Some(EventType::MeteoraDbcCurveComplete),
        DexEvent::RaydiumLaunchlabTrade(_) => Some(EventType::RaydiumLaunchlabTrade),
        DexEvent::RaydiumLaunchlabPoolCreate(_) => Some(EventType::RaydiumLaunchlabPoolCreate),
        DexEvent::RaydiumLaunchlabMigrateAmm(_) => Some(EventType::RaydiumLaunchlabMigrateAmm),
        DexEvent::RaydiumClmmSwap(_) => Some(EventType::RaydiumClmmSwap),
        DexEvent::RaydiumClmmCreatePool(_) => Some(EventType::RaydiumClmmCreatePool),
        DexEvent::RaydiumClmmOpenPosition(_) => Some(EventType::RaydiumClmmOpenPosition),
        DexEvent::RaydiumClmmOpenPositionWithTokenExtNft(_) => {
            Some(EventType::RaydiumClmmOpenPositionWithTokenExtNft)
        }
        DexEvent::RaydiumClmmClosePosition(_) => Some(EventType::RaydiumClmmClosePosition),
        DexEvent::RaydiumClmmIncreaseLiquidity(_) => Some(EventType::RaydiumClmmIncreaseLiquidity),
        DexEvent::RaydiumClmmDecreaseLiquidity(_) => Some(EventType::RaydiumClmmDecreaseLiquidity),
        DexEvent::RaydiumClmmLiquidityChange(_) => Some(EventType::RaydiumClmmLiquidityChange),
        DexEvent::RaydiumClmmConfigChange(_) => Some(EventType::RaydiumClmmConfigChange),
        DexEvent::RaydiumClmmCreatePersonalPosition(_) => {
            Some(EventType::RaydiumClmmCreatePersonalPosition)
        }
        DexEvent::RaydiumClmmLiquidityCalculate(_) => {
            Some(EventType::RaydiumClmmLiquidityCalculate)
        }
        DexEvent::RaydiumClmmOpenLimitOrder(_) => Some(EventType::RaydiumClmmOpenLimitOrder),
        DexEvent::RaydiumClmmIncreaseLimitOrder(_) => {
            Some(EventType::RaydiumClmmIncreaseLimitOrder)
        }
        DexEvent::RaydiumClmmDecreaseLimitOrder(_) => {
            Some(EventType::RaydiumClmmDecreaseLimitOrder)
        }
        DexEvent::RaydiumClmmSettleLimitOrder(_) => Some(EventType::RaydiumClmmSettleLimitOrder),
        DexEvent::RaydiumClmmUpdateRewardInfos(_) => Some(EventType::RaydiumClmmUpdateRewardInfos),
        DexEvent::RaydiumClmmCollectFee(_) => Some(EventType::RaydiumClmmCollectFee),
        DexEvent::RaydiumClmmAmmConfigAccount(_) => Some(EventType::AccountRaydiumClmmAmmConfig),
        DexEvent::RaydiumClmmPoolStateAccount(_) => Some(EventType::AccountRaydiumClmmPoolState),
        DexEvent::RaydiumClmmTickArrayStateAccount(_) => {
            Some(EventType::AccountRaydiumClmmTickArrayState)
        }
        DexEvent::RaydiumCpmmSwap(_) => Some(EventType::RaydiumCpmmSwap),
        DexEvent::RaydiumCpmmDeposit(_) => Some(EventType::RaydiumCpmmDeposit),
        DexEvent::RaydiumCpmmWithdraw(_) => Some(EventType::RaydiumCpmmWithdraw),
        DexEvent::RaydiumCpmmInitialize(_) => Some(EventType::RaydiumCpmmInitialize),
        DexEvent::RaydiumCpmmAmmConfigAccount(_) => Some(EventType::AccountRaydiumCpmmAmmConfig),
        DexEvent::RaydiumCpmmPoolStateAccount(_) => Some(EventType::AccountRaydiumCpmmPoolState),
        DexEvent::RaydiumAmmV4Swap(_) => Some(EventType::RaydiumAmmV4Swap),
        DexEvent::RaydiumAmmV4Deposit(_) => Some(EventType::RaydiumAmmV4Deposit),
        DexEvent::RaydiumAmmV4Initialize2(_) => Some(EventType::RaydiumAmmV4Initialize2),
        DexEvent::RaydiumAmmV4Withdraw(_) => Some(EventType::RaydiumAmmV4Withdraw),
        DexEvent::RaydiumAmmV4WithdrawPnl(_) => Some(EventType::RaydiumAmmV4WithdrawPnl),
        DexEvent::OrcaWhirlpoolSwap(_) => Some(EventType::OrcaWhirlpoolSwap),
        DexEvent::OrcaWhirlpoolLiquidityIncreased(_) => {
            Some(EventType::OrcaWhirlpoolLiquidityIncreased)
        }
        DexEvent::OrcaWhirlpoolLiquidityDecreased(_) => {
            Some(EventType::OrcaWhirlpoolLiquidityDecreased)
        }
        DexEvent::OrcaWhirlpoolPoolInitialized(_) => Some(EventType::OrcaWhirlpoolPoolInitialized),
        DexEvent::OrcaWhirlpoolAccount(_) => Some(EventType::AccountOrcaWhirlpool),
        DexEvent::OrcaPositionAccount(_) => Some(EventType::AccountOrcaPosition),
        DexEvent::OrcaTickArrayAccount(_) => Some(EventType::AccountOrcaTickArray),
        DexEvent::OrcaFeeTierAccount(_) => Some(EventType::AccountOrcaFeeTier),
        DexEvent::OrcaWhirlpoolsConfigAccount(_) => Some(EventType::AccountOrcaWhirlpoolsConfig),
        DexEvent::MeteoraPoolsSwap(_) => Some(EventType::MeteoraPoolsSwap),
        DexEvent::MeteoraPoolsAddLiquidity(_) => Some(EventType::MeteoraPoolsAddLiquidity),
        DexEvent::MeteoraPoolsRemoveLiquidity(_) => Some(EventType::MeteoraPoolsRemoveLiquidity),
        DexEvent::MeteoraPoolsBootstrapLiquidity(_) => {
            Some(EventType::MeteoraPoolsBootstrapLiquidity)
        }
        DexEvent::MeteoraPoolsPoolCreated(_) => Some(EventType::MeteoraPoolsPoolCreated),
        DexEvent::MeteoraPoolsSetPoolFees(_) => Some(EventType::MeteoraPoolsSetPoolFees),
        DexEvent::MeteoraDlmmSwap(_) => Some(EventType::MeteoraDlmmSwap),
        DexEvent::MeteoraDlmmAddLiquidity(_) => Some(EventType::MeteoraDlmmAddLiquidity),
        DexEvent::MeteoraDlmmRemoveLiquidity(_) => Some(EventType::MeteoraDlmmRemoveLiquidity),
        DexEvent::MeteoraDlmmInitializePool(_) => Some(EventType::MeteoraDlmmInitializePool),
        DexEvent::MeteoraDlmmInitializeBinArray(_) => {
            Some(EventType::MeteoraDlmmInitializeBinArray)
        }
        DexEvent::MeteoraDlmmCreatePosition(_) => Some(EventType::MeteoraDlmmCreatePosition),
        DexEvent::MeteoraDlmmClosePosition(_) => Some(EventType::MeteoraDlmmClosePosition),
        DexEvent::MeteoraDlmmClaimFee(_) => Some(EventType::MeteoraDlmmClaimFee),
        DexEvent::TokenAccount(_) => Some(EventType::TokenAccount),
        DexEvent::TokenInfo(_) => Some(EventType::TokenInfo),
        DexEvent::NonceAccount(_) => Some(EventType::NonceAccount),
        DexEvent::PumpSwapGlobalConfigAccount(_) => Some(EventType::AccountPumpSwapGlobalConfig),
        DexEvent::PumpSwapPoolAccount(_) => Some(EventType::AccountPumpSwapPool),
        DexEvent::BlockMeta(_) => Some(EventType::BlockMeta),
        DexEvent::Error(_) => None,
    }
}

#[cfg(test)]
mod event_type_filter_tests {
    use super::*;

    #[test]
    fn generic_trade_filters_cover_specific_trade_variants() {
        let pump = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
        assert!(pump.should_include(EventType::PumpFunTrade));
        assert!(pump.should_include(EventType::PumpFunBuy));
        assert!(pump.should_include(EventType::PumpFunSell));
        assert!(pump.should_include(EventType::PumpFunBuyExactSolIn));

        let pump_specific = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        assert!(!pump_specific.should_include(EventType::PumpFunTrade));
        assert!(pump_specific.should_include(EventType::PumpFunBuy));
        assert!(!pump_specific.should_include(EventType::PumpFunSell));
        assert!(pump_specific.should_include(EventType::PumpFunBuyExactSolIn));

        let pump_exact_buy = EventTypeFilter::include_only(vec![EventType::PumpFunBuyExactSolIn]);
        assert!(!pump_exact_buy.should_include(EventType::PumpFunTrade));
        assert!(!pump_exact_buy.should_include(EventType::PumpFunBuy));
        assert!(!pump_exact_buy.should_include(EventType::PumpFunSell));
        assert!(pump_exact_buy.should_include(EventType::PumpFunBuyExactSolIn));

        let pumpswap = EventTypeFilter::include_only(vec![EventType::PumpSwapTrade]);
        assert!(pumpswap.should_include(EventType::PumpSwapBuy));
        assert!(pumpswap.should_include(EventType::PumpSwapSell));

        let exclude_pumpswap = EventTypeFilter::exclude_types(vec![EventType::PumpSwapTrade]);
        assert!(!exclude_pumpswap.should_include(EventType::PumpSwapBuy));
        assert!(!exclude_pumpswap.should_include(EventType::PumpSwapSell));
    }

    #[test]
    fn generic_pumpfun_trade_filter_normalizes_specific_variants() {
        use crate::core::events::{DexEvent, PumpFunTradeEvent};

        let filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);
        let event = DexEvent::PumpFunBuy(PumpFunTradeEvent { is_buy: true, ..Default::default() });
        assert!(matches!(filter.normalize_dex_event(event), DexEvent::PumpFunTrade(_)));

        let specific_filter =
            EventTypeFilter::include_only(vec![EventType::PumpFunTrade, EventType::PumpFunBuy]);
        let event = DexEvent::PumpFunBuy(PumpFunTradeEvent { is_buy: true, ..Default::default() });
        assert!(matches!(specific_filter.normalize_dex_event(event), DexEvent::PumpFunBuy(_)));

        let buy_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuy]);
        let event = DexEvent::PumpFunBuyExactSolIn(PumpFunTradeEvent {
            is_buy: true,
            ..Default::default()
        });
        assert!(matches!(buy_filter.normalize_dex_event(event), DexEvent::PumpFunBuy(_)));

        let exact_filter = EventTypeFilter::include_only(vec![EventType::PumpFunBuyExactSolIn]);
        let event = DexEvent::PumpFunBuyExactSolIn(PumpFunTradeEvent {
            is_buy: true,
            ..Default::default()
        });
        assert!(matches!(
            exact_filter.normalize_dex_event(event),
            DexEvent::PumpFunBuyExactSolIn(_)
        ));

        let create_and_trade_filter =
            EventTypeFilter::include_only(vec![EventType::PumpFunCreate, EventType::PumpFunTrade]);
        let event =
            DexEvent::PumpFunSell(PumpFunTradeEvent { is_buy: false, ..Default::default() });
        assert!(matches!(
            create_and_trade_filter.normalize_dex_event(event),
            DexEvent::PumpFunTrade(_)
        ));
    }

    #[test]
    fn all_protocol_groups_are_filterable() {
        assert!(EventTypeFilter::include_only(vec![EventType::PumpFunTrade]).includes_pumpfun());
        assert!(!EventTypeFilter::include_only(vec![EventType::AccountPumpFunGlobal])
            .includes_pumpfun());
        assert!(
            !EventTypeFilter::include_only(vec![EventType::PumpFeesUpdateAdmin]).includes_pumpfun()
        );
        assert!(EventTypeFilter::include_only(vec![EventType::PumpSwapTrade]).includes_pumpswap());
        assert!(EventTypeFilter::include_only(vec![EventType::PumpFeesUpdateFeeShares])
            .includes_pump_fees());
        assert!(EventTypeFilter::include_only(vec![EventType::RaydiumLaunchlabTrade])
            .includes_raydium_launchlab());
        assert!(
            EventTypeFilter::include_only(vec![EventType::RaydiumCpmmSwap]).includes_raydium_cpmm()
        );
        assert!(
            EventTypeFilter::include_only(vec![EventType::RaydiumClmmSwap]).includes_raydium_clmm()
        );
        assert!(!EventTypeFilter::include_only(vec![EventType::AccountRaydiumClmmPoolState])
            .includes_raydium_clmm());
        assert!(EventTypeFilter::include_only(vec![EventType::RaydiumAmmV4Swap])
            .includes_raydium_amm_v4());
        assert!(EventTypeFilter::include_only(vec![EventType::OrcaWhirlpoolSwap])
            .includes_orca_whirlpool());
        assert!(EventTypeFilter::include_only(vec![EventType::MeteoraPoolsSwap])
            .includes_meteora_pools());
        assert!(EventTypeFilter::include_only(vec![EventType::MeteoraDammV2Swap])
            .includes_meteora_damm_v2());
        assert!(EventTypeFilter::include_only(vec![EventType::MeteoraDammV2InitializePool])
            .includes_meteora_damm_v2());
        assert!(
            EventTypeFilter::include_only(vec![EventType::MeteoraDlmmSwap]).includes_meteora_dlmm()
        );
        assert!(
            EventTypeFilter::include_only(vec![EventType::MeteoraDbcSwap]).includes_meteora_dbc()
        );
    }

    #[test]
    fn exclude_filters_do_not_skip_whole_protocol_groups() {
        let raydium = EventTypeFilter::exclude_types(vec![EventType::RaydiumCpmmSwap]);
        assert!(raydium.includes_raydium_cpmm());
        assert!(!raydium.should_include(EventType::RaydiumCpmmSwap));
        assert!(raydium.should_include(EventType::RaydiumCpmmDeposit));

        let all_cpmm = EventTypeFilter::exclude_types(vec![
            EventType::RaydiumCpmmSwap,
            EventType::RaydiumCpmmDeposit,
            EventType::RaydiumCpmmWithdraw,
            EventType::RaydiumCpmmInitialize,
        ]);
        assert!(!all_cpmm.includes_raydium_cpmm());

        let all_launchlab = EventTypeFilter::exclude_types(vec![
            EventType::RaydiumLaunchlabTrade,
            EventType::RaydiumLaunchlabPoolCreate,
            EventType::RaydiumLaunchlabMigrateAmm,
        ]);
        assert!(!all_launchlab.includes_raydium_launchlab());

        let pump = EventTypeFilter::exclude_types(vec![EventType::PumpFunBuy]);
        assert!(pump.includes_pumpfun());
        assert!(!pump.should_include(EventType::PumpFunBuy));
        assert!(!pump.should_include(EventType::PumpFunBuyExactSolIn));
        assert!(pump.should_include(EventType::PumpFunSell));
    }
}

#[derive(Debug, Clone)]
pub struct SlotFilter {
    pub min_slot: Option<u64>,
    pub max_slot: Option<u64>,
}

impl SlotFilter {
    pub fn new() -> Self {
        Self { min_slot: None, max_slot: None }
    }

    pub fn min_slot(mut self, slot: u64) -> Self {
        self.min_slot = Some(slot);
        self
    }

    pub fn max_slot(mut self, slot: u64) -> Self {
        self.max_slot = Some(slot);
        self
    }
}

impl Default for SlotFilter {
    fn default() -> Self {
        Self::new()
    }
}
