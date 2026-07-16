//! 所有具体的事件类型定义
//!
//! 基于您提供的回调事件列表，定义所有需要的具体事件类型

// use prost_types::Timestamp;

use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey, signature::Signature};

/// Solscan SOL quote-mint sentinel used when PumpFun legacy events omit quote_mint.
///
/// PumpFun native-SOL instructions and older trade logs do not carry a real SPL quote mint.
/// We expose the same SOL placeholder Solscan displays instead of `Pubkey::default()`.
pub const PUMPFUN_SOLSCAN_SOL_QUOTE_MINT: Pubkey =
    pubkey!("So11111111111111111111111111111111111111111");

/// SPL wrapped-SOL mint.
pub const PUMPFUN_WSOL_QUOTE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

#[inline]
pub fn normalize_pumpfun_quote_mint(quote_mint: Pubkey) -> Pubkey {
    if quote_mint == Pubkey::default() {
        PUMPFUN_SOLSCAN_SOL_QUOTE_MINT
    } else {
        quote_mint
    }
}

#[inline]
pub fn is_pumpfun_solscan_sol_quote_mint(quote_mint: Pubkey) -> bool {
    normalize_pumpfun_quote_mint(quote_mint) == PUMPFUN_SOLSCAN_SOL_QUOTE_MINT
}

/// 基础元数据 - 所有事件共享的字段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    pub signature: Signature,
    pub slot: u64,
    pub tx_index: u64, // 交易在slot中的索引，参考solana-streamer
    pub block_time_us: i64,
    pub grpc_recv_us: i64,
    /// Transaction's recent blockhash as base58 string, when available.
    #[serde(default)]
    pub recent_blockhash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pumpfun_default_quote_mint_uses_solscan_sol_sentinel() {
        let quote_mint = normalize_pumpfun_quote_mint(Pubkey::default());

        assert_eq!(quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
        assert_eq!(quote_mint.to_string(), "So11111111111111111111111111111111111111111");
    }

    #[test]
    fn pumpfun_wsol_quote_mint_is_preserved() {
        assert_eq!(normalize_pumpfun_quote_mint(PUMPFUN_WSOL_QUOTE_MINT), PUMPFUN_WSOL_QUOTE_MINT);
        assert!(!is_pumpfun_solscan_sol_quote_mint(PUMPFUN_WSOL_QUOTE_MINT));
    }
}

/// Block Meta Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetaEvent {
    pub metadata: EventMetadata,
}

/// RaydiumLaunchlab Pool Create Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabPoolCreateEvent {
    pub metadata: EventMetadata,
    pub base_mint_param: BaseMintParam,
    pub pool_state: Pubkey,
    pub creator: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMintParam {
    pub symbol: String,
    pub name: String,
    pub uri: String,
    pub decimals: u8,
}

/// RaydiumLaunchlab Trade Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabTradeEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool_state: Pubkey, // 32 bytes
    pub user: Pubkey,       // 32 bytes
    pub amount_in: u64,     // 8 bytes
    pub amount_out: u64,    // 8 bytes
    pub is_buy: bool,       // 1 byte

    // === 非 Borsh 字段（派生字段）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub trade_direction: TradeDirection,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub exact_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TradeDirection {
    #[default]
    Buy,
    Sell,
}

/// RaydiumLaunchlab Migrate AMM Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabMigrateAmmEvent {
    pub metadata: EventMetadata,
    pub old_pool: Pubkey,
    pub new_pool: Pubkey,
    pub user: Pubkey,
    pub liquidity_amount: u64,
}

/// PumpFun Trade Event - 基于官方IDL定义
///
/// 字段来源标记:
/// - EVENT: 来自原始IDL事件定义，由程序日志直接解析获得
/// - INSTRUCTION: 来自指令解析，用于补充事件缺失的上下文信息
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpFunTradeEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,

    // === IDL TradeEvent 事件字段（Borsh 序列化字段，按顺序）===
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    #[borsh(skip)]
    pub is_created_buy: bool, // 由外层逻辑设置，不在 Borsh 数据中
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
    /// Instruction name as emitted by the program, for example `"buy"`, `"buy_v2"`,
    /// `"sell"`, `"sell_v2"`, `"buy_exact_sol_in"`, or `"buy_exact_quote_in_v2"`.
    pub ix_name: String,
    /// 与链上 / Explorer `tradeEvent` 中 `mayhemMode` 一致（gRPC 日志解析填充；勿用 fee 地址推断）。
    pub mayhem_mode: bool,
    /// Cashback fee basis points (PUMP_CASHBACK_README)
    pub cashback_fee_basis_points: u64,
    /// Cashback amount (PUMP_CASHBACK_README)
    pub cashback: u64,
    pub buyback_fee_basis_points: u64,
    pub buyback_fee: u64,
    pub shareholders: Vec<PumpFeesShareholder>,
    pub quote_mint: Pubkey,
    pub quote_amount: u64,
    pub virtual_quote_reserves: u64,
    pub real_quote_reserves: u64,
    /// 是否返现代币（由 cashback_fee_basis_points > 0 推导，供 sol-trade-sdk 等构造 sell 指令用）
    #[borsh(skip)]
    pub is_cashback_coin: bool,

    // === Instruction parameter fields (not present in the on-chain TradeEvent Borsh payload) ===
    #[borsh(skip)]
    pub amount: u64, // buy/sell.args.amount
    #[borsh(skip)]
    pub max_sol_cost: u64, // buy.args.max_sol_cost
    #[borsh(skip)]
    pub min_sol_output: u64, // sell.args.min_sol_output
    #[borsh(skip)]
    pub spendable_sol_in: u64, // buy_exact_sol_in.args.spendable_sol_in
    #[borsh(skip)]
    pub spendable_quote_in: u64, // buy_exact_quote_in_v2.args.spendable_quote_in
    #[borsh(skip)]
    pub min_tokens_out: u64, // buy_exact*.args.min_tokens_out

    // === 指令账户字段 (从指令账户填充，不在 Borsh 数据中) ===
    #[borsh(skip)]
    pub global: Pubkey, // legacy 0 / v2 0
    #[borsh(skip)]
    pub bonding_curve: Pubkey, // legacy 3 / v2 10
    #[borsh(skip)]
    pub bonding_curve_v2: Pubkey, // legacy sell cashback remaining_accounts[1]
    #[borsh(skip)]
    pub associated_bonding_curve: Pubkey, // legacy 4 / v2 associated_base_bonding_curve 11
    #[borsh(skip)]
    pub associated_user: Pubkey, // legacy 5 / v2 associated_base_user 14
    #[borsh(skip)]
    pub system_program: Pubkey, // legacy 7 / v2 buy 24, sell 23
    #[borsh(skip)]
    pub token_program: Pubkey, // legacy sell 9 / buy 8 / v2 base_token_program 3
    #[borsh(skip)]
    pub quote_token_program: Pubkey, // v2 4
    #[borsh(skip)]
    pub associated_token_program: Pubkey, // v2 5
    #[borsh(skip)]
    pub creator_vault: Pubkey, // legacy sell 8 / buy 9 / v2 16
    #[borsh(skip)]
    pub associated_quote_fee_recipient: Pubkey, // v2 7
    #[borsh(skip)]
    pub buyback_fee_recipient: Pubkey, // v2 8
    #[borsh(skip)]
    pub associated_quote_buyback_fee_recipient: Pubkey, // v2 9
    #[borsh(skip)]
    pub associated_quote_bonding_curve: Pubkey, // v2 12
    #[borsh(skip)]
    pub associated_quote_user: Pubkey, // v2 15
    #[borsh(skip)]
    pub associated_creator_vault: Pubkey, // v2 17
    #[borsh(skip)]
    pub sharing_config: Pubkey, // v2 18
    #[borsh(skip)]
    pub event_authority: Pubkey, // legacy buy 10 / sell 10 / v2 buy 25, sell 24
    #[borsh(skip)]
    pub program: Pubkey, // legacy buy 11 / sell 11 / v2 buy 26, sell 25
    #[borsh(skip)]
    pub global_volume_accumulator: Pubkey, // buy/exact buy legacy 12 / v2 buy 19
    #[borsh(skip)]
    pub user_volume_accumulator: Pubkey, // legacy buy/exact buy 13 / v2 buy 20, sell 19
    #[borsh(skip)]
    pub associated_user_volume_accumulator: Pubkey, // v2 buy 21 / sell 20
    #[borsh(skip)]
    pub fee_config: Pubkey, // legacy buy 14 / sell 12 / v2 buy 22, sell 21
    #[borsh(skip)]
    pub fee_program: Pubkey, // legacy buy 15 / sell 13 / v2 buy 23, sell 22
    /// Legacy fallback alias for the last post-upgrade extra account. Prefer
    /// `bonding_curve_v2` and `buyback_fee_recipient` for structured access.
    #[borsh(skip)]
    pub account: Option<Pubkey>,
}

