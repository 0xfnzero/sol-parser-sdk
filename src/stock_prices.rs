//! Opt-in stock/ETF execution-price notifications. No HTTP or RPC in this module.
//! Prices describe reported swap amounts, not executable quotes or stock NAV.
use crate::core::events::{EventMetadata, RawInstructionDexType, TradeParseStatus};
use crate::grpc::{EventType, EventTypeFilter, TransactionFilter, YellowstoneGrpc};
use crate::DexEvent;
use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc};

pub const WSOL: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
pub const USDC: Pubkey = solana_sdk::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const DEFAULT_SOL_USDC_POOL: Pubkey =
    solana_sdk::pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockToken {
    pub mint: Pubkey,
    pub symbol: String,
    pub decimals: u8,
    /// Identity directory, not an assertion about legal issuance or backing.
    pub catalog_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockPool {
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub quote_mint: Pubkey,
    pub dex_type: RawInstructionDexType,
    /// Historical admission evidence, NOT a live fee quote.
    #[serde(default)]
    pub catalog_fee_snapshot: Option<StockPoolFeeSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockTokenTransferTax {
    pub basis_points: u16,
    /// Cap in this mint's raw units. Effective tax depends on transfer size.
    pub maximum_fee_raw: u64,
    pub transfer_hook: bool,
    #[serde(default)]
    pub transfer_hook_program: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockPoolFeeSnapshot {
    pub snapshot_slot: u64,
    pub epoch: u64,
    pub unix_timestamp: i64,
    pub base_swap_fee_bps: f64,
    /// None for dynamic/unknown rates; the base rate is not the total rate.
    pub swap_fee_bps: Option<f64>,
    /// Historical initial active-bin/tick rate, including dynamic fees when known.
    /// Not a live fee or the average fee for an entire order.
    #[serde(default)]
    pub snapshot_start_swap_fee_bps: Option<f64>,
    pub dynamic_swap_fee: bool,
    pub stock_transfer_tax: StockTokenTransferTax,
    pub quote_transfer_tax: StockTokenTransferTax,
}

/// Empty means disabled; `screened_defaults()` explicitly opts into the catalog.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StockTokenCatalog {
    pub tokens: Vec<StockToken>,
    pub pools: Vec<StockPool>,
}

impl StockTokenCatalog {
    /// Select verified stock-conversion pools from a successful transaction,
    /// independent of its launch platform, wrapper or other swap legs.
    /// These are historical identity clues, not a connected path or live quotes.
    /// Failed transactions and mismatched pool/program/mint identities are excluded.
    pub fn conversion_legs<'a>(
        &'a self,
        route: &'a crate::transaction_route::TransactionRoute,
    ) -> impl Iterator<Item = &'a crate::transaction_route::RouteSwapLeg> + 'a {
        use crate::instr::program_ids::*;
        use crate::transaction_route::SwapProtocol;
        route.legs.iter().filter(move |leg| {
            if !route.succeeded {
                return false;
            }
            let dex = match (leg.protocol, leg.program) {
                (SwapProtocol::RaydiumCpmm, RAYDIUM_CPMM_PROGRAM_ID) => {
                    RawInstructionDexType::RaydiumCpmm
                }
                (SwapProtocol::RaydiumClmm, RAYDIUM_CLMM_PROGRAM_ID) => {
                    RawInstructionDexType::RaydiumClmm
                }
                (SwapProtocol::OrcaWhirlpool, ORCA_WHIRLPOOL_PROGRAM_ID) => {
                    RawInstructionDexType::OrcaWhirlpool
                }
                (SwapProtocol::MeteoraDlmm, METEORA_DLMM_PROGRAM_ID) => {
                    RawInstructionDexType::MeteoraDlmm
                }
                _ => return false,
            };
            self.pools.iter().any(|pool| {
                pool.pool == leg.pool
                    && pool.dex_type == dex
                    && ((leg.input_mint == Some(pool.mint)
                        && leg.output_mint == Some(pool.quote_mint))
                        || (leg.input_mint == Some(pool.quote_mint)
                            && leg.output_mint == Some(pool.mint)))
            })
        })
    }

    /// Point-in-time identities screened on 2026-10-20, not live eligibility.
    pub fn screened_defaults() -> Self {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("stock_token_catalog.json"))
                .expect("bundled stock catalog is tested");
        let key = |v: &serde_json::Value, field: &str| -> Pubkey {
            v[field].as_str().unwrap().parse().expect("tested catalog address")
        };
        Self {
            tokens: data["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| StockToken {
                    mint: key(v, "mint"),
                    symbol: v["symbol"].as_str().unwrap().into(),
                    decimals: v["decimals"].as_u64().unwrap() as u8,
                    catalog_source: v["catalog_source"].as_str().unwrap().into(),
                })
                .collect(),
            pools: data["pools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| StockPool {
                    pool: key(v, "pool"),
                    mint: key(v, "mint"),
                    quote_mint: key(v, "quote_mint"),
                    dex_type: serde_json::from_value(v["dex_type"].clone()).unwrap(),
                    catalog_fee_snapshot: serde_json::from_value(v["catalog_fee_snapshot"].clone())
                        .unwrap(),
                })
                .collect(),
        }
    }

    /// First screened pool for a mint, ordered by known snapshot-start nominal fees in the
    /// bundled catalog. Historical preference only; not a live routing quote.
    pub fn preferred_pool(&self, mint: &Pubkey) -> Option<&StockPool> {
        self.pools.iter().find(|pool| &pool.mint == mint)
    }
}

