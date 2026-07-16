//! PumpFun instruction parser
//!
//! Parse PumpFun instructions using discriminator pattern matching

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// PumpFun discriminator constants
pub mod discriminators {
    /// Buy instruction: buy tokens with SOL (legacy)
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    /// Sell instruction: sell tokens for SOL (legacy)
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
    /// Create instruction: create a new bonding curve
    pub const CREATE: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
    /// CreateV2 instruction: SPL-22 / Mayhem mode (idl create_v2)
    pub const CREATE_V2: [u8; 8] = [214, 144, 76, 236, 95, 139, 49, 180];
    /// buy_exact_sol_in: Given a budget of spendable SOL, buy at least min_tokens_out (legacy)
    pub const BUY_EXACT_SOL_IN: [u8; 8] = [56, 252, 116, 8, 158, 223, 205, 95];
    /// Migrate event log discriminator (CPI)
    pub const MIGRATE_EVENT_LOG: [u8; 8] = [189, 233, 93, 185, 92, 148, 234, 148];
    /// `migrate_bonding_curve_creator` 外层 ix（`idls/pumpfun.json`）
    pub const MIGRATE_BONDING_CURVE_CREATOR: [u8; 8] = [87, 124, 52, 191, 52, 38, 214, 232];
    /// buy_v2: unified buy with quote_mint support (SOL + USDC)
    pub const BUY_V2: [u8; 8] = [184, 23, 238, 97, 103, 197, 211, 61];
    /// sell_v2: unified sell with quote_mint support (SOL + USDC)
    pub const SELL_V2: [u8; 8] = [93, 246, 130, 60, 231, 233, 64, 178];
    /// buy_exact_quote_in_v2: spend exact quote amount for min tokens out (SOL + USDC)
    pub const BUY_EXACT_QUOTE_IN_V2: [u8; 8] = [194, 171, 28, 70, 104, 77, 91, 47];
}

/// PumpFun Program ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::PUMPFUN_PROGRAM_ID;

#[inline(always)]
fn create_v2_quote_accounts_from_accounts(accounts: &[Pubkey]) -> (Pubkey, Pubkey, Pubkey) {
    if accounts.len() < 19 {
        return (PUMPFUN_SOLSCAN_SOL_QUOTE_MINT, Pubkey::default(), Pubkey::default());
    }
    let quote_mint = get_account(accounts, 16).unwrap_or_default();
    let quote_vault = get_account(accounts, 17).unwrap_or_default();
    let quote_token_program = get_account(accounts, 18).unwrap_or_default();
    if quote_mint == Pubkey::default()
        || quote_mint == program_ids::PUMPFUN_PROGRAM_ID
        || quote_vault == Pubkey::default()
        || quote_token_program == Pubkey::default()
    {
        return (Pubkey::default(), Pubkey::default(), Pubkey::default());
    }
    (normalize_pumpfun_quote_mint(quote_mint), quote_vault, quote_token_program)
}

/// Main PumpFun instruction parser
///
/// Outer instructions (8-byte discriminator): CREATE, CREATE_V2 从指令解析并返回事件；
/// BUY/SELL 仍以 log 为主。Inner CPI: MIGRATE_EVENT_LOG 仅在此解析。
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }
    let outer_disc: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    // 外层指令：Create / CreateV2（与 solana-streamer 功能对齐）
    if outer_disc == discriminators::CREATE_V2 {
        return parse_create_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }
    if outer_disc == discriminators::CREATE {
        return parse_create_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }
    if outer_disc == discriminators::BUY {
        return parse_buy_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "buy",
            false,
        );
    }
    if outer_disc == discriminators::BUY_EXACT_SOL_IN {
        return parse_buy_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "buy_exact_sol_in",
            true,
        );
    }
    if outer_disc == discriminators::SELL {
        return parse_sell_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "sell",
            false,
        );
    }
    if outer_disc == discriminators::BUY_V2 {
        return parse_buy_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "buy_v2",
            false,
        );
    }
    if outer_disc == discriminators::BUY_EXACT_QUOTE_IN_V2 {
        return parse_buy_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "buy_exact_quote_in_v2",
            true,
        );
    }
    if outer_disc == discriminators::SELL_V2 {
        return parse_sell_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            "sell_v2",
        );
    }

    // Inner CPI：仅 MIGRATE 在此解析
    if instruction_data.len() >= 16 {
        let cpi_disc: [u8; 8] = instruction_data[8..16].try_into().ok()?;
        if cpi_disc == discriminators::MIGRATE_EVENT_LOG {
            return parse_migrate_log_instruction(
                &instruction_data[16..],
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            );
        }
    }
    None
}