/// PumpFun Migrate Event — bonding curve 完成向池子（如 Pump AMM/Raydium）迁移等
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpFunMigrateEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,
    // Borsh 序列化字段（按顺序）
    pub user: Pubkey,
    pub mint: Pubkey,
    pub mint_amount: u64,
    pub sol_amount: u64,
    pub pool_migration_fee: u64,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
    pub pool: Pubkey,
    // === 额外账户信息（用于指令解析，暂时注释，以后可能会用到，AI不要删除） ===
    // pub global: Pubkey,
    // pub withdraw_authority: Pubkey,
    // pub associated_bonding_curve: Pubkey,
    // pub pump_amm: Pubkey,
    // pub pool_authority: Pubkey,
    // pub pool_authority_mint_account: Pubkey,
    // pub pool_authority_wsol_account: Pubkey,
    // pub amm_global_config: Pubkey,
    // pub wsol_mint: Pubkey,
    // pub lp_mint: Pubkey,
    // pub user_pool_token_account: Pubkey,
    // pub pool_base_token_account: Pubkey,
    // pub pool_quote_token_account: Pubkey,
}

// ---------- pump-fees IDL：`idls/pump_fees.json`（Program `pfeeUx...`）----------

/// IDL `Shareholder`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, BorshDeserialize)]
pub struct PumpFeesShareholder {
    pub address: Pubkey,
    pub share_bps: u16,
}

/// IDL `ConfigStatus`（Anchor Borsh：enum 判别）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PumpFeesConfigStatus {
    Paused,
    Active,
}

/// IDL `Fees`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PumpFeesFees {
    pub lp_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub creator_fee_bps: u64,
}

/// IDL `FeeTier`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PumpFeesFeeTier {
    pub market_cap_lamports_threshold: u128,
    pub fees: PumpFeesFees,
}

/// IDL `CreateFeeSharingConfigEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesCreateFeeSharingConfigEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub pool: Option<Pubkey>,
    pub sharing_config: Pubkey,
    pub admin: Pubkey,
    pub initial_shareholders: Vec<PumpFeesShareholder>,
    pub status: PumpFeesConfigStatus,
}

/// IDL `InitializeFeeConfigEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesInitializeFeeConfigEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub admin: Pubkey,
    pub fee_config: Pubkey,
}

/// IDL `ResetFeeSharingConfigEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesResetFeeSharingConfigEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub sharing_config: Pubkey,
    pub old_admin: Pubkey,
    pub old_shareholders: Vec<PumpFeesShareholder>,
    pub new_admin: Pubkey,
    pub new_shareholders: Vec<PumpFeesShareholder>,
}

/// IDL `RevokeFeeSharingAuthorityEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesRevokeFeeSharingAuthorityEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub sharing_config: Pubkey,
    pub admin: Pubkey,
}

/// IDL `TransferFeeSharingAuthorityEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesTransferFeeSharingAuthorityEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub sharing_config: Pubkey,
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}

/// IDL `UpdateAdminEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesUpdateAdminEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}

/// IDL `UpdateFeeConfigEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesUpdateFeeConfigEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub admin: Pubkey,
    pub fee_config: Pubkey,
    pub fee_tiers: Vec<PumpFeesFeeTier>,
    pub flat_fees: PumpFeesFees,
}

/// IDL `UpdateFeeSharesEvent`
///
/// 链上 **`update_fee_shares` / `update_fee_shares_v2` 指令**还会在账户列表中带上
/// `bonding_curve`、`pump_creator_vault`（Explorer #7/#8，`Program data` 日志体不含这两字段 ⇒ 仍为 `default`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesUpdateFeeSharesEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub sharing_config: Pubkey,
    pub admin: Pubkey,
    /// IDL：`bonding_curve`（ix 账户约 #7；Explorer `#7`）。
    #[serde(default)]
    pub bonding_curve: Pubkey,
    /// **`pump_creator_vault`**：`creator-vault` PDA(seed 含 sharing_config)，ix 账户约 #8（Explorer `#8`）。
    #[serde(default)]
    pub pump_creator_vault: Pubkey,
    pub new_shareholders: Vec<PumpFeesShareholder>,
}

/// IDL `UpsertFeeTiersEvent`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFeesUpsertFeeTiersEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub admin: Pubkey,
    pub fee_config: Pubkey,
    pub fee_tiers: Vec<PumpFeesFeeTier>,
    pub offset: u8,
}

/// Pump.fun：曲线 creator 迁移（与费分成 onboarding 同框常见）
///
/// **`new_creator` 常为后续 `tradeEvent.creator`、`creator_vault` PDA 种子。**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunMigrateBondingCurveCreatorEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub sharing_config: Pubkey,
    pub old_creator: Pubkey,
    pub new_creator: Pubkey,
}

