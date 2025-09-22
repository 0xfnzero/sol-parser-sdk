use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            buffer_size: 8192,
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
            buffer_size: 16384,
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
            buffer_size: 32768,
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
    pub filters: Vec<AccountFilterData>,
}

impl AccountFilter {
    pub fn new() -> Self {
        Self {
            account: Vec::new(),
            owner: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn add_account(mut self, account: impl Into<String>) -> Self {
        self.account.push(account.into());
        self
    }

    pub fn add_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner.push(owner.into());
        self
    }

    pub fn add_filter(mut self, filter: AccountFilterData) -> Self {
        self.filters.push(filter);
        self
    }

    /// 从程序ID列表创建所有者过滤器
    pub fn from_program_owners(program_ids: Vec<String>) -> Self {
        Self {
            account: Vec::new(),
            owner: program_ids,
            filters: Vec::new(),
        }
    }
}

impl Default for AccountFilter {
    fn default() -> Self {
        Self::new()
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
    Bonk,
    RaydiumCpmm,
    RaydiumClmm,
    RaydiumAmmV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // Block events
    BlockMeta,

    // Bonk events
    BonkTrade,
    BonkPoolCreate,
    BonkMigrateAmm,

    // PumpFun events
    PumpFunTrade,
    PumpFunCreate,
    PumpFunComplete,
    PumpFunMigrate,

    // PumpSwap events
    PumpSwapBuy,
    PumpSwapSell,
    PumpSwapCreatePool,
    PumpSwapPoolCreated,
    PumpSwapTrade,
    PumpSwapLiquidityAdded,
    PumpSwapLiquidityRemoved,
    PumpSwapPoolUpdated,
    PumpSwapFeesClaimed,

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
    MeteoraDammV2ClaimPositionFee,
    MeteoraDammV2InitializeReward,
    MeteoraDammV2FundReward,
    MeteoraDammV2ClaimReward,

    // Account events
    TokenAccount,
    NonceAccount,
    TokenInfo,
}

#[derive(Debug, Clone)]
pub struct EventTypeFilter {
    pub include_only: Option<Vec<EventType>>,
    pub exclude_types: Option<Vec<EventType>>,
}

impl EventTypeFilter {
    pub fn include_only(types: Vec<EventType>) -> Self {
        Self {
            include_only: Some(types),
            exclude_types: None,
        }
    }

    pub fn exclude_types(types: Vec<EventType>) -> Self {
        Self {
            include_only: None,
            exclude_types: Some(types),
        }
    }

    pub fn should_include(&self, event_type: EventType) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.contains(&event_type);
        }

        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.contains(&event_type);
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct SlotFilter {
    pub min_slot: Option<u64>,
    pub max_slot: Option<u64>,
}

impl SlotFilter {
    pub fn new() -> Self {
        Self {
            min_slot: None,
            max_slot: None,
        }
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