/// Parse buy/buy_exact_sol_in instruction
///
/// Account indices (from pump.json IDL), 16 个固定账户:
/// 0: global, 1: fee_recipient, 2: mint, 3: bonding_curve,
/// 4: associated_bonding_curve, 5: associated_user, 6: user,
/// 7: system_program, 8: token_program, 9: creator_vault,
/// 10: event_authority, 11: program, 12: global_volume_accumulator,
/// 13: user_volume_accumulator, 14: fee_config, 15: fee_program.
/// Post-upgrade remaining accounts: 16 bonding_curve_v2, 17 buyback_fee_recipient.
fn parse_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    ix_name: &'static str,
    exact_quote_in: bool,
) -> Option<DexEvent> {
    const LEGACY_BUY_ACCOUNTS: usize = 16;
    if accounts.len() < LEGACY_BUY_ACCOUNTS {
        return None;
    }

    // buy: amount, max_sol_cost. buy_exact_sol_in: spendable_sol_in, min_tokens_out.
    let (first_arg, second_arg) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };
    let track_volume = data.get(16).copied().map(|b| b != 0).unwrap_or(false);
    let (
        token_amount,
        sol_amount,
        amount,
        max_sol_cost,
        spendable_sol_in,
        spendable_quote_in,
        min_tokens_out,
    ) = if exact_quote_in {
        (second_arg, first_arg, second_arg, first_arg, first_arg, 0, second_arg)
    } else {
        (first_arg, second_arg, first_arg, second_arg, 0, 0, 0)
    };
    let bonding_curve_v2 = get_account(accounts, 16).unwrap_or_default();
    let buyback_fee_recipient = get_account(accounts, 17).unwrap_or_default();
    let account =
        if buyback_fee_recipient != Pubkey::default() { Some(buyback_fee_recipient) } else { None };
    let fee_program = get_account(accounts, 15).unwrap_or_default();
    let mint = get_account(accounts, 2)?;
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    let trade_event = PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        is_buy: true,
        global: get_account(accounts, 0).unwrap_or_default(),
        fee_recipient: get_account(accounts, 1).unwrap_or_default(),
        bonding_curve: get_account(accounts, 3).unwrap_or_default(),
        bonding_curve_v2,
        associated_bonding_curve: get_account(accounts, 4).unwrap_or_default(),
        associated_user: get_account(accounts, 5).unwrap_or_default(),
        user: get_account(accounts, 6).unwrap_or_default(),
        system_program: get_account(accounts, 7).unwrap_or_default(),
        token_program: get_account(accounts, 8).unwrap_or_default(),
        creator_vault: get_account(accounts, 9).unwrap_or_default(),
        event_authority: get_account(accounts, 10).unwrap_or_default(),
        program: get_account(accounts, 11).unwrap_or_default(),
        global_volume_accumulator: get_account(accounts, 12).unwrap_or_default(),
        user_volume_accumulator: get_account(accounts, 13).unwrap_or_default(),
        fee_config: get_account(accounts, 14).unwrap_or_default(),
        fee_program,
        buyback_fee_recipient,
        account,
        sol_amount,
        token_amount,
        amount,
        max_sol_cost,
        spendable_sol_in,
        spendable_quote_in,
        min_tokens_out,
        track_volume,
        ix_name: ix_name.to_string(),
        ..Default::default()
    };

    if exact_quote_in {
        Some(DexEvent::PumpFunBuyExactSolIn(trade_event))
    } else {
        Some(DexEvent::PumpFunBuy(trade_event))
    }
}