/// PumpFun Create Token Event - Based on IDL CreateEvent definition
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpFunCreateTokenEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,
    // IDL CreateEvent 字段（Borsh 序列化字段，按顺序）
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,

    pub token_program: Pubkey,
    pub is_mayhem_mode: bool,
    /// Cashback 是否开启 (IDL CreateEvent.is_cashback_enabled)
    pub is_cashback_enabled: bool,
    /// Quote mint for v2 quote pools (for example USDC).
    pub quote_mint: Pubkey,
    /// Quote-side vault account appended by PumpFun `create_v2` quote pools.
    #[borsh(skip)]
    pub quote_vault: Pubkey,
    /// Quote-side token program appended by PumpFun `create_v2` quote pools.
    #[borsh(skip)]
    pub quote_token_program: Pubkey,
    /// Initial virtual quote reserves. For SOL pools this is the SOL-side reserve;
    /// for USDC pools this is the USDC-side reserve.
    pub virtual_quote_reserves: u64,
    /// Original PumpFun instruction name: `"create"` or `"create_v2"`.
    #[borsh(skip)]
    pub ix_name: String,
    #[borsh(skip)]
    pub mint_authority: Pubkey,
    #[borsh(skip)]
    pub associated_bonding_curve: Pubkey,
    #[borsh(skip)]
    pub global: Pubkey,
    #[borsh(skip)]
    pub system_program: Pubkey,
    #[borsh(skip)]
    pub associated_token_program: Pubkey,
    #[borsh(skip)]
    pub mayhem_program_id: Pubkey,
    #[borsh(skip)]
    pub global_params: Pubkey,
    #[borsh(skip)]
    pub sol_vault: Pubkey,
    #[borsh(skip)]
    pub mayhem_state: Pubkey,
    #[borsh(skip)]
    pub mayhem_token_vault: Pubkey,
    #[borsh(skip)]
    pub event_authority: Pubkey,
    #[borsh(skip)]
    pub program: Pubkey,
    /// Same-tx later Pump Buy fee recipient, filled by `pumpfun_fee_enrich`.
    #[borsh(skip)]
    pub observed_fee_recipient: Pubkey,
}

/// PumpFun Create V2 Token Event (SPL-22 / Mayhem Mode)
/// 与 solana-streamer 对齐；指令解析时从 accounts 0..15 填充。
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpFunCreateV2TokenEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
    pub token_program: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_enabled: bool,
    #[borsh(skip)]
    pub quote_mint: Pubkey,
    #[borsh(skip)]
    pub quote_vault: Pubkey,
    #[borsh(skip)]
    pub quote_token_program: Pubkey,
    #[borsh(skip)]
    pub virtual_quote_reserves: u64,
    /// Original PumpFun instruction name: `"create"` or `"create_v2"`.
    #[borsh(skip)]
    pub ix_name: String,
    #[borsh(skip)]
    pub mint_authority: Pubkey,
    #[borsh(skip)]
    pub associated_bonding_curve: Pubkey,
    #[borsh(skip)]
    pub global: Pubkey,
    #[borsh(skip)]
    pub system_program: Pubkey,
    #[borsh(skip)]
    pub associated_token_program: Pubkey,
    #[borsh(skip)]
    pub mayhem_program_id: Pubkey,
    #[borsh(skip)]
    pub global_params: Pubkey,
    #[borsh(skip)]
    pub sol_vault: Pubkey,
    #[borsh(skip)]
    pub mayhem_state: Pubkey,
    #[borsh(skip)]
    pub mayhem_token_vault: Pubkey,
    #[borsh(skip)]
    pub event_authority: Pubkey,
    #[borsh(skip)]
    pub program: Pubkey,
    /// 同笔交易中后续 Pump Buy 的账户 #2（或 trade 日志中的 fee recipient）；由 `pumpfun_fee_enrich` 回填。
    #[borsh(skip)]
    pub observed_fee_recipient: Pubkey,
}

/// PumpSwap Trade Event - Unified trade event from IDL TradeEvent
/// Produced by: buy, sell, buy_exact_sol_in instructions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapTradeEvent {
    pub metadata: EventMetadata,
    // === IDL TradeEvent fields ===
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
    pub ix_name: String, // "buy" | "sell" | "buy_exact_sol_in"
}

/// PumpSwap Buy Event
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpSwapBuyEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub base_amount_out: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_in: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_in_with_lp_fee: u64,
    pub user_quote_amount_in: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
    pub coin_creator: Pubkey,
    pub coin_creator_fee_basis_points: u64,
    pub coin_creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
    /// Minimum base token amount expected (new field from IDL update)
    pub min_base_amount_out: u64,
    /// Instruction name (new field from IDL update)
    pub ix_name: String,
    /// Cashback fee basis points (PUMP_CASHBACK_README)
    pub cashback_fee_basis_points: u64,
    /// Cashback amount (PUMP_CASHBACK_README)
    pub cashback: u64,

    // === 额外的信息 ===
    #[borsh(skip)]
    pub is_pump_pool: bool,

    // === 额外账户信息 (from instruction accounts, not event data) ===
    #[borsh(skip)]
    pub base_mint: Pubkey,
    #[borsh(skip)]
    pub quote_mint: Pubkey,
    #[borsh(skip)]
    pub pool_base_token_account: Pubkey,
    #[borsh(skip)]
    pub pool_quote_token_account: Pubkey,
    #[borsh(skip)]
    pub coin_creator_vault_ata: Pubkey,
    #[borsh(skip)]
    pub coin_creator_vault_authority: Pubkey,
    #[borsh(skip)]
    pub base_token_program: Pubkey,
    #[borsh(skip)]
    pub quote_token_program: Pubkey,
    #[borsh(skip)]
    pub pool_v2: Pubkey,
    #[borsh(skip)]
    pub fee_recipient: Pubkey,
    #[borsh(skip)]
    pub fee_recipient_quote_token_account: Pubkey,
}

/// PumpSwap Sell Event
#[derive(Debug, Clone, Serialize, Deserialize, Default, BorshDeserialize)]
pub struct PumpSwapSellEvent {
    #[borsh(skip)]
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_out: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub user_quote_amount_out: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
    pub coin_creator: Pubkey,
    pub coin_creator_fee_basis_points: u64,
    pub coin_creator_fee: u64,
    /// Cashback fee basis points (PUMP_CASHBACK_README)
    pub cashback_fee_basis_points: u64,
    /// Cashback amount (PUMP_CASHBACK_README)
    pub cashback: u64,

    // === 额外的信息 ===
    #[borsh(skip)]
    pub is_pump_pool: bool,

    // === 额外账户信息 (from instruction accounts, not event data) ===
    #[borsh(skip)]
    pub base_mint: Pubkey,
    #[borsh(skip)]
    pub quote_mint: Pubkey,
    #[borsh(skip)]
    pub pool_base_token_account: Pubkey,
    #[borsh(skip)]
    pub pool_quote_token_account: Pubkey,
    #[borsh(skip)]
    pub coin_creator_vault_ata: Pubkey,
    #[borsh(skip)]
    pub coin_creator_vault_authority: Pubkey,
    #[borsh(skip)]
    pub base_token_program: Pubkey,
    #[borsh(skip)]
    pub quote_token_program: Pubkey,
    #[borsh(skip)]
    pub pool_v2: Pubkey,
    #[borsh(skip)]
    pub fee_recipient: Pubkey,
    #[borsh(skip)]
    pub fee_recipient_quote_token_account: Pubkey,
}

/// PumpSwap Create Pool Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapCreatePoolEvent {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint_decimals: u8,
    pub quote_mint_decimals: u8,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub pool_base_amount: u64,
    pub pool_quote_amount: u64,
    pub minimum_liquidity: u64,
    pub initial_liquidity: u64,
    pub lp_token_amount_out: u64,
    pub pool_bump: u8,
    pub pool: Pubkey,
    pub lp_mint: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub coin_creator: Pubkey,
    /// IDL CreatePoolEvent last field.
    pub is_mayhem_mode: bool,
    /// create_pool instruction arg and Pool account field. Log-only CreatePoolEvent payloads do
    /// not carry this value, so log-only parses keep the default `false`.
    #[serde(default)]
    pub is_cashback_coin: bool,
}

