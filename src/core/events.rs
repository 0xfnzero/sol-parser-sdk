//! 所有具体的事件类型定义
//!
//! 基于您提供的回调事件列表，定义所有需要的具体事件类型

// use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// 基础元数据 - 所有事件共享的字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
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

/// PumpFun Trade Event - 基于官方IDL定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunTradeEvent {
    pub metadata: EventMetadata,
    // IDL定义的核心字段
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
    // 指令解析补充字段
    pub bonding_curve: Pubkey,
    pub max_sol_cost: u64,
    pub min_sol_output: u64,
    pub amount: u64,
    // PumpFun 相关账户信息（有意义的账户）
    pub global: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub associated_user: Pubkey,
    pub creator_vault: Pubkey,
    pub event_authority: Pubkey,
    pub global_volume_accumulator: Pubkey,
    pub user_volume_accumulator: Pubkey,
}

/// PumpFun Complete Token Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunCompleteTokenEvent {
    pub metadata: EventMetadata,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
}

/// PumpFun Migrate Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunMigrateEvent {
    pub metadata: EventMetadata,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub mint_amount: u64,
    pub sol_amount: u64,
    pub pool_migration_fee: u64,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
    pub pool: Pubkey,
    // 额外账户信息（用于指令解析）
    pub global: Pubkey,
    pub withdraw_authority: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub system_program: Pubkey,
    pub token_program: Pubkey,
    pub pump_amm: Pubkey,
    pub pool_authority: Pubkey,
    pub pool_authority_mint_account: Pubkey,
    pub pool_authority_wsol_account: Pubkey,
    pub amm_global_config: Pubkey,
    pub wsol_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub user_pool_token_account: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub token_2022_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub pump_amm_event_authority: Pubkey,
    pub event_authority: Pubkey,
    pub program: Pubkey,
}

/// PumpFun Create Token Event - 精简版本，只包含实际可获取的有用数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunCreateTokenEvent {
    pub metadata: EventMetadata,
    // 代币基本信息（从instruction获取）
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    // 储备量信息（从logs获取）
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
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

/// PumpSwap Pool Created Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPoolCreated {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
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

/// PumpSwap Trade Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapTrade {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
    pub pool_account: Pubkey,
    pub user: Pubkey,
    pub user_token_in_account: Pubkey,
    pub user_token_out_account: Pubkey,
    pub pool_token_in_vault: Pubkey,
    pub pool_token_out_vault: Pubkey,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub is_token_a_to_b: bool,
}

/// PumpSwap Liquidity Added Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapLiquidityAdded {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
    pub pool_account: Pubkey,
    pub user: Pubkey,
    pub user_token_a_account: Pubkey,
    pub user_token_b_account: Pubkey,
    pub user_lp_token_account: Pubkey,
    pub pool_token_a_vault: Pubkey,
    pub pool_token_b_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub max_token_a_amount: u64,
    pub max_token_b_amount: u64,
    pub min_lp_tokens: u64,
}

/// PumpSwap Liquidity Removed Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapLiquidityRemoved {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
    pub pool_account: Pubkey,
    pub user: Pubkey,
    pub user_token_a_account: Pubkey,
    pub user_token_b_account: Pubkey,
    pub user_lp_token_account: Pubkey,
    pub pool_token_a_vault: Pubkey,
    pub pool_token_b_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub lp_tokens_to_burn: u64,
    pub min_token_a_amount: u64,
    pub min_token_b_amount: u64,
}

/// PumpSwap Pool Updated Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapPoolUpdated {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
    pub pool_account: Pubkey,
    pub authority: Pubkey,
    pub admin: Pubkey,
    pub new_fee_rate: u64,
}

/// PumpSwap Fees Claimed Event - 指令解析版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSwapFeesClaimed {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_index: u32,
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

/// Raydium CLMM Collect Fee Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumClmmCollectFeeEvent {
    pub metadata: EventMetadata,
    pub pool_state: Pubkey,
    pub position_nft_mint: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
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

// ====================== Raydium AMM V4 Events ======================

