//! 所有具体的事件类型定义
//!
//! 基于您提供的回调事件列表，定义所有需要的具体事件类型

use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 基础元数据 - 所有事件共享的字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<Timestamp>,
    pub block_time_ms: Option<i64>,
    pub program_id: Pubkey,
    pub outer_index: i64,
    pub inner_index: Option<i64>,
    pub transaction_index: Option<u64>,
    pub recv_us: i64,
    pub handle_us: i64,
}

/// Block Meta Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetaEvent {
    pub metadata: EventMetadata,
}

/// Bonk Pool Create Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkPoolCreateEvent {
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

/// Bonk Trade Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkTradeEvent {
    pub metadata: EventMetadata,
    pub pool_state: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_buy: bool,
    pub trade_direction: TradeDirection,
    pub exact_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeDirection {
    Buy,
    Sell,
}

/// Bonk Migrate AMM Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkMigrateAmmEvent {
    pub metadata: EventMetadata,
    pub old_pool: Pubkey,
    pub new_pool: Pubkey,
    pub user: Pubkey,
    pub liquidity_amount: u64,
}

/// PumpFun Trade Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunTradeEvent {
    pub metadata: EventMetadata,
    pub mint: Pubkey,
    pub user: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub bonding_curve: Pubkey,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}

/// PumpFun Complete Token Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunCompleteTokenEvent {
    pub metadata: EventMetadata,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
}

/// PumpFun Create Token Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunCreateTokenEvent {
    pub metadata: EventMetadata,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
}

/// PumpSwap Buy Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapBuyEvent {
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub price: u64,
    pub slippage: u16,
}

/// PumpSwap Sell Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapSellEvent {
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub token_amount: u64,
    pub sol_amount: u64,
    pub price: u64,
    pub slippage: u16,
}

/// PumpSwap Create Pool Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapCreatePoolEvent {
    pub metadata: EventMetadata,
    pub pool_id: Pubkey,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub initial_sol_amount: u64,
    pub initial_token_amount: u64,
    pub fee_rate: u16,
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

/// Raydium CPMM Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmSwapEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_base_input: bool,
}

/// Raydium CPMM Deposit Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmDepositEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub lp_token_amount: u64,
    pub token0_amount: u64,
    pub token1_amount: u64,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmWithdrawEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub lp_token_amount: u64,
    pub token0_amount: u64,
    pub token1_amount: u64,
}

/// Raydium CLMM Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmSwapEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: u128,
    pub is_base_input: bool,
}

/// Raydium CLMM Close Position Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmClosePositionEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub position_nft_mint: Pubkey,
}

/// Raydium CLMM Decrease Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmDecreaseLiquidityEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub liquidity: u128,
    pub amount0_min: u64,
    pub amount1_min: u64,
}

/// Raydium CLMM Create Pool Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmCreatePoolEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub creator: Pubkey,
    pub sqrt_price_x64: u128,
    pub open_time: u64,
}

/// Raydium CLMM Increase Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmIncreaseLiquidityEvent {
    pub metadata: EventMetadata,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub liquidity: u128,
    pub amount0_max: u64,
    pub amount1_max: u64,
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

/// Raydium AMM Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmSwapEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub direction: u8, // 0: base to quote, 1: quote to base
}

/// Raydium AMM Deposit Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmDepositEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
    pub max_coin_amount: u64,
    pub max_pc_amount: u64,
}

/// Raydium AMM Initialize Alt Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmInitializeAltEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub creator: Pubkey,
    pub nonce: u8,
    pub open_time: u64,
}

/// Raydium AMM Withdraw Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmWithdrawEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
    pub pool_coin_amount: u64,
}

/// Raydium AMM Withdraw PnL Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmWithdrawPnlEvent {
    pub metadata: EventMetadata,
    pub amm_id: Pubkey,
    pub user: Pubkey,
}

// ====================== Account Events ======================

/// Bonk Pool State Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkPoolStateAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub pool_state: BonkPoolState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkPoolState {
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub virtual_base: u64,
    pub virtual_quote: u64,
    pub real_base: u64,
    pub real_quote: u64,
}

/// Bonk Global Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkGlobalConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub global_config: BonkGlobalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkGlobalConfig {
    pub protocol_fee_rate: u64,
    pub trade_fee_rate: u64,
    pub migration_fee_rate: u64,
}

/// Bonk Platform Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkPlatformConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub platform_config: BonkPlatformConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonkPlatformConfig {
    pub fee_recipient: Pubkey,
    pub fee_rate: u64,
}

/// PumpSwap Global Config Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapGlobalConfigAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub global_config: PumpSwapGlobalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapGlobalConfig {
    pub fee_recipient: Pubkey,
    pub fee_rate: u64,
}

/// PumpSwap Pool Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPoolAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub pool: PumpSwapPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPool {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_reserves: u64,
    pub quote_reserves: u64,
}

/// PumpFun Bonding Curve Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunBondingCurveAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub bonding_curve: PumpFunBondingCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunBondingCurve {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
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
    pub discriminator: u64,
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

/// Raydium AMM AMM Info Account Event
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
    pub fund_owner: Pubkey,
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
    pub token_mint0: Pubkey,
    pub token_mint1: Pubkey,
    pub token_vault0: Pubkey,
    pub token_vault1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals0: u8,
    pub mint_decimals1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
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
    pub discriminator: u64,
    pub pool_id: Pubkey,
    pub start_tick_index: i32,
    pub ticks: Vec<Tick>,
    pub initialized_tick_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    pub reward_growths_outside_x64: [u128; 3],
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
    pub token0_vault: Pubkey,
    pub token1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    pub token0_program: Pubkey,
    pub token1_program: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint0_decimals: u8,
    pub mint1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
}

/// Token Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: u8,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
}

/// Nonce Account Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceAccountEvent {
    pub metadata: EventMetadata,
    pub pubkey: Pubkey,
    pub authority: Pubkey,
    pub nonce: String,
    pub fee_calculator: FeeCalculator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculator {
    pub lamports_per_signature: u64,
}

/// Token Info Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfoEvent {
    pub metadata: EventMetadata,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub supply: u64,
}

// ====================== 统一的 DEX 事件枚举 ======================

/// 统一的 DEX 事件枚举 - 参考 sol-dex-shreds 的做法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DexEvent {
    // PumpFun 事件
    PumpFunCreate(PumpFunCreateTokenEvent),
    PumpFunTrade(PumpFunTradeEvent),
    PumpFunComplete(PumpFunCompleteTokenEvent),

    // Bonk 事件
    BonkTrade(BonkTradeEvent),
    BonkPoolCreate(BonkPoolCreateEvent),
    BonkMigrateAmm(BonkMigrateAmmEvent),

    // PumpSwap 事件
    PumpSwapBuy(PumpSwapBuyEvent),
    PumpSwapSell(PumpSwapSellEvent),
    PumpSwapCreatePool(PumpSwapCreatePoolEvent),

    // Raydium CLMM 事件
    RaydiumClmmSwap(RaydiumClmmSwapEvent),
    RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent),
    RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent),
    RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent),
    RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent),
    RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent),

    // Raydium CPMM 事件
    RaydiumCpmmSwap(RaydiumCpmmSwapEvent),
    RaydiumCpmmDeposit(RaydiumCpmmDepositEvent),
    RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent),
    RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent),

    // 账户事件
    TokenAccount(TokenAccountEvent),
    NonceAccount(NonceAccountEvent),

    // 区块元数据事件
    BlockMeta(BlockMetaEvent),

    // Token 信息事件
    TokenInfo(TokenInfoEvent),

    // 错误事件
    Error(String),
}