/// PumpSwap Pool Created Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPoolCreated {
    pub metadata: EventMetadata,
    pub pool_account: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub creator: Pubkey,
    pub authority: Pubkey,
    pub initial_token_a_amount: u64,
    pub initial_token_b_amount: u64,
}

// PumpSwap Trade Event - 指令解析版本
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PumpSwapTrade {
//     pub metadata: EventMetadata,
//     pub pool_account: Pubkey,
//     pub user: Pubkey,
//     pub user_token_in_account: Pubkey,
//     pub user_token_out_account: Pubkey,
//     pub pool_token_in_vault: Pubkey,
//     pub pool_token_out_vault: Pubkey,
//     pub token_in_mint: Pubkey,
//     pub token_out_mint: Pubkey,
//     pub amount_in: u64,
//     pub minimum_amount_out: u64,
//     pub is_token_a_to_b: bool,
// }

/// PumpSwap Liquidity Added Event - Instruction parsing version
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapLiquidityAdded {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub lp_token_amount_out: u64,
    pub max_base_amount_in: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub lp_mint_supply: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub user_pool_token_account: Pubkey,
}

/// PumpSwap Liquidity Removed Event - Instruction parsing version
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapLiquidityRemoved {
    pub metadata: EventMetadata,
    pub timestamp: i64,
    pub lp_token_amount_in: u64,
    pub min_base_amount_out: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_out: u64,
    pub quote_amount_out: u64,
    pub lp_mint_supply: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub user_pool_token_account: Pubkey,
}

/// PumpSwap Pool Updated Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPoolUpdated {
    pub metadata: EventMetadata,
    pub pool_account: Pubkey,
    pub authority: Pubkey,
    pub admin: Pubkey,
    pub new_fee_rate: u64,
}

/// PumpSwap Fees Claimed Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapFeesClaimed {
    pub metadata: EventMetadata,
    pub pool_account: Pubkey,
    pub authority: Pubkey,
    pub admin: Pubkey,
    pub admin_token_a_account: Pubkey,
    pub admin_token_b_account: Pubkey,
    pub pool_fee_vault: Pubkey,
}

/// PumpSwap Deposit Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapDepositEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
}

/// PumpSwap Withdraw Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapWithdrawEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
}

/// Raydium CPMM Swap Event (基于IDL SwapEvent + swapBaseInput指令定义)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmSwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub pool_id: Pubkey,
    pub input_amount: u64,
    pub output_amount: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub input_vault_before: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub output_vault_before: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub input_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub output_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub base_input: bool,
    // === 指令参数字段 (暂时注释，以后可能会用到，AI不要删除) ===
    // pub amount_in: u64,
    // pub minimum_amount_out: u64,

    // === 指令账户字段 (暂时注释，以后可能会用到，AI不要删除) ===
    // pub payer: Pubkey,              // 0: payer
    // pub authority: Pubkey,          // 1: authority
    // pub amm_config: Pubkey,         // 2: ammConfig
    // pub pool_state: Pubkey,         // 3: poolState
    // pub input_token_account: Pubkey, // 4: inputTokenAccount
    // pub output_token_account: Pubkey, // 5: outputTokenAccount
    // pub input_vault: Pubkey,        // 6: inputVault
    // pub output_vault: Pubkey,       // 7: outputVault
    // pub input_token_mint: Pubkey,   // 10: inputTokenMint
    // pub output_token_mint: Pubkey,  // 11: outputTokenMint
}

/// Raydium CPMM Deposit Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmDepositEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub pool: Pubkey,
    pub token0_amount: u64,
    pub token1_amount: u64,
    pub lp_token_amount: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user: Pubkey,
}

/// Raydium CPMM Initialize Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmInitializeEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub creator: Pubkey,
    pub init_amount0: u64,
    pub init_amount1: u64,
}

/// Raydium CPMM Withdraw Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmWithdrawEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub pool: Pubkey,
    pub lp_token_amount: u64,
    pub token0_amount: u64,
    pub token1_amount: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user: Pubkey,
}

/// Raydium CLMM Swap Event (IDL `SwapEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmSwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    pub pool_state: Pubkey,
    pub sender: Pubkey,
    pub token_account_0: Pubkey,
    pub token_account_1: Pubkey,
    pub amount_0: u64,
    pub transfer_fee_0: u64,
    pub amount_1: u64,
    pub transfer_fee_1: u64,
    pub zero_for_one: bool,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick: i32,
}

/// Raydium CLMM Close Position Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmClosePositionEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub position_nft_mint: Pubkey,
}

/// Raydium CLMM Decrease Liquidity Event (IDL `DecreaseLiquidityEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmDecreaseLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    pub position_nft_mint: Pubkey,
    pub liquidity: u128,
    pub decrease_amount_0: u64,
    pub decrease_amount_1: u64,
    pub fee_amount_0: u64,
    pub fee_amount_1: u64,
    pub reward_amounts: [u64; 3],
    pub transfer_fee_0: u64,
    pub transfer_fee_1: u64,

    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amount0_min: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amount1_min: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user: Pubkey,
}

/// Raydium CLMM Collect Fee Event
///
/// Raydium emits separate `CollectPersonalFeeEvent` and
/// `CollectProtocolFeeEvent` layouts. They are normalized into this single SDK
/// event, so this struct intentionally does not derive `BorshDeserialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmCollectFeeEvent {
    pub metadata: EventMetadata,
    pub pool_state: Pubkey,
    pub position_nft_mint: Pubkey,
    pub recipient_token_account_0: Pubkey,
    pub recipient_token_account_1: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
}

/// Raydium CLMM Create Pool Event (IDL `PoolCreatedEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmCreatePoolEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub tick_spacing: u16,
    pub pool: Pubkey,
    pub sqrt_price_x64: u128,
    pub tick: i32,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,

    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub fee_rate: u32,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub creator: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub open_time: u64,
}

/// Raydium CLMM Increase Liquidity Event (IDL `IncreaseLiquidityEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmIncreaseLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    pub position_nft_mint: Pubkey,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub amount_0_transfer_fee: u64,
    pub amount_1_transfer_fee: u64,

    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amount0_max: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amount1_max: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user: Pubkey,
}

/// Raydium CLMM Liquidity Change Event (IDL `LiquidityChangeEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmLiquidityChangeEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_state: Pubkey,
    pub tick: i32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_before: u128,
    pub liquidity_after: u128,
}

/// Raydium CLMM Config Change Event (IDL `ConfigChangeEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmConfigChangeEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub index: u16,
    pub owner: Pubkey,
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u16,
    pub fund_fee_rate: u32,
    pub fund_owner: Pubkey,
}

/// Raydium CLMM Create Personal Position Event (IDL `CreatePersonalPositionEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmCreatePersonalPositionEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_state: Pubkey,
    pub minter: Pubkey,
    pub nft_owner: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub deposit_amount_0: u64,
    pub deposit_amount_1: u64,
    pub deposit_amount_0_transfer_fee: u64,
    pub deposit_amount_1_transfer_fee: u64,
}