/// Raydium AMM V4 Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4SwapEvent {
    pub metadata: EventMetadata,
    // base in
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    // base out
    pub max_amount_in: u64,
    pub amount_out: u64,

    pub token_program: Pubkey,
    pub amm: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Option<Pubkey>,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market: Pubkey,
    pub serum_bids: Pubkey,
    pub serum_asks: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_coin_vault_account: Pubkey,
    pub serum_pc_vault_account: Pubkey,
    pub serum_vault_signer: Pubkey,
    pub user_source_token_account: Pubkey,
    pub user_destination_token_account: Pubkey,
    pub user_source_owner: Pubkey,
}

/// Raydium AMM V4 Deposit Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4DepositEvent {
    pub metadata: EventMetadata,
    pub max_coin_amount: u64,
    pub max_pc_amount: u64,
    pub base_side: u64,

    pub token_program: Pubkey,
    pub amm: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub lp_mint_address: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub serum_market: Pubkey,
    pub user_coin_token_account: Pubkey,
    pub user_pc_token_account: Pubkey,
    pub user_lp_token_account: Pubkey,
    pub user_owner: Pubkey,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmV4WithdrawEvent {
    pub metadata: EventMetadata,
    pub amount: u64,

    pub token_program: Pubkey,
    pub amm: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub lp_mint_address: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub pool_withdraw_queue: Pubkey,
    pub pool_temp_lp_token_account: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market: Pubkey,
    pub serum_coin_vault_account: Pubkey,
    pub serum_pc_vault_account: Pubkey,
    pub serum_vault_signer: Pubkey,
    pub user_lp_token_account: Pubkey,
    pub user_coin_token_account: Pubkey,
    pub user_pc_token_account: Pubkey,
    pub user_owner: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_bids: Pubkey,
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

// ====================== Orca Whirlpool Events ======================

/// Orca Whirlpool Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolSwapEvent {
    pub metadata: EventMetadata,
    pub whirlpool: Pubkey,
    pub a_to_b: bool,
    pub pre_sqrt_price: u128,
    pub post_sqrt_price: u128,
    pub input_amount: u64,
    pub output_amount: u64,
    pub input_transfer_fee: u64,
    pub output_transfer_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
}

/// Orca Whirlpool Liquidity Increased Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolLiquidityIncreasedEvent {
    pub metadata: EventMetadata,
    pub whirlpool: Pubkey,
    pub position: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub token_a_transfer_fee: u64,
    pub token_b_transfer_fee: u64,
}

/// Orca Whirlpool Liquidity Decreased Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolLiquidityDecreasedEvent {
    pub metadata: EventMetadata,
    pub whirlpool: Pubkey,
    pub position: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub token_a_transfer_fee: u64,
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
    pub protocol_fee: u64,
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
    pub protocol_trade_fee_numerator: u64,
    pub protocol_trade_fee_denominator: u64,
    pub pool: Pubkey,
}

// ====================== Meteora DAMM V2 Events ======================

/// Meteora DAMM V2 Swap Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2SwapEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub from: Pubkey,
    pub start_bin_id: i32,
    pub end_bin_id: i32,
    pub amount_in: u64,
    pub amount_out: u64,
    pub swap_for_y: bool,
    pub fee: u64,
    pub protocol_fee: u64,
    pub fee_bps: u128,
    pub host_fee: u64,
}

/// Meteora DAMM V2 Add Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2AddLiquidityEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub from: Pubkey,
    pub position: Pubkey,
    pub amounts: [u64; 2],
    pub active_bin_id: i32,
}

/// Meteora DAMM V2 Remove Liquidity Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2RemoveLiquidityEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub from: Pubkey,
    pub position: Pubkey,
    pub amounts: [u64; 2],
    pub active_bin_id: i32,
}

/// Meteora DAMM V2 Initialize Pool Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2InitializePoolEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub bin_step: u16,
    pub token_x: Pubkey,
    pub token_y: Pubkey,
}

/// Meteora DAMM V2 Create Position Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2CreatePositionEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
}