/// Parse sell instruction
///
/// Account indices (from pump.json IDL), 14 个固定账户:
/// 0: global, 1: fee_recipient, 2: mint, 3: bonding_curve,
/// 4: associated_bonding_curve, 5: associated_user, 6: user,
/// 7: system_program, 8: creator_vault, 9: token_program,
/// 10: event_authority, 11: program, 12: fee_config, 13: fee_program.
/// Post-upgrade non-cashback: 14 bonding_curve_v2, 15 buyback_fee_recipient.
/// Post-upgrade cashback: 14 user_volume_accumulator, 15 bonding_curve_v2, 16 buyback_fee_recipient.
fn parse_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    ix_name: &'static str,
    v2_accounts: bool,
) -> Option<DexEvent> {
    let min_accounts = if v2_accounts { 26 } else { 14 };
    if accounts.len() < min_accounts {
        return None;
    }

    // Parse args: amount (u64), min_sol_output (u64)
    let (amount, min_sol_output) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };
    let token_amount = amount;
    let sol_amount = min_sol_output;

    let (
        global_idx,
        mint_idx,
        bonding_curve_idx,
        associated_bonding_curve_idx,
        associated_user_idx,
        user_idx,
        system_program_idx,
        fee_recipient_idx,
        token_program_idx,
        creator_vault_idx,
        event_authority_idx,
        program_idx,
        user_volume_accumulator_idx,
        fee_config_idx,
        fee_program_idx,
    ) = if v2_accounts {
        (0, 1, 10, 11, 14, 13, 23, 6, 3, 16, 24, 25, 19, 21, 22)
    } else {
        (0, 2, 3, 4, 5, 6, 7, 1, 9, 8, 10, 11, usize::MAX, 12, 13)
    };
    let mint = get_account(accounts, mint_idx)?;
    let (legacy_user_volume_accumulator, legacy_bonding_curve_v2, legacy_buyback_fee_recipient) =
        if v2_accounts {
            (Pubkey::default(), Pubkey::default(), Pubkey::default())
        } else if accounts.len() >= 17 {
            (
                get_account(accounts, 14).unwrap_or_default(),
                get_account(accounts, 15).unwrap_or_default(),
                get_account(accounts, 16).unwrap_or_default(),
            )
        } else if accounts.len() >= 16 {
            (
                Pubkey::default(),
                get_account(accounts, 14).unwrap_or_default(),
                get_account(accounts, 15).unwrap_or_default(),
            )
        } else {
            (Pubkey::default(), get_account(accounts, 14).unwrap_or_default(), Pubkey::default())
        };
    let account = if legacy_buyback_fee_recipient != Pubkey::default() {
        Some(legacy_buyback_fee_recipient)
    } else {
        None
    };
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunSell(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: normalize_pumpfun_quote_mint(if v2_accounts {
            get_account(accounts, 2).unwrap_or_default()
        } else {
            Pubkey::default()
        }),
        is_buy: false,
        global: get_account(accounts, global_idx).unwrap_or_default(),
        bonding_curve: get_account(accounts, bonding_curve_idx).unwrap_or_default(),
        bonding_curve_v2: legacy_bonding_curve_v2,
        associated_bonding_curve: get_account(accounts, associated_bonding_curve_idx)
            .unwrap_or_default(),
        associated_user: get_account(accounts, associated_user_idx).unwrap_or_default(),
        user: get_account(accounts, user_idx).unwrap_or_default(),
        system_program: get_account(accounts, system_program_idx).unwrap_or_default(),
        fee_recipient: get_account(accounts, fee_recipient_idx).unwrap_or_default(),
        token_program: get_account(accounts, token_program_idx).unwrap_or_default(),
        quote_token_program: if v2_accounts {
            get_account(accounts, 4).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        associated_token_program: if v2_accounts {
            get_account(accounts, 5).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        creator_vault: get_account(accounts, creator_vault_idx).unwrap_or_default(),
        associated_quote_fee_recipient: if v2_accounts {
            get_account(accounts, 7).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        associated_quote_buyback_fee_recipient: if v2_accounts {
            get_account(accounts, 9).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        associated_quote_bonding_curve: if v2_accounts {
            get_account(accounts, 12).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        associated_quote_user: if v2_accounts {
            get_account(accounts, 15).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        associated_creator_vault: if v2_accounts {
            get_account(accounts, 17).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        sharing_config: if v2_accounts {
            get_account(accounts, 18).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        event_authority: get_account(accounts, event_authority_idx).unwrap_or_default(),
        program: get_account(accounts, program_idx).unwrap_or_default(),
        user_volume_accumulator: if v2_accounts {
            get_account(accounts, user_volume_accumulator_idx).unwrap_or_default()
        } else {
            legacy_user_volume_accumulator
        },
        associated_user_volume_accumulator: if v2_accounts {
            get_account(accounts, 20).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        fee_config: get_account(accounts, fee_config_idx).unwrap_or_default(),
        fee_program: get_account(accounts, fee_program_idx).unwrap_or_default(),
        buyback_fee_recipient: if v2_accounts {
            get_account(accounts, 8).unwrap_or_default()
        } else {
            legacy_buyback_fee_recipient
        },
        account,
        sol_amount,
        token_amount,
        amount,
        min_sol_output,
        ix_name: ix_name.to_string(),
        ..Default::default()
    }))
}

fn parse_buy_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    ix_name: &'static str,
    exact_quote_in: bool,
) -> Option<DexEvent> {
    const MIN_ACC: usize = 27;
    if accounts.len() < MIN_ACC {
        return None;
    }

    // buy_v2: amount, max_sol_cost. buy_exact_quote_in_v2: spendable quote in, min_tokens_out.
    let (first_arg, second_arg) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };
    let (
        token_amount,
        sol_amount,
        amount,
        max_sol_cost,
        quote_amount,
        spendable_quote_in,
        min_tokens_out,
    ) = if exact_quote_in {
        (second_arg, first_arg, second_arg, 0, first_arg, first_arg, second_arg)
    } else {
        (first_arg, second_arg, first_arg, second_arg, 0, 0, 0)
    };

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);
    let trade_event = PumpFunTradeEvent {
        metadata,
        mint: accounts[1],
        quote_mint: normalize_pumpfun_quote_mint(accounts[2]),
        is_buy: true,
        global: accounts[0],
        bonding_curve: accounts[10],
        associated_bonding_curve: accounts[11],
        associated_user: accounts[14],
        user: accounts[13],
        system_program: accounts[24],
        quote_token_program: accounts[4],
        associated_token_program: accounts[5],
        sol_amount,
        token_amount,
        amount,
        max_sol_cost,
        quote_amount,
        spendable_sol_in: 0,
        spendable_quote_in,
        min_tokens_out,
        fee_recipient: accounts[6],
        token_program: accounts[3],
        creator_vault: accounts[16],
        associated_quote_fee_recipient: accounts[7],
        buyback_fee_recipient: accounts[8],
        associated_quote_buyback_fee_recipient: accounts[9],
        associated_quote_bonding_curve: accounts[12],
        associated_quote_user: accounts[15],
        associated_creator_vault: accounts[17],
        sharing_config: accounts[18],
        event_authority: accounts[25],
        program: accounts[26],
        global_volume_accumulator: accounts[19],
        user_volume_accumulator: accounts[20],
        associated_user_volume_accumulator: accounts[21],
        fee_config: accounts[22],
        fee_program: accounts[23],
        ix_name: ix_name.to_string(),
        ..Default::default()
    };

    Some(DexEvent::PumpFunBuy(trade_event))
}

fn parse_sell_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    ix_name: &'static str,
) -> Option<DexEvent> {
    parse_sell_instruction(
        data,
        accounts,
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
        ix_name,
        true,
    )
}

/// Parse create instruction (legacy)
///
/// Account indices (from pump.json):
/// 0: mint, 1: mint_authority, 2: bonding_curve, 3: associated_bonding_curve,
/// 4: global, 5: mpl_token_metadata, 6: metadata, 7: user. 共至少 8 个账户。
fn parse_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if accounts.len() < 8 {
        return None;
    }

    let mut offset = 0;

    // Parse args: name (string), symbol (string), uri (string), creator (pubkey)
    // String format: 4-byte length prefix + content
    let name = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    let symbol = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    let uri = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    // 读取 mint, bonding_curve, user, creator (在 name, symbol, uri 之后)
    if data.len() < offset + 32 + 32 + 32 + 32 {
        return None;
    }

    let mint = read_pubkey(data, offset).unwrap_or_default();
    offset += 32;

    let bonding_curve = read_pubkey(data, offset).unwrap_or_default();
    offset += 32;

    let user = read_pubkey(data, offset).unwrap_or_default();
    offset += 32;

    let creator = read_pubkey(data, offset).unwrap_or_default();

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        creator,
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        ix_name: "create".to_string(),
        ..Default::default()
    }))
}