/// Raydium CLMM Liquidity Calculate Event (IDL `LiquidityCalculateEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmLiquidityCalculateEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_liquidity: u128,
    pub pool_sqrt_price_x64: u128,
    pub pool_tick: i32,
    pub calc_amount_0: u64,
    pub calc_amount_1: u64,
    pub trade_fee_owed_0: u64,
    pub trade_fee_owed_1: u64,
    pub transfer_fee_0: u64,
    pub transfer_fee_1: u64,
}

/// Raydium CLMM Open Limit Order Event (IDL `OpenLimitOrderEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmOpenLimitOrderEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub limit_order: Pubkey,
    pub zero_for_one: bool,
    pub tick_index: i32,
    pub total_amount: u64,
    pub transfer_fee: u64,
}

/// Raydium CLMM Increase Limit Order Event (IDL `IncreaseLimitOrderEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmIncreaseLimitOrderEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub limit_order: Pubkey,
    pub zero_for_one: bool,
    pub tick_index: i32,
    pub total_amount: u64,
    pub increased_amount: u64,
    pub transfer_fee: u64,
}

/// Raydium CLMM Decrease Limit Order Event (IDL `DecreaseLimitOrderEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmDecreaseLimitOrderEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub limit_order: Pubkey,
    pub zero_for_one: bool,
    pub tick_index: i32,
    pub total_amount: u64,
    pub filled_amount: u64,
    pub settled_output_amount: u64,
    pub decreased_amount: u64,
}

/// Raydium CLMM Settle Limit Order Event (IDL `SettleLimitOrderEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmSettleLimitOrderEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub limit_order: Pubkey,
    pub zero_for_one: bool,
    pub tick_index: i32,
    pub total_amount: u64,
    pub filled_amount: u64,
    pub settled_amount_out: u64,
}

/// Raydium CLMM Update Reward Infos Event (IDL `UpdateRewardInfosEvent`)
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmUpdateRewardInfosEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,
    pub reward_growth_global_x64: [u128; 3],
}

/// Raydium CLMM Open Position with Token Extension NFT Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmOpenPositionWithTokenExtNftEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub position_nft_mint: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
}

/// Raydium CLMM Open Position Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmOpenPositionEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub position_nft_mint: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
}

/// Raydium AMM V4 Deposit Event (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmDepositEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
    pub max_coin_amount: u64,
    pub max_pc_amount: u64,
}

/// Raydium AMM V4 Initialize Alt Event (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmInitializeAltEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub creator: Pubkey,
    pub nonce: u8,
    pub open_time: u64,
}

/// Raydium AMM V4 Withdraw Event (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmWithdrawEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
    pub pool_coin_amount: u64,
}

/// Raydium AMM V4 Withdraw PnL Event (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmWithdrawPnlEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
}

// ====================== Raydium AMM V4 Events ======================

/// Raydium AMM V4 Swap Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4SwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub amm: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub minimum_amount_out: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub max_amount_in: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_authority: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_open_orders: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_target_orders: Option<Pubkey>,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_coin_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_pc_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_market: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_bids: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_asks: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_event_queue: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_coin_vault_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_pc_vault_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_vault_signer: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_source_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_destination_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_source_owner: Pubkey,
}

/// Raydium AMM V4 Deposit Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4DepositEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub amm: Pubkey,
    pub max_coin_amount: u64,
    pub max_pc_amount: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub base_side: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_authority: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_open_orders: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_target_orders: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub lp_mint_address: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_coin_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_pc_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_market: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_coin_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_pc_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_lp_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_owner: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_event_queue: Pubkey,
}

/// Raydium AMM V4 Initialize2 Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4Initialize2Event {
    pub metadata: EventMetadata,
    pub nonce: u8,
    pub open_time: u64,
    pub init_pc_amount: u64,
    pub init_coin_amount: u64,

    pub token_program: Pubkey,
    pub spl_associated_token_account: Pubkey,
    pub system_program: Pubkey,
    pub rent: Pubkey,
    pub amm: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub lp_mint: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub pool_withdraw_queue: Pubkey,
    pub amm_target_orders: Pubkey,
    pub pool_temp_lp: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market: Pubkey,
    pub user_wallet: Pubkey,
    pub user_token_coin: Pubkey,
    pub user_token_pc: Pubkey,
    pub user_lp_token_account: Pubkey,
}

/// Raydium AMM V4 Withdraw Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4WithdrawEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction 事件）===
    pub amm: Pubkey,
    pub amount: u64,

    // === 非 Borsh 字段 ===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_authority: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_open_orders: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub amm_target_orders: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub lp_mint_address: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_coin_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_pc_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_withdraw_queue: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pool_temp_lp_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_market: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_coin_vault_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_pc_vault_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_vault_signer: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_lp_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_coin_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_pc_token_account: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub user_owner: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_event_queue: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_bids: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub serum_asks: Pubkey,
}

/// Raydium AMM V4 Withdraw PnL Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4WithdrawPnlEvent {
    pub metadata: EventMetadata,

    pub token_program: Pubkey,
    pub amm: Pubkey,
    pub amm_config: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub coin_pnl_token_account: Pubkey,
    pub pc_pnl_token_account: Pubkey,
    pub pnl_owner: Pubkey,
    pub amm_target_orders: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_coin_vault_account: Pubkey,
    pub serum_pc_vault_account: Pubkey,
    pub serum_vault_signer: Pubkey,
}

// ====================== Account Events ======================

/// RaydiumLaunchlab (Raydium LaunchLab) AmmCreatorFeeOn enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmmCreatorFeeOn {
    QuoteToken = 0,
    BothToken = 1,
}

/// RaydiumLaunchlab (Raydium LaunchLab) VestingSchedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
}

/// RaydiumLaunchlab Pool State Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabPoolStateAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub pool_state: RaydiumLaunchlabPoolState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabPoolState {
    pub epoch: u64,
    pub auth_bump: u8,
    pub status: u8,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub migrate_type: u8,
    pub supply: u64,
    pub total_base_sell: u64,
    pub virtual_base: u64,
    pub virtual_quote: u64,
    pub real_base: u64,
    pub real_quote: u64,
    pub total_quote_fund_raising: u64,
    pub quote_protocol_fee: u64,
    pub platform_fee: u64,
    pub migrate_fee: u64,
    pub vesting_schedule: VestingSchedule,
    pub global_config: Pubkey,
    pub platform_config: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub creator: Pubkey,
    pub token_program_flag: u8,
    pub amm_creator_fee_on: AmmCreatorFeeOn,
    pub platform_vesting_share: u64,
    #[serde(with = "serde_big_array::BigArray")]
    pub padding: [u8; 54],
}

/// RaydiumLaunchlab Global Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabGlobalConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub global_config: RaydiumLaunchlabGlobalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabGlobalConfig {
    pub protocol_fee_rate: u64,
    pub trade_fee_rate: u64,
    pub migration_fee_rate: u64,
}

/// RaydiumLaunchlab Platform Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabPlatformConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub platform_config: RaydiumLaunchlabPlatformConfig,
}