/// Meteora DAMM V2 Close Position Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2ClosePositionEvent {
    pub metadata: EventMetadata,
    pub position: Pubkey,
    pub owner: Pubkey,
}

/// Meteora DAMM V2 Claim Position Fee Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2ClaimPositionFeeEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub fee_x: u64,
    pub fee_y: u64,
}

/// Meteora DAMM V2 Initialize Reward Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2InitializeRewardEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub reward_mint: Pubkey,
    pub funder: Pubkey,
    pub reward_index: u64,
    pub reward_duration: u64,
}

/// Meteora DAMM V2 Fund Reward Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2FundRewardEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub funder: Pubkey,
    pub reward_index: u64,
    pub amount: u64,
}

/// Meteora DAMM V2 Claim Reward Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2ClaimRewardEvent {
    pub metadata: EventMetadata,
    pub lb_pair: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub reward_index: u64,
    pub total_reward: u64,
}

// ====================== 统一的 DEX 事件枚举 ======================

/// 统一的 DEX 事件枚举 - 参考 sol-dex-shreds 的做法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DexEvent {
    // PumpFun 事件
    PumpFunCreate(PumpFunCreateTokenEvent),
    PumpFunTrade(PumpFunTradeEvent),
    PumpFunComplete(PumpFunCompleteTokenEvent),
    PumpFunMigrate(PumpFunMigrateEvent),

    // Bonk 事件
    BonkTrade(BonkTradeEvent),
    BonkPoolCreate(BonkPoolCreateEvent),
    BonkMigrateAmm(BonkMigrateAmmEvent),

    // PumpSwap 事件
    PumpSwapBuy(PumpSwapBuyEvent),
    PumpSwapSell(PumpSwapSellEvent),
    PumpSwapCreatePool(PumpSwapCreatePoolEvent),
    PumpSwapPoolCreated(PumpSwapPoolCreated),
    PumpSwapTrade(PumpSwapTrade),
    PumpSwapLiquidityAdded(PumpSwapLiquidityAdded),
    PumpSwapLiquidityRemoved(PumpSwapLiquidityRemoved),
    PumpSwapPoolUpdated(PumpSwapPoolUpdated),
    PumpSwapFeesClaimed(PumpSwapFeesClaimed),

    // Raydium CLMM 事件
    RaydiumClmmSwap(RaydiumClmmSwapEvent),
    RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent),
    RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent),
    RaydiumClmmOpenPositionWithTokenExtNft(RaydiumClmmOpenPositionWithTokenExtNftEvent),
    RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent),
    RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent),
    RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent),
    RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent),

    // Raydium CPMM 事件
    RaydiumCpmmSwap(RaydiumCpmmSwapEvent),
    RaydiumCpmmDeposit(RaydiumCpmmDepositEvent),
    RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent),
    RaydiumCpmmInitialize(RaydiumCpmmInitializeEvent),

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

    // Meteora Pools 事件
    MeteoraPoolsSwap(MeteoraPoolsSwapEvent),
    MeteoraPoolsAddLiquidity(MeteoraPoolsAddLiquidityEvent),
    MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent),
    MeteoraPoolsBootstrapLiquidity(MeteoraPoolsBootstrapLiquidityEvent),
    MeteoraPoolsPoolCreated(MeteoraPoolsPoolCreatedEvent),
    MeteoraPoolsSetPoolFees(MeteoraPoolsSetPoolFeesEvent),

    // Meteora DAMM V2 事件
    MeteoraDammV2Swap(MeteoraDammV2SwapEvent),
    MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent),
    MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent),
    MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent),
    MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent),
    MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent),
    MeteoraDammV2ClaimPositionFee(MeteoraDammV2ClaimPositionFeeEvent),
    MeteoraDammV2InitializeReward(MeteoraDammV2InitializeRewardEvent),
    MeteoraDammV2FundReward(MeteoraDammV2FundRewardEvent),
    MeteoraDammV2ClaimReward(MeteoraDammV2ClaimRewardEvent),

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