/// Parse create_v2 instruction (SPL-22；Mayhem 由 **data** 中 `is_mayhem_mode` 决定，不要用 mayhem 程序账户是否非空推断)
///
/// Account indices (idl pumpfun.json create_v2): 0 mint, 1 mint_authority, 2 bonding_curve,
/// 3 associated_bonding_curve, 4 global, 5 user, 6 system_program, 7 token_program,
/// 8 associated_token_program, 9 mayhem_program_id, 10 global_params, 11 sol_vault,
/// 12 mayhem_state, 13 mayhem_token_vault, 14 event_authority, 15 program. 共 16 个账户。
/// Quote-pool variant appends: 16 quote_mint, 17 quote_vault, 18 quote_token_program.
/// Instruction args (after disc): name, symbol, uri, creator, is_mayhem_mode (`bool`), is_cashback_enabled (`OptionBool` = 1-byte bool on wire)。
/// Guard: return None when accounts.len() < 16 to avoid index out of bounds (e.g. ALT-loaded tx).
fn parse_create_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    const CREATE_V2_MIN_ACCOUNTS: usize = 16;
    if accounts.len() < CREATE_V2_MIN_ACCOUNTS {
        return None;
    }
    let acc = &accounts[0..CREATE_V2_MIN_ACCOUNTS];

    // IDL args: name, symbol, uri, creator, is_mayhem_mode, is_cashback_enabled — mint/bc/user 仅在 accounts
    let mut offset = 0usize;
    let name = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    let symbol = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    let uri = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    if data.len() < offset + 32 + 1 {
        return None;
    }
    let creator = read_pubkey(data, offset)?;
    offset += 32;
    let is_mayhem_mode = read_bool(data, offset)?;
    offset += 1;
    let is_cashback_enabled = read_option_bool_idl(data, offset).unwrap_or(false);

    let mint = acc[0];
    let bonding_curve = acc[2];
    let user = acc[5];
    let (quote_mint, quote_vault, quote_token_program) =
        create_v2_quote_accounts_from_accounts(accounts);

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        creator,
        mint_authority: acc[1],
        associated_bonding_curve: acc[3],
        global: acc[4],
        system_program: acc[6],
        token_program: acc[7],
        associated_token_program: acc[8],
        mayhem_program_id: acc[9],
        global_params: acc[10],
        sol_vault: acc[11],
        mayhem_state: acc[12],
        mayhem_token_vault: acc[13],
        event_authority: acc[14],
        program: acc[15],
        is_mayhem_mode,
        is_cashback_enabled,
        quote_mint,
        quote_vault,
        quote_token_program,
        ix_name: "create_v2".to_string(),
        ..Default::default()
    }))
}