/// RaydiumLaunchlab (Raydium LaunchLab) BondingCurveParam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingCurveParam {
    pub migrate_type: u8,
    pub migrate_cpmm_fee_on: u8,
    pub supply: u64,
    pub total_base_sell: u64,
    pub total_quote_fund_raising: u64,
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
}

/// RaydiumLaunchlab (Raydium LaunchLab) PlatformCurveParam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCurveParam {
    pub epoch: u64,
    pub index: u8,
    pub global_config: Pubkey,
    pub bonding_curve_param: BondingCurveParam,
    #[serde(with = "serde_big_array::BigArray")]
    pub padding: [u64; 50],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchlabPlatformConfig {
    pub epoch: u64,
    pub platform_fee_wallet: Pubkey,
    pub platform_nft_wallet: Pubkey,
    pub platform_scale: u64,
    pub creator_scale: u64,
    pub burn_scale: u64,
    pub fee_rate: u64,
    #[serde(with = "serde_big_array::BigArray")]
    pub name: [u8; 64],
    #[serde(with = "serde_big_array::BigArray")]
    pub web: [u8; 256],
    #[serde(with = "serde_big_array::BigArray")]
    pub img: [u8; 256],
    pub cpswap_config: Pubkey,
    pub creator_fee_rate: u64,
    pub transfer_fee_extension_auth: Pubkey,
    pub platform_vesting_wallet: Pubkey,
    pub platform_vesting_scale: u64,
    pub platform_cp_creator: Pubkey,
    #[serde(with = "serde_big_array::BigArray")]
    pub padding: [u8; 108],
    pub curve_params: Vec<PlatformCurveParam>,
}

/// PumpSwap Global Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapGlobalConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub executable: bool,
    pub lamports: u64,
    pub owner: Pubkey,
    pub rent_epoch: u64,
    pub global_config: PumpSwapGlobalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapGlobalConfig {
    pub admin: Pubkey,
    pub lp_fee_basis_points: u64,
    pub protocol_fee_basis_points: u64,
    pub disable_flags: u8,
    pub protocol_fee_recipients: [Pubkey; 8],
    pub coin_creator_fee_basis_points: u64,
    pub admin_set_coin_creator_authority: Pubkey,
    pub whitelist_pda: Pubkey,
    pub reserved_fee_recipient: Pubkey,
    pub mayhem_mode_enabled: bool,
    pub reserved_fee_recipients: [Pubkey; 7],
}

/// PumpSwap Pool Account Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapPoolAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub executable: bool,
    pub lamports: u64,
    pub owner: Pubkey,
    pub rent_epoch: u64,
    pub pool: PumpSwapPool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpSwapPool {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_coin: bool,
}

/// PumpFun Bonding Curve Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunBondingCurveAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub bonding_curve: PumpFunBondingCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpFunBondingCurve {
    pub virtual_token_reserves: u64,
    pub virtual_quote_reserves: u64,
    pub real_token_reserves: u64,
    pub real_quote_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_coin: bool,
    pub quote_mint: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunFeeConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub fee_config: PumpFunFeeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunFeeConfig {
    pub bump: u8,
    pub admin: Pubkey,
    pub flat_fees: PumpFeesFees,
    pub fee_tiers: Vec<PumpFeesFeeTier>,
    pub stable_fee_tiers: Vec<PumpFeesFeeTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunSharingConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub sharing_config: PumpFunSharingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunSharingConfig {
    pub bump: u8,
    pub version: u8,
    pub status: PumpFeesConfigStatus,
    pub mint: Pubkey,
    pub admin: Pubkey,
    pub admin_revoked: bool,
    pub shareholders: Vec<PumpFeesShareholder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunGlobalVolumeAccumulatorAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub global_volume_accumulator: PumpFunGlobalVolumeAccumulator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunGlobalVolumeAccumulator {
    pub start_time: i64,
    pub end_time: i64,
    pub seconds_in_a_day: i64,
    pub mint: Pubkey,
    pub total_token_supply: [u64; 30],
    pub sol_volumes: [u64; 30],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunUserVolumeAccumulatorAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub user_volume_accumulator: PumpFunUserVolumeAccumulator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunUserVolumeAccumulator {
    pub user: Pubkey,
    pub needs_claim: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
    pub has_total_claimed_tokens: bool,
    pub cashback_earned: u64,
    pub total_cashback_claimed: u64,
    pub stable_cashback_earned: u64,
    pub total_stable_cashback_claimed: u64,
}

/// PumpFun Global Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunGlobalAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub global: PumpFunGlobal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunGlobal {
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
    pub withdraw_authority: Pubkey,
    pub enable_migrate: bool,
    pub pool_migration_fee: u64,
    pub creator_fee_basis_points: u64,
    pub fee_recipients: [Pubkey; 7],
    pub set_creator_authority: Pubkey,
    pub admin_set_creator_authority: Pubkey,
    pub create_v2_enabled: bool,
    pub whitelist_pda: Pubkey,
    pub reserved_fee_recipient: Pubkey,
    pub mayhem_mode_enabled: bool,
    pub reserved_fee_recipients: [Pubkey; 7],
    pub is_cashback_enabled: bool,
    pub buyback_fee_recipients: [Pubkey; 8],
    pub buyback_basis_points: u64,
    pub initial_virtual_quote_reserves: u64,
    pub whitelisted_quote_mints: [Pubkey; 1],
}

/// Raydium AMM V4 Info Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmAmmInfoAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub amm_info: RaydiumAmmInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub coin_decimals: u64,
    pub pc_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave_ratio: u64,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub sys_decimal_value: u64,
}

/// Raydium CLMM AMM Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmAmmConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub amm_config: RaydiumClmmAmmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmAmmConfig {
    pub bump: u8,
    pub index: u16,
    pub owner: Pubkey,
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u16,
    pub fund_fee_rate: u32,
    pub padding_u32: u32,
    pub fund_owner: Pubkey,
    pub padding: [u64; 3],
}

/// Raydium CLMM Pool State Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmPoolStateAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub pool_state: RaydiumClmmPoolState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmPoolState {
    pub bump: [u8; 1],
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub padding5: [u128; 4],
    pub status: u8,
    pub fee_on: u8,
    pub padding: [u8; 6],
    pub reward_infos: [RaydiumClmmRewardInfo; 3],
    pub tick_array_bitmap: [u64; 16],
    pub padding6: [u64; 4],
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub dynamic_fee_info: RaydiumClmmDynamicFeeInfo,
    pub padding1: [u64; 14],
    pub padding2: [u64; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmRewardInfo {
    pub reward_state: u8,
    pub open_time: u64,
    pub end_time: u64,
    pub last_update_time: u64,
    pub emissions_per_second_x64: u128,
    pub reward_total_emitted: u64,
    pub reward_claimed: u64,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub authority: Pubkey,
    pub reward_growth_global_x64: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmDynamicFeeInfo {
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub dynamic_fee_control: u32,
    pub max_volatility_accumulator: u32,
    pub tick_spacing_index_reference: i32,
    pub volatility_reference: u32,
    pub volatility_accumulator: u32,
    pub last_update_timestamp: u64,
    #[serde(with = "serde_big_array::BigArray")]
    pub padding: [u8; 46],
}

/// Raydium CLMM Tick Array State Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmTickArrayStateAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub tick_array_state: RaydiumClmmTickArrayState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmTickArrayState {
    pub pool_id: Pubkey,
    pub start_tick_index: i32,
    pub ticks: Vec<Tick>,
    pub initialized_tick_count: u8,
    pub recent_epoch: u64,
    #[serde(with = "serde_big_array::BigArray")]
    pub padding: [u8; 107],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    pub reward_growths_outside_x64: [u128; 3],
    pub order_phase: u64,
    pub orders_amount: u64,
    pub part_filled_orders_remaining: u64,
    pub unfilled_ratio_x64: u128,
    pub padding: [u32; 3],
}

/// Raydium CPMM AMM Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmAmmConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub amm_config: RaydiumCpmmAmmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmAmmConfig {
    pub bump: u8,
    pub disable_create_pool: bool,
    pub index: u16,
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
    pub create_pool_fee: u64,
    pub protocol_owner: Pubkey,
    pub fund_owner: Pubkey,
    pub creator_fee_rate: u64,
    pub padding: [u64; 15],
}