#[derive(Clone, Debug)]
pub struct StockPriceConfig {
    pub catalog: StockTokenCatalog,
    pub sol_usdc_pool: Pubkey,
    pub sol_usdc_dex: RawInstructionDexType,
    /// Local receive-time lifetime for conversion and optional liquidity snapshots.
    pub max_age_us: i64,
    /// Also reject cached data more than this many slots behind the trade.
    pub max_slot_lag: u64,
}

impl Default for StockPriceConfig {
    fn default() -> Self {
        Self {
            catalog: StockTokenCatalog::default(),
            sol_usdc_pool: DEFAULT_SOL_USDC_POOL,
            sol_usdc_dex: RawInstructionDexType::OrcaWhirlpool,
            max_age_us: 30_000_000,
            max_slot_lag: 75,
        }
    }
}

/// Optional independently collected TVL/depth, never inferred from CLMM liquidity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockPoolLiquidity {
    pub pool: Pubkey,
    pub slot: u64,
    pub observed_us: i64,
    pub tvl_usdc: Option<f64>,
    pub buy_depth_usdc: Option<f64>,
    pub sell_depth_usdc: Option<f64>,
    /// Maximum tolerated price impact in basis points; must be below 10,000.
    pub depth_max_price_impact_bps: Option<u16>,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolUsdcReference {
    pub pool: Pubkey,
    pub usdc_per_sol: f64,
    pub metadata: EventMetadata,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StockPriceBasis {
    /// Ratio of the protocol's reported executed amounts, including its fee semantics.
    /// Not a fee-adjusted user receipt, pool mid-price, or a quote for another size.
    ReportedSwapAmounts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockTokenPriceEvent {
    pub metadata: EventMetadata,
    pub mint: Pubkey,
    pub symbol: String,
    pub pool: Pubkey,
    pub dex_type: RawInstructionDexType,
    pub quote_mint: Pubkey,
    pub price_in_quote: f64,
    pub price_sol: Option<f64>,
    pub price_usdc: Option<f64>,
    pub basis: StockPriceBasis,
    pub stock_amount_raw: u64,
    pub quote_amount_raw: u64,
    pub sol_usdc_reference: Option<SolUsdcReference>,
    pub liquidity: Option<StockPoolLiquidity>,
}

/// Stateful, network-free processor; usable with an existing DEX event callback.
pub struct StockPriceTracker {
    config: StockPriceConfig,
    tokens: HashMap<Pubkey, StockToken>,
    pools: HashMap<Pubkey, TrackedPool>,
    reference: Option<SolUsdcReference>,
    liquidity: HashMap<Pubkey, StockPoolLiquidity>,
    last_position: HashMap<Pubkey, (u64, u64)>,
}

struct TrackedPool {
    pool: StockPool,
    /// Immutable decimal conversion computed once, outside event processing.
    quote_scale: f64,
}

fn supported(dex: RawInstructionDexType) -> bool {
    matches!(
        dex,
        RawInstructionDexType::RaydiumCpmm
            | RawInstructionDexType::RaydiumClmm
            | RawInstructionDexType::OrcaWhirlpool
            | RawInstructionDexType::MeteoraDlmm
    )
}

impl StockPriceTracker {
    pub fn new(mut config: StockPriceConfig) -> Result<Self, String> {
        if config.max_age_us <= 0
            || !supported(config.sol_usdc_dex)
            || config.sol_usdc_pool == Pubkey::default()
        {
            return Err("invalid price freshness or unsupported reference DEX".into());
        }
        let catalog = std::mem::take(&mut config.catalog);
        let mut tokens = HashMap::with_capacity(catalog.tokens.len());
        for token in catalog.tokens {
            if token.mint == Pubkey::default()
                || token.mint == WSOL
                || token.mint == USDC
                || token.decimals > 18
                || tokens.insert(token.mint, token).is_some()
            {
                return Err("invalid or duplicate stock token".into());
            }
        }
        let mut pools = HashMap::with_capacity(catalog.pools.len());
        for pool in catalog.pools {
            if !tokens.contains_key(&pool.mint)
                || ![WSOL, USDC].contains(&pool.quote_mint)
                || !supported(pool.dex_type)
                || pool.pool == Pubkey::default()
                || pool.pool == config.sol_usdc_pool
                || pools.contains_key(&pool.pool)
            {
                return Err("invalid, duplicate or unsupported stock pool".into());
            }
            let quote_decimals = if pool.quote_mint == WSOL { 9 } else { 6 };
            let quote_scale = 10_f64.powi(i32::from(tokens[&pool.mint].decimals) - quote_decimals);
            pools.insert(pool.pool, TrackedPool { pool, quote_scale });
        }
        Ok(Self {
            config,
            tokens,
            pools,
            reference: None,
            liquidity: HashMap::new(),
            last_position: HashMap::new(),
        })
    }

    /// Invoke after disconnects, queue loss, or a caller-detected fork rollback.
    pub fn reset(&mut self) {
        self.reference = None;
        self.liquidity.clear();
        self.last_position.clear();
    }

    /// Publish externally collected coherent liquidity. No automatic RPC fallback.
    pub fn update_liquidity(&mut self, snapshot: StockPoolLiquidity) -> Result<(), String> {
        if !self.pools.contains_key(&snapshot.pool)
            || snapshot.observed_us <= 0
            || snapshot.source.trim().is_empty()
            || snapshot.depth_max_price_impact_bps.is_some_and(|bps| bps >= 10_000)
            || [snapshot.tvl_usdc, snapshot.buy_depth_usdc, snapshot.sell_depth_usdc]
                .into_iter()
                .flatten()
                .any(|x| !x.is_finite() || x < 0.0)
            || ((snapshot.buy_depth_usdc.is_some() || snapshot.sell_depth_usdc.is_some())
                && snapshot.depth_max_price_impact_bps.is_none())
        {
            return Err("invalid liquidity snapshot".into());
        }
        if let Some(old) = self.liquidity.get(&snapshot.pool) {
            if snapshot.slot < old.slot || snapshot.observed_us < old.observed_us {
                return Err("out-of-order liquidity snapshot".into());
            }
        }
        self.liquidity.insert(snapshot.pool, snapshot);
        Ok(())
    }

    fn fresh(&self, observed_us: i64, slot: u64, metadata: &EventMetadata, now_us: i64) -> bool {
        observed_us > 0
            && observed_us <= now_us
            && now_us.saturating_sub(observed_us) <= self.config.max_age_us
            && slot <= metadata.slot
            && metadata.slot - slot <= self.config.max_slot_lag
    }

    /// One notification per supported confirmed swap in a configured pool.
    /// Instruction-only Shred candidates never produce a price. `now_us` must
    /// use the same wall-clock domain as EventMetadata.grpc_recv_us.
    pub fn process(&mut self, event: &DexEvent, now_us: i64) -> Option<StockTokenPriceEvent> {
        if self.pools.is_empty()
            || event.trade_parse_status() != Some(TradeParseStatus::ExecutionConfirmed)
        {
            return None;
        }
        let swap = executed_swap(event)?;
        let metadata = event.metadata();
        if swap.input_amount == 0
            || swap.output_amount == 0
            || !self.fresh(metadata.grpc_recv_us, metadata.slot, metadata, now_us)
        {
            return None;
        }
        let position = (metadata.slot, metadata.tx_index);
        if self.last_position.get(&swap.pool).is_some_and(|old| *old > position) {
            return None;
        }
        if swap.pool == self.config.sol_usdc_pool && swap.dex == self.config.sol_usdc_dex {
            let (sol, usdc) = if swap.input == WSOL && swap.output == USDC {
                (swap.input_amount, swap.output_amount)
            } else if swap.input == USDC && swap.output == WSOL {
                (swap.output_amount, swap.input_amount)
            } else {
                return None;
            };
            self.reference = Some(SolUsdcReference {
                pool: swap.pool,
                usdc_per_sol: usdc as f64 / sol as f64 * 1000.0,
                metadata: metadata.clone(),
            });
            self.last_position.insert(swap.pool, position);
            return None;
        }
        let tracked = self.pools.get(&swap.pool)?;
        let pool = &tracked.pool;
        if pool.dex_type != swap.dex {
            return None;
        }
        let token = self.tokens.get(&pool.mint)?;
        let (stock, quote) = if swap.input == pool.mint && swap.output == pool.quote_mint {
            (swap.input_amount, swap.output_amount)
        } else if swap.output == pool.mint && swap.input == pool.quote_mint {
            (swap.output_amount, swap.input_amount)
        } else {
            return None;
        };
        let price = quote as f64 / stock as f64 * tracked.quote_scale;
        if !price.is_finite() || price <= 0.0 {
            return None;
        }
        let reference = self
            .reference
            .as_ref()
            .filter(|r| {
                // Metadata has no universal CPI position. A reference from the
                // same transaction could execute after this swap, so exclude it.
                (r.metadata.slot, r.metadata.tx_index) < position
                    && self.fresh(r.metadata.grpc_recv_us, r.metadata.slot, metadata, now_us)
            })
            .cloned();
        let (price_sol, price_usdc) = if pool.quote_mint == WSOL {
            (Some(price), reference.as_ref().map(|r| price * r.usdc_per_sol))
        } else {
            (reference.as_ref().map(|r| price / r.usdc_per_sol), Some(price))
        };
        let liquidity = self
            .liquidity
            .get(&swap.pool)
            .filter(|l| self.fresh(l.observed_us, l.slot, metadata, now_us))
            .cloned();
        self.last_position.insert(swap.pool, position);
        Some(StockTokenPriceEvent {
            metadata: metadata.clone(),
            mint: token.mint,
            symbol: token.symbol.clone(),
            pool: pool.pool,
            dex_type: pool.dex_type,
            quote_mint: pool.quote_mint,
            price_in_quote: price,
            price_sol,
            price_usdc,
            basis: StockPriceBasis::ReportedSwapAmounts,
            stock_amount_raw: stock,
            quote_amount_raw: quote,
            sol_usdc_reference: reference,
            liquidity,
        })
    }
}

struct ExecutedSwap {
    pool: Pubkey,
    dex: RawInstructionDexType,
    input: Pubkey,
    output: Pubkey,
    input_amount: u64,
    output_amount: u64,
}

fn executed_swap(event: &DexEvent) -> Option<ExecutedSwap> {
    use RawInstructionDexType::*;
    let (pool, dex, input, output, input_amount, output_amount) = match event {
        DexEvent::RaydiumCpmmSwap(e) if e.input_vault_before > 0 && e.output_vault_before > 0 => (
            e.pool_id,
            RaydiumCpmm,
            e.input_token_mint,
            e.output_token_mint,
            e.input_amount,
            e.output_amount,
        ),
        DexEvent::RaydiumClmmSwap(e) if e.sqrt_price_x64 > 0 => {
            let (a, b) =
                if e.zero_for_one { (e.amount_0, e.amount_1) } else { (e.amount_1, e.amount_0) };
            (e.pool_state, RaydiumClmm, e.input_mint, e.output_mint, a, b)
        }
        DexEvent::OrcaWhirlpoolSwap(e) if e.post_sqrt_price > 0 => {
            let (a, b) = if e.a_to_b {
                (e.token_mint_a, e.token_mint_b)
            } else {
                (e.token_mint_b, e.token_mint_a)
            };
            (e.whirlpool, OrcaWhirlpool, a, b, e.input_amount, e.output_amount)
        }
        // Legacy exact-out instructions place max_input in amount_in. Successful
        // execution alone cannot turn that limit into a price; require a decoded
        // binary execution event (version 1/2), not instruction/text fallback.
        DexEvent::MeteoraDlmmSwap(e) if matches!(e.event_version, 1 | 2) => {
            let (a, b) = if e.swap_for_y {
                (e.token_x_mint, e.token_y_mint)
            } else {
                (e.token_y_mint, e.token_x_mint)
            };
            (e.pool, MeteoraDlmm, a, b, e.amount_in, e.amount_out)
        }
        _ => return None,
    };
    Some(ExecutedSwap { pool, dex, input, output, input_amount, output_amount })
}

/// Dedicated opt-in price queue. Use a separate YellowstoneGrpc client: like
/// subscribe_dex_events, subscribing replaces that client's previous subscription.
pub struct StockPriceSubscription {
    grpc: YellowstoneGrpc,
    queue: Arc<ArrayQueue<DexEvent>>,
    tracker: StockPriceTracker,
    revision: u64,
}

impl StockPriceSubscription {
    pub async fn subscribe(
        grpc: YellowstoneGrpc,
        config: StockPriceConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let tracker = StockPriceTracker::new(config).map_err(std::io::Error::other)?;
        if tracker.pools.is_empty() {
            return Err("stock price subscription needs configured pools".into());
        }
        let mut accounts: Vec<String> = tracker.pools.keys().map(ToString::to_string).collect();
        accounts.push(tracker.config.sol_usdc_pool.to_string());
        let filter = EventTypeFilter::include_only(vec![
            EventType::RaydiumCpmmSwap,
            EventType::RaydiumClmmSwap,
            EventType::OrcaWhirlpoolSwap,
            EventType::MeteoraDlmmSwap,
        ]);
        let queue = grpc
            .subscribe_dex_events(
                vec![TransactionFilter { account_include: accounts, ..Default::default() }],
                vec![],
                Some(filter),
            )
            .await?;
        let revision = grpc.subscription_status().continuity_revision;
        Ok(Self { grpc, queue, tracker, revision })
    }

    pub fn update_liquidity(&mut self, snapshot: StockPoolLiquidity) -> Result<(), String> {
        self.tracker.update_liquidity(snapshot)
    }

    /// Bounded drain; call again when None. Clears stale caches after stream gaps.
    pub fn pop(&mut self, now_us: i64) -> Option<StockTokenPriceEvent> {
        let status = self.grpc.subscription_status();
        if status.continuity_revision != self.revision || !status.connected {
            self.revision = status.continuity_revision;
            self.tracker.reset();
            for _ in 0..self.queue.len() {
                self.queue.pop();
            }
            return None;
        }
        for _ in 0..256 {
            let event = self.queue.pop()?;
            if let Some(price) = self.tracker.process(&event, now_us) {
                if self.grpc.subscription_status().continuity_revision != self.revision {
                    self.tracker.reset();
                    return None;
                }
                return Some(price);
            }
        }
        None
    }

    pub async fn stop(&self) {
        self.grpc.stop().await;
    }
}