/// Parse Migrate CPI instruction
#[allow(unused_variables)]
fn parse_migrate_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // user (Pubkey - 32 bytes)
    let user = read_pubkey(data, offset)?;
    offset += 32;

    // mint (Pubkey - 32 bytes)
    let mint = read_pubkey(data, offset)?;
    offset += 32;

    // mintAmount (u64 - 8 bytes)
    let mint_amount = read_u64_le(data, offset)?;
    offset += 8;

    // solAmount (u64 - 8 bytes)
    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    // poolMigrationFee (u64 - 8 bytes)
    let pool_migration_fee = read_u64_le(data, offset)?;
    offset += 8;

    // bondingCurve (Pubkey - 32 bytes)
    let bonding_curve = read_pubkey(data, offset)?;
    offset += 32;

    // timestamp (i64 - 8 bytes)
    let timestamp = read_u64_le(data, offset)? as i64;
    offset += 8;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
        metadata,
        user,
        mint,
        mint_amount,
        sol_amount,
        pool_migration_fee,
        bonding_curve,
        timestamp,
        pool,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction_data(discriminator: [u8; 8], first: u64, second: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&discriminator);
        data.extend_from_slice(&first.to_le_bytes());
        data.extend_from_slice(&second.to_le_bytes());
        data
    }

    fn accounts(n: usize) -> Vec<Pubkey> {
        (0..n).map(|_| Pubkey::new_unique()).collect()
    }

    fn str_arg(s: &str, out: &mut Vec<u8>) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    fn create_v2_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::CREATE_V2);
        str_arg("Token", &mut data);
        str_arg("TOK", &mut data);
        str_arg("https://example.invalid/token.json", &mut data);
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        data.push(1);
        data.push(1);
        data
    }

    #[test]
    fn pumpfun_create_v2_instruction_emits_canonical_create() {
        let acc = accounts(16);
        let event =
            parse_instruction(&create_v2_data(), &acc, Signature::default(), 1, 0, None, 99)
                .expect("event");

        match event {
            DexEvent::PumpFunCreate(c) => {
                assert_eq!(c.name, "Token");
                assert_eq!(c.symbol, "TOK");
                assert_eq!(c.mint, acc[0]);
                assert_eq!(c.mint_authority, acc[1]);
                assert_eq!(c.bonding_curve, acc[2]);
                assert_eq!(c.associated_bonding_curve, acc[3]);
                assert_eq!(c.user, acc[5]);
                assert_eq!(c.token_program, acc[7]);
                assert_eq!(c.mayhem_program_id, acc[9]);
                assert_eq!(c.program, acc[15]);
                assert_eq!(c.ix_name, "create_v2");
                assert!(c.is_mayhem_mode);
                assert!(c.is_cashback_enabled);
                assert_eq!(c.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
            }
            other => panic!("expected canonical PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_create_v2_instruction_uses_appended_quote_mint_only_for_19_accounts() {
        let mut acc = accounts(19);
        acc[16] = PUMPFUN_WSOL_QUOTE_MINT;
        let event =
            parse_instruction(&create_v2_data(), &acc, Signature::default(), 1, 0, None, 99)
                .expect("event");

        match event {
            DexEvent::PumpFunCreate(c) => {
                assert_eq!(c.quote_mint, PUMPFUN_WSOL_QUOTE_MINT);
            }
            other => panic!("expected canonical PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_create_v2_instruction_does_not_emit_partial_quote_tail() {
        let mut acc = accounts(19);
        acc[16] = PUMPFUN_WSOL_QUOTE_MINT;
        acc[17] = Pubkey::default();
        let event =
            parse_instruction(&create_v2_data(), &acc, Signature::default(), 1, 0, None, 99)
                .expect("event");

        match event {
            DexEvent::PumpFunCreate(c) => {
                assert_eq!(c.quote_mint, Pubkey::default());
                assert_eq!(c.quote_vault, Pubkey::default());
                assert_eq!(c.quote_token_program, Pubkey::default());
            }
            other => panic!("expected canonical PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_create_v2_instruction_rejects_program_id_as_quote_mint() {
        let mut acc = accounts(19);
        acc[16] = program_ids::PUMPFUN_PROGRAM_ID;
        acc[17] = Pubkey::new_unique();
        acc[18] = Pubkey::new_unique();
        let event =
            parse_instruction(&create_v2_data(), &acc, Signature::default(), 1, 0, None, 99)
                .expect("event");

        match event {
            DexEvent::PumpFunCreate(c) => {
                assert_eq!(c.quote_mint, Pubkey::default());
                assert_eq!(c.quote_vault, Pubkey::default());
                assert_eq!(c.quote_token_program, Pubkey::default());
            }
            other => panic!("expected canonical PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_buy_instruction_exposes_raw_args() {
        let data = instruction_data(discriminators::BUY, 123, 456);
        let acc = accounts(18);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.amount, 123);
                assert_eq!(t.max_sol_cost, 456);
                assert_eq!(t.min_sol_output, 0);
                assert_eq!(t.spendable_sol_in, 0);
                assert_eq!(t.min_tokens_out, 0);
                assert_eq!(t.token_amount, 123);
                assert_eq!(t.sol_amount, 456);
                assert_eq!(t.bonding_curve_v2, acc[16]);
                assert_eq!(t.buyback_fee_recipient, acc[17]);
                assert_eq!(t.ix_name, "buy");
            }
            other => panic!("expected PumpFunBuy, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_legacy_trade_rejects_short_account_lists() {
        let buy_data = instruction_data(discriminators::BUY, 123, 456);
        assert!(parse_instruction(&buy_data, &accounts(15), Signature::default(), 1, 0, None, 99)
            .is_none());

        let sell_data = instruction_data(discriminators::SELL, 321, 654);
        assert!(parse_instruction(&sell_data, &accounts(13), Signature::default(), 1, 0, None, 99)
            .is_none());
    }

    #[test]
    fn pumpfun_sell_instruction_exposes_raw_args() {
        let data = instruction_data(discriminators::SELL, 321, 654);
        let acc = accounts(16);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunSell(t) => {
                assert_eq!(t.amount, 321);
                assert_eq!(t.max_sol_cost, 0);
                assert_eq!(t.min_sol_output, 654);
                assert_eq!(t.spendable_sol_in, 0);
                assert_eq!(t.min_tokens_out, 0);
                assert_eq!(t.token_amount, 321);
                assert_eq!(t.sol_amount, 654);
                assert_eq!(t.user_volume_accumulator, Pubkey::default());
                assert_eq!(t.bonding_curve_v2, acc[14]);
                assert_eq!(t.buyback_fee_recipient, acc[15]);
                assert_eq!(t.ix_name, "sell");
            }
            other => panic!("expected PumpFunSell, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_cashback_sell_uses_17_account_layout() {
        let data = instruction_data(discriminators::SELL, 321, 654);
        let acc = accounts(17);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunSell(t) => {
                assert_eq!(t.user_volume_accumulator, acc[14]);
                assert_eq!(t.bonding_curve_v2, acc[15]);
                assert_eq!(t.buyback_fee_recipient, acc[16]);
            }
            other => panic!("expected PumpFunSell, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_buy_exact_sol_in_exposes_exact_args() {
        let data = instruction_data(discriminators::BUY_EXACT_SOL_IN, 1_111, 2_222);
        let acc = accounts(18);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunBuyExactSolIn(t) => {
                assert_eq!(t.spendable_sol_in, 1_111);
                assert_eq!(t.spendable_quote_in, 0);
                assert_eq!(t.min_tokens_out, 2_222);
                assert_eq!(t.sol_amount, 1_111);
                assert_eq!(t.token_amount, 2_222);
                assert_eq!(t.global, acc[0]);
                assert_eq!(t.associated_user, acc[5]);
                assert_eq!(t.event_authority, acc[10]);
                assert_eq!(t.fee_program, acc[15]);
                assert_eq!(t.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
                assert_eq!(t.bonding_curve_v2, acc[16]);
                assert_eq!(t.buyback_fee_recipient, acc[17]);
                assert_eq!(t.ix_name, "buy_exact_sol_in");
            }
            other => panic!("expected PumpFunBuyExactSolIn, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_v2_instruction_args_use_v2_account_layout() {
        let data = instruction_data(discriminators::BUY_V2, 777, 888);
        let acc = accounts(27);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.amount, 777);
                assert_eq!(t.max_sol_cost, 888);
                assert_eq!(t.mint, acc[1]);
                assert_eq!(t.quote_mint, acc[2]);
                assert_eq!(t.bonding_curve, acc[10]);
                assert_eq!(t.associated_bonding_curve, acc[11]);
                assert_eq!(t.associated_quote_bonding_curve, acc[12]);
                assert_eq!(t.user, acc[13]);
                assert_eq!(t.associated_quote_user, acc[15]);
                assert_eq!(t.quote_token_program, acc[4]);
                assert_eq!(t.associated_token_program, acc[5]);
                assert_eq!(t.associated_quote_fee_recipient, acc[7]);
                assert_eq!(t.buyback_fee_recipient, acc[8]);
                assert_eq!(t.associated_quote_buyback_fee_recipient, acc[9]);
                assert_eq!(t.associated_creator_vault, acc[17]);
                assert_eq!(t.sharing_config, acc[18]);
                assert_eq!(t.global_volume_accumulator, acc[19]);
                assert_eq!(t.associated_user_volume_accumulator, acc[21]);
                assert_eq!(t.ix_name, "buy_v2");
            }
            other => panic!("expected PumpFunBuy, got {other:?}"),
        }
    }

    #[test]
    fn pumpfun_buy_exact_quote_in_v2_uses_quote_amount_fields() {
        let data = instruction_data(discriminators::BUY_EXACT_QUOTE_IN_V2, 777, 888);
        let acc = accounts(27);
        let event =
            parse_instruction(&data, &acc, Signature::default(), 1, 0, None, 99).expect("event");

        match event {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.ix_name, "buy_exact_quote_in_v2");
                assert_eq!(t.amount, 888);
                assert_eq!(t.max_sol_cost, 0);
                assert_eq!(t.quote_amount, 777);
                assert_eq!(t.spendable_quote_in, 777);
                assert_eq!(t.min_tokens_out, 888);
                assert_eq!(t.quote_mint, acc[2]);
            }
            other => panic!("expected PumpFunBuy, got {other:?}"),
        }
    }
}