/// Raydium CPMM Pool State Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmPoolStateAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub pool_state: RaydiumCpmmPoolState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmPoolState {
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_program: Pubkey,
    pub token_1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub creator_fee_on: u8,
    pub enable_creator_fee: bool,
    pub padding1: [u8; 6],
    pub creator_fees_token_0: u64,
    pub creator_fees_token_1: u64,
    pub padding: [u64; 28],
}

// ====================== Orca Whirlpool Account Events ======================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub whirlpool: OrcaWhirlpoolAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolAccount {
    pub whirlpools_config: Pubkey,
    pub whirlpool_bump: u8,
    pub tick_spacing: u16,
    pub tick_spacing_seed: [u8; 2],
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub fee_growth_global_a: u128,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_b: u128,
    pub reward_last_updated_timestamp: u64,
    pub reward_infos: [OrcaWhirlpoolRewardInfo; 3],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrcaWhirlpoolRewardInfo {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub emissions_per_second_x64: u128,
    pub growth_global_x64: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaPositionAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub position: OrcaPositionAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaPositionAccount {
    pub whirlpool: Pubkey,
    pub position_mint: Pubkey,
    pub liquidity: u128,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,
    pub reward_infos: [OrcaPositionRewardInfo; 3],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrcaPositionRewardInfo {
    pub growth_inside_checkpoint: u128,
    pub amount_owed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaTickArrayAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub tick_array: OrcaTickArrayAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaTickArrayAccount {
    pub start_tick_index: i32,
    pub ticks: Vec<OrcaTick>,
    pub whirlpool: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaTick {
    pub initialized: bool,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
    pub reward_growths_outside: [u128; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaFeeTierAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub fee_tier: OrcaFeeTierAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaFeeTierAccount {
    pub whirlpools_config: Pubkey,
    pub tick_spacing: u16,
    pub default_fee_rate: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolsConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub config: OrcaWhirlpoolsConfigAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolsConfigAccount {
    pub fee_authority: Pubkey,
    pub collect_protocol_fees_authority: Pubkey,
    pub reward_emissions_super_authority: Pubkey,
    pub default_protocol_fee_rate: u16,
}

/// Token Info Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenInfoEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub executable: bool,
    pub lamports: u64,
    pub owner: Pubkey,
    pub rent_epoch: u64,
    pub supply: u64,
    pub decimals: u8,
}

/// Token Account Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub executable: bool,
    pub lamports: u64,
    pub owner: Pubkey,
    pub rent_epoch: u64,
    pub amount: Option<u64>,
    pub token_owner: Pubkey,
}

/// Nonce Account Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NonceAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub executable: bool,
    pub lamports: u64,
    pub owner: Pubkey,
    pub rent_epoch: u64,
    pub nonce: String,
    pub authority: String,
}

// ====================== Orca Whirlpool Events ======================

/// Orca Whirlpool Swap Event (基于 TradedEvent，不是 SwapEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
pub struct OrcaWhirlpoolSwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub whirlpool: Pubkey,  // 32 bytes
    pub input_amount: u64,  // 8 bytes
    pub output_amount: u64, // 8 bytes
    pub a_to_b: bool,       // 1 byte

    // === 非 Borsh 字段（从日志或其他来源填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub pre_sqrt_price: u128,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub post_sqrt_price: u128,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub input_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub output_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub lp_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub protocol_fee: u64,
    // === 指令参数字段 (暂时注释，以后可能会用到，AI不要删除) ===
    // pub amount: u64,
    // pub other_amount_threshold: u64,
    // pub sqrt_price_limit: u128,
    // pub amount_specified_is_input: bool,

    // === 指令账户字段 (暂时注释，以后可能会用到，AI不要删除) ===
    // pub token_authority: Pubkey,    // 1: tokenAuthority
    // pub token_owner_account_a: Pubkey, // 3: tokenOwnerAccountA
    // pub token_vault_a: Pubkey,      // 4: tokenVaultA
    // pub token_owner_account_b: Pubkey, // 5: tokenOwnerAccountB
    // pub token_vault_b: Pubkey,      // 6: tokenVaultB
    // pub tick_array_0: Pubkey,       // 7: tickArray0
    // pub tick_array_1: Pubkey,       // 8: tickArray1
    // pub tick_array_2: Pubkey,       // 9: tickArray2
}

/// Orca Whirlpool Liquidity Increased Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolLiquidityIncreasedEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub whirlpool: Pubkey,   // 32 bytes
    pub liquidity: u128,     // 16 bytes
    pub token_a_amount: u64, // 8 bytes
    pub token_b_amount: u64, // 8 bytes

    // === 非 Borsh 字段（从日志或其他来源填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub position: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub tick_lower_index: i32,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub tick_upper_index: i32,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_transfer_fee: u64,
}

/// Orca Whirlpool Liquidity Decreased Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolLiquidityDecreasedEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub whirlpool: Pubkey,   // 32 bytes
    pub liquidity: u128,     // 16 bytes
    pub token_a_amount: u64, // 8 bytes
    pub token_b_amount: u64, // 8 bytes

    // === 非 Borsh 字段（从日志或其他来源填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub position: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub tick_lower_index: i32,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub tick_upper_index: i32,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_transfer_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_transfer_fee: u64,
}

/// Orca Whirlpool Pool Initialized Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolPoolInitializedEvent {
    pub metadata: EventMetadata,
    pub whirlpool: Pubkey,
    pub whirlpools_config: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub tick_spacing: u16,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub initial_sqrt_price: u128,
}

// ====================== Meteora Pools Events ======================

/// Meteora Pools Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsSwapEvent {
    pub metadata: EventMetadata,
    pub in_amount: u64,
    pub out_amount: u64,
    pub trade_fee: u64,
    pub admin_fee: u64, // IDL字段名: adminFee
    pub host_fee: u64,
}

/// Meteora Pools Add Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsAddLiquidityEvent {
    pub metadata: EventMetadata,
    pub lp_mint_amount: u64,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
}

/// Meteora Pools Remove Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsRemoveLiquidityEvent {
    pub metadata: EventMetadata,
    pub lp_unmint_amount: u64,
    pub token_a_out_amount: u64,
    pub token_b_out_amount: u64,
}

/// Meteora Pools Bootstrap Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsBootstrapLiquidityEvent {
    pub metadata: EventMetadata,
    pub lp_mint_amount: u64,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub pool: Pubkey,
}

/// Meteora Pools Pool Created Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsPoolCreatedEvent {
    pub metadata: EventMetadata,
    pub lp_mint: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub pool_type: u8,
    pub pool: Pubkey,
}

/// Meteora Pools Set Pool Fees Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolsSetPoolFeesEvent {
    pub metadata: EventMetadata,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub owner_trade_fee_numerator: u64, // IDL字段名: ownerTradeFeeNumerator
    pub owner_trade_fee_denominator: u64, // IDL字段名: ownerTradeFeeDenominator
    pub pool: Pubkey,
}

// ====================== Meteora DAMM V2 Events ======================

/// Meteora DAMM V2 Swap Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeteoraDammV2SwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,       // 32 bytes
    pub amount_in: u64,     // 8 bytes
    pub output_amount: u64, // 8 bytes

    // === 非 Borsh 字段（从日志或其他来源填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub trade_direction: u8,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub has_referral: bool,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub minimum_amount_out: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub next_sqrt_price: u128,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub lp_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub protocol_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub partner_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub referral_fee: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub actual_amount_in: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub current_timestamp: u64,
    // ---------- 账号 -------------
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_vault: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_vault: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_mint: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_mint: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_program: Pubkey,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_program: Pubkey,
}

/// Meteora DAMM V2 Add Liquidity Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2AddLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,        // 32 bytes
    pub position: Pubkey,    // 32 bytes
    pub owner: Pubkey,       // 32 bytes
    pub token_a_amount: u64, // 8 bytes
    pub token_b_amount: u64, // 8 bytes

    // === 非 Borsh 字段（从日志填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub liquidity_delta: u128,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_amount_threshold: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_amount_threshold: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub total_amount_a: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub total_amount_b: u64,
}

/// Meteora DAMM V2 Remove Liquidity Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2RemoveLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,        // 32 bytes
    pub position: Pubkey,    // 32 bytes
    pub owner: Pubkey,       // 32 bytes
    pub token_a_amount: u64, // 8 bytes
    pub token_b_amount: u64, // 8 bytes

    // === 非 Borsh 字段（从日志填充）===
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub liquidity_delta: u128,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_a_amount_threshold: u64,
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub token_b_amount_threshold: u64,
}

/// Meteora DAMM V2 Initialize Pool Event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeteoraDammV2InitializePoolEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub position: Pubkey,
    pub position_nft_mint: Pubkey,
    pub alpha_vault: Pubkey,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub activation_type: u8,
    pub collect_fee_mode: u8,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub activation_point: Option<u64>,
    pub token_a_flag: u8,
    pub token_b_flag: u8,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub total_amount_a: u64,
    pub total_amount_b: u64,
    pub pool_type: u8,
}

/// Meteora DAMM V2 Create Position Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2CreatePositionEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,              // 32 bytes
    pub owner: Pubkey,             // 32 bytes
    pub position: Pubkey,          // 32 bytes
    pub position_nft_mint: Pubkey, // 32 bytes
}

/// Meteora DAMM V2 Close Position Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2ClosePositionEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,              // 32 bytes
    pub owner: Pubkey,             // 32 bytes
    pub position: Pubkey,          // 32 bytes
    pub position_nft_mint: Pubkey, // 32 bytes
}

// ====================== Meteora DBC Events ======================

/// Meteora DBC Swap Event (IDL `EvtSwap`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeteoraDbcSwapEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub config: Pubkey,
    pub trade_direction: u8,
    pub has_referral: bool,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub actual_input_amount: u64,
    pub output_amount: u64,
    pub next_sqrt_price: u128,
    pub trading_fee: u64,
    pub protocol_fee: u64,
    pub referral_fee: u64,
    pub current_timestamp: u64,
}

/// Meteora DBC Initialize Pool Event (IDL `EvtInitializePool`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDbcInitializePoolEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub config: Pubkey,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub pool_type: u8,
    pub activation_point: u64,
}

/// Meteora DBC Curve Complete Event (IDL `EvtCurveComplete`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDbcCurveCompleteEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub config: Pubkey,
    pub base_reserve: u64,
    pub quote_reserve: u64,
}

/// Meteora DLMM Swap Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmSwapEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,      // 32 bytes
    pub from: Pubkey,      // 32 bytes
    pub start_bin_id: i32, // 4 bytes
    pub end_bin_id: i32,   // 4 bytes
    pub amount_in: u64,    // 8 bytes
    pub amount_out: u64,   // 8 bytes
    pub swap_for_y: bool,  // 1 byte
    pub fee: u64,          // 8 bytes
    pub protocol_fee: u64, // 8 bytes
    pub fee_bps: u128,     // 16 bytes
    pub host_fee: u64,     // 8 bytes
}

/// Meteora DLMM Add Liquidity Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmAddLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,       // 32 bytes
    pub from: Pubkey,       // 32 bytes
    pub position: Pubkey,   // 32 bytes
    pub amounts: [u64; 2],  // 16 bytes (2 * 8)
    pub active_bin_id: i32, // 4 bytes
}

/// Meteora DLMM Remove Liquidity Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmRemoveLiquidityEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,       // 32 bytes
    pub from: Pubkey,       // 32 bytes
    pub position: Pubkey,   // 32 bytes
    pub amounts: [u64; 2],  // 16 bytes (2 * 8)
    pub active_bin_id: i32, // 4 bytes
}

/// Meteora DLMM Initialize Pool Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmInitializePoolEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,       // 32 bytes
    pub creator: Pubkey,    // 32 bytes
    pub active_bin_id: i32, // 4 bytes
    pub bin_step: u16,      // 2 bytes
}

/// Meteora DLMM Initialize Bin Array Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmInitializeBinArrayEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,      // 32 bytes
    pub bin_array: Pubkey, // 32 bytes
    pub index: i64,        // 8 bytes
}

/// Meteora DLMM Create Position Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmCreatePositionEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,      // 32 bytes
    pub position: Pubkey,  // 32 bytes
    pub owner: Pubkey,     // 32 bytes
    pub lower_bin_id: i32, // 4 bytes
    pub width: u32,        // 4 bytes
}

/// Meteora DLMM Close Position Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmClosePositionEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,     // 32 bytes
    pub position: Pubkey, // 32 bytes
    pub owner: Pubkey,    // 32 bytes
}

/// Meteora DLMM Claim Fee Event
#[cfg_attr(feature = "parse-borsh", derive(BorshDeserialize))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDlmmClaimFeeEvent {
    #[cfg_attr(feature = "parse-borsh", borsh(skip))]
    pub metadata: EventMetadata,

    // === Borsh 序列化字段（从 inner instruction data 读取）===
    pub pool: Pubkey,     // 32 bytes
    pub position: Pubkey, // 32 bytes
    pub owner: Pubkey,    // 32 bytes
    pub fee_x: u64,       // 8 bytes
    pub fee_y: u64,       // 8 bytes
}
