//! ShredStream 热路径：DEX **外层**指令解析（无 inner CPI）。
//!
//! - 与 `client.rs` 解耦，便于维护与 `#[inline]` 边界优化。
//! - 避免每笔交易克隆整张 `static_account_keys`、避免 `Vec<IxRef>` 指令副本。
//! - Pump.fun 使用专用外层热路径；其它已支持 DEX 协议走统一指令解析入口。

use smallvec::SmallVec;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;

use crate::accounts::program_ids::SPL_TOKEN_2022_PROGRAM_ID;
use crate::core::events::{
    normalize_pumpfun_quote_mint, DexEvent, EventMetadata, PumpFunCreateTokenEvent,
    PumpFunMigrateBondingCurveCreatorEvent, PumpFunTradeEvent, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
};
use crate::grpc::types::EventTypeFilter;
use crate::instr::program_ids::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_PROGRAM_ID, METEORA_POOLS_PROGRAM_ID,
    ORCA_WHIRLPOOL_PROGRAM_ID, PUMPSWAP_PROGRAM_ID, PUMP_FEES_PROGRAM_ID,
    RAYDIUM_AMM_V4_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID,
    RAYDIUM_LAUNCHLAB_PROGRAM_ID,
};
use crate::instr::pump::discriminators;
use crate::instr::pump::PROGRAM_ID_PUBKEY;
use crate::instr::utils::{
    read_bool, read_option_bool_idl, read_pubkey, read_str_unchecked, read_u64_le,
};

type PumpMintSet = SmallVec<[Pubkey; 4]>;
type ShredIxAccounts = SmallVec<[Pubkey; 64]>;

#[inline(always)]
fn account_from_static_or_default(static_keys: &[Pubkey], idx: u8) -> Pubkey {
    static_keys.get(idx as usize).copied().unwrap_or_default()
}

#[inline(always)]
fn program_id_references_loaded_key(program_id_index: u8, static_key_len: usize) -> bool {
    program_id_index as usize >= static_key_len
}

#[inline(always)]
fn supported_unified_outer_programs() -> &'static [Pubkey] {
    &[
        PUMPSWAP_PROGRAM_ID,
        PUMP_FEES_PROGRAM_ID,
        RAYDIUM_LAUNCHLAB_PROGRAM_ID,
        RAYDIUM_CPMM_PROGRAM_ID,
        RAYDIUM_CLMM_PROGRAM_ID,
        RAYDIUM_AMM_V4_PROGRAM_ID,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        METEORA_POOLS_PROGRAM_ID,
        METEORA_DAMM_V2_PROGRAM_ID,
        METEORA_DLMM_PROGRAM_ID,
    ]
}

#[inline(always)]
fn build_shred_ix_accounts(static_keys: &[Pubkey], ix_accounts: &[u8]) -> ShredIxAccounts {
    let mut accounts = SmallVec::with_capacity(ix_accounts.len());
    for &idx in ix_accounts {
        accounts.push(account_from_static_or_default(static_keys, idx));
    }
    accounts
}

#[inline(always)]
fn disc8(data: &[u8]) -> Option<[u8; 8]> {
    data.get(..8)?.try_into().ok()
}

#[inline(always)]
fn pumpfun_outer_data_may_parse(data: &[u8]) -> bool {
    let Some(disc) = disc8(data) else {
        return false;
    };
    matches!(
        disc,
        discriminators::CREATE
            | discriminators::CREATE_V2
            | discriminators::BUY
            | discriminators::SELL
            | discriminators::BUY_EXACT_SOL_IN
            | discriminators::BUY_V2
            | discriminators::BUY_EXACT_QUOTE_IN_V2
            | discriminators::SELL_V2
            | discriminators::MIGRATE_BONDING_CURVE_CREATOR
    )
}

#[inline(always)]
fn unified_outer_data_may_parse(program_id: Pubkey, data: &[u8]) -> bool {
    crate::instr::instruction_data_may_parse(&program_id, data)
}

#[inline(always)]
fn unknown_outer_data_may_parse(data: &[u8], filter: Option<&EventTypeFilter>) -> bool {
    if filter.is_none_or(|f| f.includes_pumpfun()) && pumpfun_outer_data_may_parse(data) {
        return true;
    }
    supported_unified_outer_programs().iter().any(|program_id| {
        is_supported_unified_outer_program(program_id, filter)
            && unified_outer_data_may_parse(*program_id, data)
    })
}

#[inline(always)]
fn push_unique_mint(mints: &mut PumpMintSet, mint: Pubkey) {
    if !mints.contains(&mint) {
        mints.push(mint);
    }
}

#[inline(always)]
fn token_program_or_default(token_program: Pubkey) -> Pubkey {
    if token_program == Pubkey::default() {
        SPL_TOKEN_2022_PROGRAM_ID
    } else {
        token_program
    }
}

#[inline(always)]
fn quote_mint_from_shred_v2_account(quote_mint: Option<Pubkey>) -> Pubkey {
    normalize_pumpfun_quote_mint(quote_mint.unwrap_or_default())
}

#[inline(always)]
fn create_v2_quote_accounts_from_shred_accounts(
    accounts_len: usize,
    get_account: impl Fn(usize) -> Option<Pubkey>,
) -> (Pubkey, Pubkey, Pubkey) {
    if accounts_len < 19 {
        return (PUMPFUN_SOLSCAN_SOL_QUOTE_MINT, Pubkey::default(), Pubkey::default());
    }
    let quote_mint = get_account(16).unwrap_or_default();
    let quote_vault = get_account(17).unwrap_or_default();
    let quote_token_program = get_account(18).unwrap_or_default();
    if quote_mint == Pubkey::default()
        || quote_mint == PROGRAM_ID_PUBKEY
        || quote_vault == Pubkey::default()
        || quote_token_program == Pubkey::default()
    {
        return (Pubkey::default(), Pubkey::default(), Pubkey::default());
    }
    (quote_mint_from_shred_v2_account(Some(quote_mint)), quote_vault, quote_token_program)
}

#[inline]
fn scan_create_mint_from_ix(
    program_id_index: u8,
    ix_accounts: &[u8],
    data: &[u8],
    static_keys: &[Pubkey],
    created_mints: &mut PumpMintSet,
    mayhem_mints: &mut PumpMintSet,
) {
    if data.len() < 8 {
        return;
    }
    if program_id_references_loaded_key(program_id_index, static_keys.len()) {
        scan_create_mint_from_unknown_program_ix(
            ix_accounts,
            data,
            static_keys,
            created_mints,
            mayhem_mints,
        );
        return;
    }
    let Some(program_id) = static_keys.get(program_id_index as usize) else {
        return;
    };
    if *program_id != PROGRAM_ID_PUBKEY {
        return;
    }
    let disc: [u8; 8] = data[0..8].try_into().unwrap_or_default();
    if disc != discriminators::CREATE && disc != discriminators::CREATE_V2 {
        return;
    }
    let Some(&mint_idx) = ix_accounts.first() else {
        return;
    };
    let Some(&mint) = static_keys.get(mint_idx as usize) else {
        return;
    };
    push_unique_mint(created_mints, mint);
    if disc == discriminators::CREATE_V2 {
        let is_mayhem = crate::instr::utils::parse_create_v2_tail_fields(&data[8..])
            .map(|(_, m, _)| m)
            .unwrap_or(false);
        if is_mayhem {
            push_unique_mint(mayhem_mints, mint);
        }
    }
}

#[inline]
fn scan_create_mint_from_unknown_program_ix(
    ix_accounts: &[u8],
    data: &[u8],
    static_keys: &[Pubkey],
    created_mints: &mut PumpMintSet,
    mayhem_mints: &mut PumpMintSet,
) {
    if data.len() < 8 {
        return;
    }
    let disc: [u8; 8] = data[0..8].try_into().unwrap_or_default();
    if disc != discriminators::CREATE && disc != discriminators::CREATE_V2 {
        return;
    }
    let Some(&mint_idx) = ix_accounts.first() else {
        return;
    };
    let Some(&mint) = static_keys.get(mint_idx as usize) else {
        return;
    };
    push_unique_mint(created_mints, mint);
    if disc == discriminators::CREATE_V2 {
        let is_mayhem = crate::instr::utils::parse_create_v2_tail_fields(&data[8..])
            .map(|(_, m, _)| m)
            .unwrap_or(false);
        if is_mayhem {
            push_unique_mint(mayhem_mints, mint);
        }
    }
}

/// 第一遍：收集本笔交易内 Pump Create/CreateV2 的 mint（**零指令副本**，直接引用 message 内 `CompiledInstruction`）。
#[inline]
fn detect_pumpfun_create_mints(
    message: &VersionedMessage,
    static_keys: &[Pubkey],
) -> (PumpMintSet, PumpMintSet) {
    let mut created_mints = PumpMintSet::new();
    let mut mayhem_mints = PumpMintSet::new();
    match message {
        VersionedMessage::Legacy(msg) => {
            for ix in &msg.instructions {
                scan_create_mint_from_ix(
                    ix.program_id_index,
                    &ix.accounts,
                    &ix.data,
                    static_keys,
                    &mut created_mints,
                    &mut mayhem_mints,
                );
            }
        }
        VersionedMessage::V0(msg) => {
            for ix in &msg.instructions {
                scan_create_mint_from_ix(
                    ix.program_id_index,
                    &ix.accounts,
                    &ix.data,
                    static_keys,
                    &mut created_mints,
                    &mut mayhem_mints,
                );
            }
        }
    }
    (created_mints, mayhem_mints)
}

/// DEX 外层指令解析，保持与交易内 ix 顺序一致。
#[inline]
fn dispatch_shred_outer(
    program_id_index: u8,
    ix_accounts: &[u8],
    data: &[u8],
    static_keys: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    filter: Option<&EventTypeFilter>,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
    events: &mut Vec<DexEvent>,
) {
    if data.is_empty() {
        return;
    }
    if program_id_references_loaded_key(program_id_index, static_keys.len()) {
        log::trace!(
            target: "sol_parser_sdk::shredstream",
            "program_id_index references an ALT-loaded account; trying discriminator-only best-effort parse"
        );
        parse_unknown_program_outer(
            ix_accounts,
            data,
            static_keys,
            signature,
            slot,
            tx_index,
            recv_us,
            filter,
            created_mints,
            mayhem_mints,
            events,
        );
        return;
    }
    let Some(program_id) = static_keys.get(program_id_index as usize) else {
        return;
    };
    if *program_id == PROGRAM_ID_PUBKEY {
        if filter.is_some_and(|f| !f.includes_pumpfun()) || !pumpfun_outer_data_may_parse(data) {
            return;
        }
        let accounts = build_shred_ix_accounts(static_keys, ix_accounts);
        if let Some(ev) = parse_pumpfun_instruction(
            data,
            &accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ) {
            push_filtered_shred_event(events, ev, filter);
        }
        return;
    }
    if !is_supported_unified_outer_program(program_id, filter) {
        return;
    }
    if !unified_outer_data_may_parse(*program_id, data) {
        return;
    }
    let accounts = build_shred_ix_accounts(static_keys, ix_accounts);
    if let Some(ev) = parse_non_pump_dex_outer(
        *program_id,
        data,
        &accounts,
        signature,
        slot,
        tx_index,
        recv_us,
        filter,
    ) {
        push_filtered_shred_event(events, ev, filter);
    }
}

#[inline]
fn push_filtered_shred_event(
    events: &mut Vec<DexEvent>,
    event: DexEvent,
    filter: Option<&EventTypeFilter>,
) -> bool {
    let Some(filter) = filter else {
        events.push(event);
        return true;
    };
    if filter.should_include_dex_event(&event) {
        events.push(filter.normalize_dex_event(event));
        true
    } else {
        false
    }
}

#[inline]
fn parse_unknown_program_outer(
    ix_accounts: &[u8],
    data: &[u8],
    static_keys: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    filter: Option<&EventTypeFilter>,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
    events: &mut Vec<DexEvent>,
) {
    if !unknown_outer_data_may_parse(data, filter) {
        return;
    }
    let accounts = build_shred_ix_accounts(static_keys, ix_accounts);

    if filter.is_none_or(|f| f.includes_pumpfun()) && pumpfun_outer_data_may_parse(data) {
        if let Some(ev) = parse_pumpfun_instruction(
            data,
            &accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ) {
            if push_filtered_shred_event(events, ev, filter) {
                log::trace!(
                    target: "sol_parser_sdk::shredstream",
                    "unknown-program shred ix parsed as first matching candidate; use a narrow filter to avoid discriminator collisions"
                );
                return;
            }
        }
    }

    for &program_id in supported_unified_outer_programs() {
        if !is_supported_unified_outer_program(&program_id, filter) {
            continue;
        }
        if let Some(ev) = parse_non_pump_dex_outer(
            program_id, data, &accounts, signature, slot, tx_index, recv_us, filter,
        ) {
            push_filtered_shred_event(events, ev, filter);
            log::trace!(
                target: "sol_parser_sdk::shredstream",
                "unknown-program shred ix parsed as first matching candidate; use a narrow filter to avoid discriminator collisions"
            );
            return;
        }
    }
}

#[inline(always)]
fn is_supported_unified_outer_program(
    program_id: &Pubkey,
    filter: Option<&EventTypeFilter>,
) -> bool {
    let Some(f) = filter else {
        return supported_unified_outer_programs().contains(program_id);
    };

    if *program_id == PUMPSWAP_PROGRAM_ID {
        f.includes_pumpswap()
    } else if *program_id == PUMP_FEES_PROGRAM_ID {
        f.includes_pump_fees()
    } else if *program_id == RAYDIUM_LAUNCHLAB_PROGRAM_ID {
        f.includes_raydium_launchlab()
    } else if *program_id == RAYDIUM_CPMM_PROGRAM_ID {
        f.includes_raydium_cpmm()
    } else if *program_id == RAYDIUM_CLMM_PROGRAM_ID {
        f.includes_raydium_clmm()
    } else if *program_id == RAYDIUM_AMM_V4_PROGRAM_ID {
        f.includes_raydium_amm_v4()
    } else if *program_id == ORCA_WHIRLPOOL_PROGRAM_ID {
        f.includes_orca_whirlpool()
    } else if *program_id == METEORA_POOLS_PROGRAM_ID {
        f.includes_meteora_pools()
    } else if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        f.includes_meteora_damm_v2()
    } else if *program_id == METEORA_DLMM_PROGRAM_ID {
        f.includes_meteora_dlmm()
    } else {
        false
    }
}

#[inline]
fn parse_non_pump_dex_outer(
    program_id: Pubkey,
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    crate::instr::parse_instruction_unified(
        data,
        accounts,
        signature,
        slot,
        tx_index,
        None,
        recv_us,
        filter,
        &program_id,
    )
}

#[inline]
pub fn parse_transaction_dex_events(
    transaction: &VersionedTransaction,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    events: &mut Vec<DexEvent>,
) {
    parse_transaction_dex_events_with_filter(
        transaction,
        signature,
        slot,
        tx_index,
        recv_us,
        None,
        events,
    );
}

#[inline]
pub fn parse_transaction_dex_events_with_filter(
    transaction: &VersionedTransaction,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    filter: Option<&EventTypeFilter>,
    events: &mut Vec<DexEvent>,
) {
    parse_transaction_pump_events_with_filter(
        transaction,
        signature,
        slot,
        tx_index,
        recv_us,
        filter,
        events,
    );
}

#[inline]
fn parse_transaction_pump_events_with_filter(
    transaction: &VersionedTransaction,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    filter: Option<&EventTypeFilter>,
    events: &mut Vec<DexEvent>,
) {
    let static_keys = transaction.message.static_account_keys();
    let (created_mints, mayhem_mints) = if filter.is_none_or(|f| f.includes_pumpfun()) {
        detect_pumpfun_create_mints(&transaction.message, static_keys)
    } else {
        (PumpMintSet::new(), PumpMintSet::new())
    };
    match &transaction.message {
        VersionedMessage::Legacy(msg) => {
            for ix in &msg.instructions {
                dispatch_shred_outer(
                    ix.program_id_index,
                    &ix.accounts,
                    &ix.data,
                    static_keys,
                    signature,
                    slot,
                    tx_index,
                    recv_us,
                    filter,
                    &created_mints,
                    &mayhem_mints,
                    events,
                );
            }
        }
        VersionedMessage::V0(msg) => {
            for ix in &msg.instructions {
                dispatch_shred_outer(
                    ix.program_id_index,
                    &ix.accounts,
                    &ix.data,
                    static_keys,
                    signature,
                    slot,
                    tx_index,
                    recv_us,
                    filter,
                    &created_mints,
                    &mayhem_mints,
                    events,
                );
            }
        }
    }
}

// --- 单条 outer ix 解析（由原 `client.rs` 迁入） ---

#[inline]
fn parse_pumpfun_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
) -> Option<DexEvent> {
    if data.len() < 8 {
        return None;
    }
    let disc: [u8; 8] = data[0..8].try_into().ok()?;
    let ix_data = &data[8..];

    match disc {
        d if d == discriminators::CREATE => {
            parse_create_instruction(data, accounts, signature, slot, tx_index, recv_us)
        }
        d if d == discriminators::CREATE_V2 => {
            parse_create_v2_instruction(data, accounts, signature, slot, tx_index, recv_us)
        }
        d if d == discriminators::BUY => parse_buy_instruction(
            ix_data,
            accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ),
        d if d == discriminators::SELL => {
            parse_sell_instruction(ix_data, accounts, signature, slot, tx_index, recv_us)
        }
        d if d == discriminators::BUY_EXACT_SOL_IN => parse_buy_exact_sol_in_instruction(
            ix_data,
            accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ),
        d if d == discriminators::BUY_V2 => parse_buy_v2_instruction(
            ix_data,
            accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ),
        d if d == discriminators::BUY_EXACT_QUOTE_IN_V2 => parse_buy_exact_quote_in_v2_instruction(
            ix_data,
            accounts,
            signature,
            slot,
            tx_index,
            recv_us,
            created_mints,
            mayhem_mints,
        ),
        d if d == discriminators::SELL_V2 => {
            parse_sell_v2_instruction(ix_data, accounts, signature, slot, tx_index, recv_us)
        }
        d if d == discriminators::MIGRATE_BONDING_CURVE_CREATOR => {
            parse_migrate_bonding_curve_creator_shred(accounts, signature, slot, tx_index, recv_us)
        }
        _ => None,
    }
}

/// `migrate_bonding_curve_creator` 外层 ix（`idls/pumpfun.json`）；无链上事件体时 `timestamp=0`，
/// `old_creator` 未知则填默认，`new_creator` 取 `sharing_config` 账户（与常见费分成迁移一致）。
#[inline]
fn parse_migrate_bonding_curve_creator_shred(
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
) -> Option<DexEvent> {
    const MIN_ACC: usize = 5;
    if accounts.len() < MIN_ACC {
        return None;
    }
    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };
    let mint = get_account(0)?;
    let bonding_curve = get_account(1).unwrap_or_default();
    let sharing_config = get_account(2).unwrap_or_default();
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };
    Some(DexEvent::PumpFunMigrateBondingCurveCreator(PumpFunMigrateBondingCurveCreatorEvent {
        metadata,
        timestamp: 0,
        mint,
        bonding_curve,
        sharing_config,
        old_creator: Pubkey::default(),
        new_creator: sharing_config,
    }))
}

#[inline]
fn parse_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
) -> Option<DexEvent> {
    if accounts.len() < 10 {
        return None;
    }

    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let mut offset = 8;

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

    let creator = if offset + 32 <= data.len() {
        read_pubkey(data, offset).unwrap_or_default()
    } else {
        Pubkey::default()
    };

    let mint = get_account(0)?;
    let bonding_curve = get_account(2).unwrap_or_default();
    let user = get_account(7).unwrap_or_default();

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        creator,
        token_program: get_account(9).unwrap_or_default(),
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        ix_name: "create".to_string(),
        ..Default::default()
    }))
}

#[inline]
fn parse_create_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
) -> Option<DexEvent> {
    const CREATE_V2_MIN_ACCOUNTS: usize = 16;
    if accounts.len() < CREATE_V2_MIN_ACCOUNTS {
        return None;
    }

    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let payload = &data[8..];
    let mut offset = 0usize;
    let name = if let Some((s, len)) = read_str_unchecked(payload, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    let symbol = if let Some((s, len)) = read_str_unchecked(payload, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    let uri = if let Some((s, len)) = read_str_unchecked(payload, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };
    if payload.len() < offset + 32 + 1 {
        return None;
    }
    let creator = read_pubkey(payload, offset).unwrap_or_default();
    offset += 32;
    let is_mayhem_mode = read_bool(payload, offset).unwrap_or(false);
    offset += 1;
    let is_cashback_enabled = read_option_bool_idl(payload, offset).unwrap_or(false);

    let mint = get_account(0)?;
    let bonding_curve = get_account(2).unwrap_or_default();
    let user = get_account(5).unwrap_or_default();

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    let mayhem_program_id = get_account(9).unwrap_or_default();
    let (quote_mint, quote_vault, quote_token_program) =
        create_v2_quote_accounts_from_shred_accounts(accounts.len(), get_account);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve,
        user,
        creator,
        mint_authority: get_account(1).unwrap_or_default(),
        associated_bonding_curve: get_account(3).unwrap_or_default(),
        global: get_account(4).unwrap_or_default(),
        system_program: get_account(6).unwrap_or_default(),
        token_program: get_account(7).unwrap_or_default(),
        associated_token_program: get_account(8).unwrap_or_default(),
        mayhem_program_id,
        global_params: get_account(10).unwrap_or_default(),
        sol_vault: get_account(11).unwrap_or_default(),
        mayhem_state: get_account(12).unwrap_or_default(),
        mayhem_token_vault: get_account(13).unwrap_or_default(),
        event_authority: get_account(14).unwrap_or_default(),
        program: get_account(15).unwrap_or_default(),
        is_mayhem_mode,
        is_cashback_enabled,
        quote_mint,
        quote_vault,
        quote_token_program,
        ix_name: "create_v2".to_string(),
        ..Default::default()
    }))
}

#[inline]
fn parse_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
) -> Option<DexEvent> {
    const LEGACY_BUY_ACCOUNTS: usize = 16;
    if accounts.len() < LEGACY_BUY_ACCOUNTS {
        return None;
    }

    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (token_amount, sol_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(2)?;
    let is_created_buy = created_mints.contains(&mint);
    let is_mayhem_mode = mayhem_mints.contains(&mint);

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunBuy(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(3).unwrap_or_default(),
        bonding_curve_v2: get_account(16).unwrap_or_default(),
        associated_bonding_curve: get_account(4).unwrap_or_default(),
        associated_user: get_account(5).unwrap_or_default(),
        user: get_account(6).unwrap_or_default(),
        system_program: get_account(7).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: sol_amount,
        min_sol_output: 0,
        spendable_sol_in: 0,
        spendable_quote_in: 0,
        min_tokens_out: 0,
        fee_recipient: get_account(1).unwrap_or_default(),
        token_program: token_program_or_default(get_account(8).unwrap_or_default()),
        creator_vault: get_account(9).unwrap_or_default(),
        event_authority: get_account(10).unwrap_or_default(),
        program: get_account(11).unwrap_or_default(),
        global_volume_accumulator: get_account(12).unwrap_or_default(),
        user_volume_accumulator: get_account(13).unwrap_or_default(),
        fee_config: get_account(14).unwrap_or_default(),
        fee_program: get_account(15).unwrap_or_default(),
        is_buy: true,
        is_created_buy,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "buy".to_string(),
        mayhem_mode: is_mayhem_mode,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        buyback_fee_recipient: get_account(17).unwrap_or_default(),
        account: get_account(17).filter(|pk| *pk != Pubkey::default()),
        ..Default::default()
    }))
}

#[inline]
fn parse_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
) -> Option<DexEvent> {
    const LEGACY_SELL_ACCOUNTS: usize = 14;
    if accounts.len() < LEGACY_SELL_ACCOUNTS {
        return None;
    }

    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (token_amount, sol_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(2)?;
    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunSell(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(3).unwrap_or_default(),
        bonding_curve_v2: if accounts.len() >= 17 {
            get_account(15).unwrap_or_default()
        } else {
            get_account(14).unwrap_or_default()
        },
        associated_bonding_curve: get_account(4).unwrap_or_default(),
        associated_user: get_account(5).unwrap_or_default(),
        user: get_account(6).unwrap_or_default(),
        system_program: get_account(7).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: 0,
        min_sol_output: sol_amount,
        spendable_sol_in: 0,
        spendable_quote_in: 0,
        min_tokens_out: 0,
        fee_recipient: get_account(1).unwrap_or_default(),
        token_program: token_program_or_default(get_account(9).unwrap_or_default()),
        creator_vault: get_account(8).unwrap_or_default(),
        event_authority: get_account(10).unwrap_or_default(),
        program: get_account(11).unwrap_or_default(),
        global_volume_accumulator: Pubkey::default(),
        user_volume_accumulator: if accounts.len() >= 17 {
            get_account(14).unwrap_or_default()
        } else {
            Pubkey::default()
        },
        fee_config: get_account(12).unwrap_or_default(),
        fee_program: get_account(13).unwrap_or_default(),
        is_buy: false,
        is_created_buy: false,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "sell".to_string(),
        mayhem_mode: false,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        buyback_fee_recipient: if accounts.len() >= 17 {
            get_account(16).unwrap_or_default()
        } else {
            get_account(15).unwrap_or_default()
        },
        account: if accounts.len() >= 17 {
            get_account(16).filter(|pk| *pk != Pubkey::default())
        } else {
            get_account(15).filter(|pk| *pk != Pubkey::default())
        },
        ..Default::default()
    }))
}

#[inline]
fn parse_buy_exact_sol_in_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
) -> Option<DexEvent> {
    const LEGACY_BUY_ACCOUNTS: usize = 16;
    if accounts.len() < LEGACY_BUY_ACCOUNTS {
        return None;
    }

    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (sol_amount, token_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(2)?;
    let is_created_buy = created_mints.contains(&mint);
    let is_mayhem_mode = mayhem_mints.contains(&mint);

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunBuyExactSolIn(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: PUMPFUN_SOLSCAN_SOL_QUOTE_MINT,
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(3).unwrap_or_default(),
        bonding_curve_v2: get_account(16).unwrap_or_default(),
        associated_bonding_curve: get_account(4).unwrap_or_default(),
        associated_user: get_account(5).unwrap_or_default(),
        user: get_account(6).unwrap_or_default(),
        system_program: get_account(7).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: sol_amount,
        min_sol_output: 0,
        spendable_sol_in: sol_amount,
        spendable_quote_in: 0,
        min_tokens_out: token_amount,
        fee_recipient: get_account(1).unwrap_or_default(),
        token_program: token_program_or_default(get_account(8).unwrap_or_default()),
        creator_vault: get_account(9).unwrap_or_default(),
        event_authority: get_account(10).unwrap_or_default(),
        program: get_account(11).unwrap_or_default(),
        global_volume_accumulator: get_account(12).unwrap_or_default(),
        user_volume_accumulator: get_account(13).unwrap_or_default(),
        fee_config: get_account(14).unwrap_or_default(),
        fee_program: get_account(15).unwrap_or_default(),
        is_buy: true,
        is_created_buy,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "buy_exact_sol_in".to_string(),
        mayhem_mode: is_mayhem_mode,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        buyback_fee_recipient: get_account(17).unwrap_or_default(),
        account: get_account(17).filter(|pk| *pk != Pubkey::default()),
        ..Default::default()
    }))
}

/// `buy_v2`：27 个固定账户（IDL `buy_v2`）；mint=#1 bonding_curve=#10 user=#13 fee=#6 base_token_program=#3。
/// ShredStream can see shortened account lists, so only `mint` is required and all later accounts
/// are filled best-effort.
#[inline]
fn parse_buy_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
) -> Option<DexEvent> {
    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (token_amount, sol_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(1)?;
    let is_created_buy = created_mints.contains(&mint);
    let is_mayhem_mode = mayhem_mints.contains(&mint);

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunBuy(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: quote_mint_from_shred_v2_account(get_account(2)),
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(10).unwrap_or_default(),
        associated_bonding_curve: get_account(11).unwrap_or_default(),
        associated_quote_bonding_curve: get_account(12).unwrap_or_default(),
        associated_user: get_account(14).unwrap_or_default(),
        associated_quote_user: get_account(15).unwrap_or_default(),
        user: get_account(13).unwrap_or_default(),
        system_program: get_account(24).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: sol_amount,
        min_sol_output: 0,
        spendable_sol_in: 0,
        spendable_quote_in: 0,
        min_tokens_out: 0,
        fee_recipient: get_account(6).unwrap_or_default(),
        token_program: token_program_or_default(get_account(3).unwrap_or_default()),
        quote_token_program: token_program_or_default(get_account(4).unwrap_or_default()),
        associated_token_program: get_account(5).unwrap_or_default(),
        creator_vault: get_account(16).unwrap_or_default(),
        associated_quote_fee_recipient: get_account(7).unwrap_or_default(),
        buyback_fee_recipient: get_account(8).unwrap_or_default(),
        associated_quote_buyback_fee_recipient: get_account(9).unwrap_or_default(),
        associated_creator_vault: get_account(17).unwrap_or_default(),
        sharing_config: get_account(18).unwrap_or_default(),
        event_authority: get_account(25).unwrap_or_default(),
        program: get_account(26).unwrap_or_default(),
        global_volume_accumulator: get_account(19).unwrap_or_default(),
        user_volume_accumulator: get_account(20).unwrap_or_default(),
        associated_user_volume_accumulator: get_account(21).unwrap_or_default(),
        fee_config: get_account(22).unwrap_or_default(),
        fee_program: get_account(23).unwrap_or_default(),
        is_buy: true,
        is_created_buy,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "buy_v2".to_string(),
        mayhem_mode: is_mayhem_mode,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        account: None,
        ..Default::default()
    }))
}

#[inline]
fn parse_buy_exact_quote_in_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
    created_mints: &PumpMintSet,
    mayhem_mints: &PumpMintSet,
) -> Option<DexEvent> {
    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (sol_amount, token_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(1)?;
    let is_created_buy = created_mints.contains(&mint);
    let is_mayhem_mode = mayhem_mints.contains(&mint);

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunBuy(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: quote_mint_from_shred_v2_account(get_account(2)),
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(10).unwrap_or_default(),
        associated_bonding_curve: get_account(11).unwrap_or_default(),
        associated_quote_bonding_curve: get_account(12).unwrap_or_default(),
        associated_user: get_account(14).unwrap_or_default(),
        associated_quote_user: get_account(15).unwrap_or_default(),
        user: get_account(13).unwrap_or_default(),
        system_program: get_account(24).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: 0,
        quote_amount: sol_amount,
        min_sol_output: 0,
        spendable_sol_in: 0,
        spendable_quote_in: sol_amount,
        min_tokens_out: token_amount,
        fee_recipient: get_account(6).unwrap_or_default(),
        token_program: token_program_or_default(get_account(3).unwrap_or_default()),
        quote_token_program: token_program_or_default(get_account(4).unwrap_or_default()),
        associated_token_program: get_account(5).unwrap_or_default(),
        creator_vault: get_account(16).unwrap_or_default(),
        associated_quote_fee_recipient: get_account(7).unwrap_or_default(),
        buyback_fee_recipient: get_account(8).unwrap_or_default(),
        associated_quote_buyback_fee_recipient: get_account(9).unwrap_or_default(),
        associated_creator_vault: get_account(17).unwrap_or_default(),
        sharing_config: get_account(18).unwrap_or_default(),
        event_authority: get_account(25).unwrap_or_default(),
        program: get_account(26).unwrap_or_default(),
        global_volume_accumulator: get_account(19).unwrap_or_default(),
        user_volume_accumulator: get_account(20).unwrap_or_default(),
        associated_user_volume_accumulator: get_account(21).unwrap_or_default(),
        fee_config: get_account(22).unwrap_or_default(),
        fee_program: get_account(23).unwrap_or_default(),
        is_buy: true,
        is_created_buy,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "buy_exact_quote_in_v2".to_string(),
        mayhem_mode: is_mayhem_mode,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        account: None,
        ..Default::default()
    }))
}

/// `sell_v2`：26 个固定账户（IDL `sell_v2`）。
/// ShredStream can see shortened account lists, so only `mint` is required and all later accounts
/// are filled best-effort.
#[inline]
fn parse_sell_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    recv_us: i64,
) -> Option<DexEvent> {
    let get_account = |idx: usize| -> Option<Pubkey> { accounts.get(idx).copied() };

    let (token_amount, sol_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(1)?;

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: 0,
        grpc_recv_us: recv_us,
        recent_blockhash: None,
    };

    Some(DexEvent::PumpFunSell(PumpFunTradeEvent {
        metadata,
        mint,
        quote_mint: quote_mint_from_shred_v2_account(get_account(2)),
        global: get_account(0).unwrap_or_default(),
        bonding_curve: get_account(10).unwrap_or_default(),
        associated_bonding_curve: get_account(11).unwrap_or_default(),
        associated_quote_bonding_curve: get_account(12).unwrap_or_default(),
        associated_user: get_account(14).unwrap_or_default(),
        associated_quote_user: get_account(15).unwrap_or_default(),
        user: get_account(13).unwrap_or_default(),
        system_program: get_account(23).unwrap_or_default(),
        sol_amount,
        token_amount,
        amount: token_amount,
        max_sol_cost: 0,
        min_sol_output: sol_amount,
        spendable_sol_in: 0,
        spendable_quote_in: 0,
        min_tokens_out: 0,
        fee_recipient: get_account(6).unwrap_or_default(),
        token_program: token_program_or_default(get_account(3).unwrap_or_default()),
        quote_token_program: token_program_or_default(get_account(4).unwrap_or_default()),
        associated_token_program: get_account(5).unwrap_or_default(),
        creator_vault: get_account(16).unwrap_or_default(),
        associated_quote_fee_recipient: get_account(7).unwrap_or_default(),
        buyback_fee_recipient: get_account(8).unwrap_or_default(),
        associated_quote_buyback_fee_recipient: get_account(9).unwrap_or_default(),
        associated_creator_vault: get_account(17).unwrap_or_default(),
        sharing_config: get_account(18).unwrap_or_default(),
        event_authority: get_account(24).unwrap_or_default(),
        program: get_account(25).unwrap_or_default(),
        global_volume_accumulator: Pubkey::default(),
        user_volume_accumulator: get_account(19).unwrap_or_default(),
        associated_user_volume_accumulator: get_account(20).unwrap_or_default(),
        fee_config: get_account(21).unwrap_or_default(),
        fee_program: get_account(22).unwrap_or_default(),
        is_buy: false,
        is_created_buy: false,
        timestamp: 0,
        virtual_sol_reserves: 0,
        virtual_token_reserves: 0,
        real_sol_reserves: 0,
        real_token_reserves: 0,
        fee_basis_points: 0,
        fee: 0,
        creator: Pubkey::default(),
        creator_fee_basis_points: 0,
        creator_fee: 0,
        track_volume: false,
        total_unclaimed_tokens: 0,
        total_claimed_tokens: 0,
        current_sol_volume: 0,
        last_update_timestamp: 0,
        ix_name: "sell_v2".to_string(),
        mayhem_mode: false,
        cashback_fee_basis_points: 0,
        cashback: 0,
        is_cashback_coin: false,
        account: None,
        ..Default::default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::PUMPFUN_WSOL_QUOTE_MINT;
    use solana_sdk::hash::Hash;
    use solana_sdk::message::compiled_instruction::CompiledInstruction;
    use solana_sdk::message::{v0, MessageHeader};
    use solana_sdk::signature::Signature;
    use solana_sdk::transaction::VersionedTransaction;
    use std::str::FromStr;

    fn unique_accounts(n: usize) -> Vec<Pubkey> {
        (0..n).map(|_| Pubkey::new_unique()).collect()
    }

    fn ix_accounts(n: usize) -> Vec<u8> {
        (0..n).map(|i| i as u8).collect()
    }

    fn pk(s: &str) -> Pubkey {
        Pubkey::from_str(s).unwrap()
    }

    fn amount_data(first: u64, second: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&first.to_le_bytes());
        data.extend_from_slice(&second.to_le_bytes());
        data
    }

    fn instruction_data(discriminator: [u8; 8], first: u64, second: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&discriminator);
        data.extend_from_slice(&amount_data(first, second));
        data
    }

    fn str_arg(s: &str, out: &mut Vec<u8>) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    fn create_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::CREATE);
        str_arg("Alt Coin", &mut data);
        str_arg("ALT", &mut data);
        str_arg("https://example.invalid/alt.json", &mut data);
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        data
    }

    fn create_v2_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::CREATE_V2);
        str_arg("Alt Coin", &mut data);
        str_arg("ALT", &mut data);
        str_arg("https://example.invalid/alt.json", &mut data);
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        data.push(1);
        data.push(1);
        data
    }

    fn v0_tx(
        program_id_index: u8,
        account_keys: Vec<Pubkey>,
        ix_accounts: Vec<u8>,
        data: Vec<u8>,
    ) -> VersionedTransaction {
        VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(v0::Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 0,
                },
                account_keys,
                recent_blockhash: Hash::default(),
                instructions: vec![CompiledInstruction::new_from_raw_parts(
                    program_id_index,
                    data,
                    ix_accounts,
                )],
                address_table_lookups: Vec::new(),
            }),
        }
    }

    fn v0_tx_with_instructions(
        account_keys: Vec<Pubkey>,
        instructions: Vec<(u8, Vec<u8>, Vec<u8>)>,
    ) -> VersionedTransaction {
        VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(v0::Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 0,
                },
                account_keys,
                recent_blockhash: Hash::default(),
                instructions: instructions
                    .into_iter()
                    .map(|(program_id_index, accounts, data)| {
                        CompiledInstruction::new_from_raw_parts(program_id_index, data, accounts)
                    })
                    .collect(),
                address_table_lookups: Vec::new(),
            }),
        }
    }

    fn parse_shred_events_like_client(tx: &VersionedTransaction) -> Vec<DexEvent> {
        let mut events = Vec::new();
        parse_transaction_dex_events_with_filter(
            tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );
        crate::core::pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge(&mut events);
        events
    }

    #[test]
    fn unified_shred_outer_programs_cover_supported_protocols() {
        for program_id in [
            PUMPSWAP_PROGRAM_ID,
            PUMP_FEES_PROGRAM_ID,
            RAYDIUM_LAUNCHLAB_PROGRAM_ID,
            RAYDIUM_CPMM_PROGRAM_ID,
            RAYDIUM_CLMM_PROGRAM_ID,
            RAYDIUM_AMM_V4_PROGRAM_ID,
            ORCA_WHIRLPOOL_PROGRAM_ID,
            METEORA_POOLS_PROGRAM_ID,
            METEORA_DAMM_V2_PROGRAM_ID,
            METEORA_DLMM_PROGRAM_ID,
        ] {
            assert!(
                is_supported_unified_outer_program(&program_id, None),
                "ShredStream outer parser missing {program_id}"
            );
        }
    }

    #[test]
    fn unified_shred_outer_program_filter_skips_unrequested_protocols() {
        let raydium_only =
            EventTypeFilter::include_only(vec![crate::grpc::types::EventType::RaydiumCpmmSwap]);

        assert!(is_supported_unified_outer_program(&RAYDIUM_CPMM_PROGRAM_ID, Some(&raydium_only)));
        assert!(!is_supported_unified_outer_program(
            &ORCA_WHIRLPOOL_PROGRAM_ID,
            Some(&raydium_only)
        ));
    }

    #[test]
    fn shred_pumpfun_create_best_effort_keeps_static_mint_when_other_accounts_are_alt() {
        let mut static_keys = vec![Pubkey::new_unique(); 10];
        static_keys[9] = PROGRAM_ID_PUBKEY;
        let mint = static_keys[0];
        let mut ix_accounts = ix_accounts(10);
        ix_accounts[2] = 44;
        ix_accounts[7] = 45;
        ix_accounts[9] = 46;

        let tx = v0_tx(9, static_keys, ix_accounts, create_data());
        let mut events = Vec::new();
        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.mint, mint);
                assert_eq!(event.name, "Alt Coin");
                assert_eq!(event.bonding_curve, Pubkey::default());
                assert_eq!(event.user, Pubkey::default());
                assert_eq!(event.token_program, Pubkey::default());
                assert_eq!(event.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
                assert_eq!(event.ix_name, "create");
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_create_uses_instruction_order_accounts() {
        let mut static_keys = vec![Pubkey::new_unique(); 12];
        static_keys[9] = PROGRAM_ID_PUBKEY;
        let mint = static_keys[5];
        let bonding_curve = static_keys[3];
        let user = static_keys[11];
        let token_program = static_keys[4];
        let mut ix_accounts = vec![5, 1, 3, 2, 6, 7, 8, 11, 10, 4];
        ix_accounts[8] = 44;

        let tx = v0_tx(9, static_keys, ix_accounts, create_data());
        let mut events = Vec::new();
        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.mint, mint);
                assert_eq!(event.bonding_curve, bonding_curve);
                assert_eq!(event.user, user);
                assert_eq!(event.token_program, token_program);
                assert_eq!(event.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
                assert_eq!(event.ix_name, "create");
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_create_v2_uses_appended_quote_mint_only_for_19_accounts() {
        let mut static_keys = vec![Pubkey::new_unique(); 20];
        static_keys[19] = PROGRAM_ID_PUBKEY;
        static_keys[16] = PUMPFUN_WSOL_QUOTE_MINT;
        let tx = v0_tx(19, static_keys, ix_accounts(19), create_v2_data());
        let mut events = Vec::new();

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.ix_name, "create_v2");
                assert_eq!(event.quote_mint, PUMPFUN_WSOL_QUOTE_MINT);
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }

        let mut static_keys = vec![Pubkey::new_unique(); 20];
        static_keys[19] = PROGRAM_ID_PUBKEY;
        let mut alt_ix_accounts = ix_accounts(19);
        alt_ix_accounts[16] = 42;
        let tx = v0_tx(19, static_keys, alt_ix_accounts, create_v2_data());
        events.clear();

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.ix_name, "create_v2");
                assert_eq!(event.quote_mint, Pubkey::default());
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }

        let mut static_keys = vec![Pubkey::new_unique(); 17];
        static_keys[16] = PROGRAM_ID_PUBKEY;
        let tx = v0_tx(16, static_keys, ix_accounts(16), create_v2_data());
        events.clear();
        let signature =
            "H6azwLqtRtrnVNC5iwcjYM9idU3e9SRyLZXTwjfJGJxA4X7dZL7vyhFAJNvQy7bb6bmQNmFHUt1KkkPPmhdge3G";

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.ix_name, "create_v2");
                assert_eq!(event.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT, "{signature}");
                assert_eq!(event.quote_vault, Pubkey::default(), "{signature}");
                assert_eq!(event.quote_token_program, Pubkey::default(), "{signature}");
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_create_v2_rejects_program_id_as_quote_mint() {
        let mut static_keys = vec![Pubkey::new_unique(); 20];
        static_keys[19] = PROGRAM_ID_PUBKEY;
        static_keys[16] = PROGRAM_ID_PUBKEY;
        static_keys[17] = Pubkey::new_unique();
        static_keys[18] = Pubkey::new_unique();
        let tx = v0_tx(19, static_keys, ix_accounts(19), create_v2_data());

        let events = parse_shred_events_like_client(&tx);

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(event) => {
                assert_eq!(event.ix_name, "create_v2");
                assert_eq!(event.quote_mint, Pubkey::default());
                assert_eq!(event.quote_vault, Pubkey::default());
                assert_eq!(event.quote_token_program, Pubkey::default());
            }
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_create_v2_full_accounts_cover_real_quote_cases() {
        // These cases come from user-provided mainnet signatures:
        // 4GCVgY2F... / 5HwZKTwc... / 3jWGFYXT...: create_v2 accounts[16] = So111...12 (WSOL, 19 accounts)
        // 3MVawF6...: create_v2 accounts[16] = EPjF... (USDC)
        // 2dZAucK...: create_v2 accounts[16] = EPjF... (USDC, 19 accounts)
        // oY9YQbie... and 4h9kYj...: create_v2 accounts[16] = So111...12 (WSOL, 20 accounts)
        struct Case {
            signature: &'static str,
            name: &'static str,
            account_len: usize,
            mint: &'static str,
            user: &'static str,
            quote_mint: Pubkey,
            quote_vault: &'static str,
        }
        let token_2022_program = crate::accounts::program_ids::SPL_TOKEN_2022_PROGRAM_ID;
        let spl_token_program = crate::accounts::program_ids::SPL_TOKEN_PROGRAM_ID;
        let cases = [
            Case {
                signature: "4GCVgY2FnT1s4q5zemnPL4mzSbuhUTgQo9mc9jewhLZzsCXKe8ehz6xD4QDJE853CLrF6doJbf4JNwJVeEYLA4De",
                name: "wsol 19-account create_v2",
                account_len: 19,
                mint: "CGY36MoFU627gPH4TLM5NP4Xnvhz6Nesc71TQecPpump",
                user: "Aqje5DsN4u2PHmQxGF9PKfpsDGwQRCBhWeLKHCFhSMXk",
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault: "CWR85PmUfzNNgmNN9Ref8L8BvMibZ1tzchiT5bTZpJhn",
            },
            Case {
                signature: "5HwZKTwcGFjSBPugSX5hE9JSq5wKmUooK3tLXuEoyDDzrTvHu7op3XDbhBXuteiC5EePNPh8TC1j6Fns47YvnyeG",
                name: "wsol 19-account create_v2 exact quote buy",
                account_len: 19,
                mint: "7NSSfLGsjNHzKxrgggQ56C2UdKxJVJvrECJR3dsbBuuG",
                user: "2bBRwhGoL4fRZk6g8NnhBZywsF8PdLJnBRfWDCEMogD2",
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault: "6jFz2oefpJUE6opjA7vxs3iXou7YYyb6e6E4LN2BFs1W",
            },
            Case {
                signature: "3MVawF6EPtG7rEPXdsyQfQUBLv3epRVNpNS4tRE4uwTPMqLNPqhuABwxU3QZH4uD6CuVupcpGchpNRK5HTbHRLNK",
                name: "usdc 19-account create_v2",
                account_len: 19,
                mint: "FUsqvH5x8QUrxmJhspt6meQZtfBr17m2YsTFuVsYpump",
                user: "9Gg6Mf8tq9zLSpK8qccrQiue3iE7wmyeogKkGZpnz2w5",
                quote_mint: pk("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
                quote_vault: "7SLtvqMx4bPoWSbPcnWBWpBem3RXbKraWUsiApXjB1VL",
            },
            Case {
                signature: "3jWGFYXT5V33Qc2roEBFDRAWHeybDowr53dSdnYSRkrPdYybU7oyEH9BfgSRxkgFHVKmUjv4e5T33AEnhJvBCuP2",
                name: "wsol 19-account create_v2 with later buy",
                account_len: 19,
                mint: "5i8AZEBc8o5dhfnTQdD3QTVejgbjitwQ1ADHg1jZpump",
                user: "2b2N2p7xCS9ibDqxwYgXpDSTniJwwye7n93WYuzmr74s",
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault: "9QB9SyXGDbHUsvvF8XMbYH5ioJMHKHhXTjQDoL56uHT7",
            },
            Case {
                signature: "2dZAucKwr4n5Lqu3BtJ4P8JsjCDtUXJzthadddfURraEJRTgn6XWaTNUNBbgUfP5c2wcVdubqViQhr48eWsgRqPX",
                name: "usdc 19-account create_v2 exact quote buy",
                account_len: 19,
                mint: "DsE8Ptubc1HWWethf9ant4eV9YnofEv5kfGyLdj7jk2Y",
                user: "easy7tXgADWkRMNjFRS2XsLXUAaKH5tEPodh9g7kcX8",
                quote_mint: pk("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
                quote_vault: "8QTKfEBf5yChuos4eTzQPbV3jXveCu5GkNKLFoS8oS7t",
            },
            Case {
                signature: "oY9YQbie16Bw11GsqbAPVnW6YjMHAj3kP9sufjcuQjdfcU86iUY8CiSaDrvu4QXJFnGY4jqQc2Kc1YVuAzujvyv",
                name: "wsol 20-account create_v2",
                account_len: 20,
                mint: "Bv3zjsdJ5KuA9KsGirqssC8pVJwCeCeyLjo4Hqpfpump",
                user: "2SWqdMbn1FJVUMUEpuyP2St8BPRtqJYXJPWFfmZr486q",
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault: "9QdMAuwtpnHSzjTQcTkjU1GFSs2gNtR66sdQofFv5P7B",
            },
            Case {
                signature: "4h9kYjzYpqqyYZuFnjf14zRwrGyChCuKAYVy6a4ZBig19bydEYsHwp6VbiKqTzT3pLf6NXnf6E25dn1NiU8LR4YB",
                name: "wsol 20-account create_v2 with jit account",
                account_len: 20,
                mint: "6EvDE4a7Yw8F65oy6UhhN3JBshGk9tV3b2yxNyhypump",
                user: "2SWqdMbn1FJVUMUEpuyP2St8BPRtqJYXJPWFfmZr486q",
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault: "27jyvk4PUYjcDQkKn8VGT9zNdAxZWWjqALpRUpjMqc2y",
            },
        ];

        for case in cases {
            let mut static_keys = vec![Pubkey::new_unique(); case.account_len + 1];
            let program_idx = case.account_len as u8;
            static_keys[program_idx as usize] = PROGRAM_ID_PUBKEY;
            static_keys[0] = pk(case.mint);
            static_keys[5] = pk(case.user);
            static_keys[7] = token_2022_program;
            static_keys[16] = case.quote_mint;
            static_keys[17] = pk(case.quote_vault);
            if case.account_len > 18 {
                static_keys[18] = spl_token_program;
            }
            let mut accounts = ix_accounts(case.account_len);
            accounts[15] = program_idx;
            let tx = v0_tx(program_idx, static_keys, accounts, create_v2_data());

            let events = parse_shred_events_like_client(&tx);

            assert_eq!(events.len(), 1, "{}", case.name);
            match &events[0] {
                DexEvent::PumpFunCreate(event) => {
                    assert_eq!(event.ix_name, "create_v2", "{}", case.name);
                    assert_eq!(event.mint, pk(case.mint), "{}: {}", case.name, case.signature);
                    assert_eq!(event.user, pk(case.user), "{}: {}", case.name, case.signature);
                    assert_eq!(
                        event.token_program, token_2022_program,
                        "{}: {}",
                        case.name, case.signature
                    );
                    assert_eq!(
                        event.quote_mint, case.quote_mint,
                        "{}: {}",
                        case.name, case.signature
                    );
                    assert_eq!(
                        event.quote_vault,
                        pk(case.quote_vault),
                        "{}: {}",
                        case.name,
                        case.signature
                    );
                    assert_eq!(
                        event.quote_token_program, spl_token_program,
                        "{}: {}",
                        case.name, case.signature
                    );
                    assert_eq!(
                        event.program, PROGRAM_ID_PUBKEY,
                        "{}: {}",
                        case.name, case.signature
                    );
                }
                other => panic!("{}: expected PumpFunCreate, got {other:?}", case.name),
            }
        }
    }

    #[test]
    fn shred_pumpfun_create_v2_alt_quote_index_is_not_guessable() {
        // Real compiled-index pattern from the user-provided signatures:
        // - 4GCVgY2F... create_v2 has 19 accounts, but accounts[16] is global idx 27 (ALT).
        // - 5HwZKTwc... create_v2 has 19 accounts, but accounts[16] is global idx 28 (ALT).
        // - 3MVawF6... create_v2 has 19 accounts, but accounts[16] is global idx 30 (ALT).
        // - 2dZAucK... create_v2 has 19 accounts, but accounts[16] is global idx 33 (ALT).
        // - oY9YQbie... create_v2 has 20 accounts, but accounts[16] is global idx 27 (ALT).
        // - 3jWGFYXT... create_v2 has 19 accounts, but accounts[16] is global idx 30 (ALT).
        // - 4h9kYjzY... create_v2 has 20 accounts, but accounts[16] is global idx 27 (ALT).
        //
        // ShredStream currently receives a VersionedTransaction and only has static_account_keys().
        // ALT-loaded addresses are not in that list, so the hot path must not invent USDC/WSOL.
        for (signature, name, static_len, program_idx, quote_global_idx, account_len) in [
            (
                "4GCVgY2FnT1s4q5zemnPL4mzSbuhUTgQo9mc9jewhLZzsCXKe8ehz6xD4QDJE853CLrF6doJbf4JNwJVeEYLA4De",
                "wsol 19-account quote in ALT",
                15usize,
                12u8,
                27u8,
                19usize,
            ),
            (
                "5HwZKTwcGFjSBPugSX5hE9JSq5wKmUooK3tLXuEoyDDzrTvHu7op3XDbhBXuteiC5EePNPh8TC1j6Fns47YvnyeG",
                "wsol 19-account quote in ALT exact quote buy",
                20usize,
                15u8,
                28u8,
                19usize,
            ),
            (
                "3MVawF6EPtG7rEPXdsyQfQUBLv3epRVNpNS4tRE4uwTPMqLNPqhuABwxU3QZH4uD6CuVupcpGchpNRK5HTbHRLNK",
                "usdc 19-account quote in ALT",
                19usize,
                16u8,
                30u8,
                19usize,
            ),
            (
                "oY9YQbie16Bw11GsqbAPVnW6YjMHAj3kP9sufjcuQjdfcU86iUY8CiSaDrvu4QXJFnGY4jqQc2Kc1YVuAzujvyv",
                "wsol 20-account quote in ALT",
                15usize,
                12u8,
                27u8,
                20usize,
            ),
            (
                "3jWGFYXT5V33Qc2roEBFDRAWHeybDowr53dSdnYSRkrPdYybU7oyEH9BfgSRxkgFHVKmUjv4e5T33AEnhJvBCuP2",
                "wsol 19-account quote in ALT with later buy",
                18usize,
                13u8,
                30u8,
                19usize,
            ),
            (
                "2dZAucKwr4n5Lqu3BtJ4P8JsjCDtUXJzthadddfURraEJRTgn6XWaTNUNBbgUfP5c2wcVdubqViQhr48eWsgRqPX",
                "usdc 19-account quote in ALT exact quote buy",
                19usize,
                15u8,
                33u8,
                19usize,
            ),
            (
                "4h9kYjzYpqqyYZuFnjf14zRwrGyChCuKAYVy6a4ZBig19bydEYsHwp6VbiKqTzT3pLf6NXnf6E25dn1NiU8LR4YB",
                "wsol 20-account quote in ALT with jit account",
                15usize,
                12u8,
                27u8,
                20usize,
            ),
        ] {
            let mut static_keys = vec![Pubkey::new_unique(); static_len];
            static_keys[program_idx as usize] = PROGRAM_ID_PUBKEY;
            let mut ix_accounts = ix_accounts(account_len);
            ix_accounts[15] = program_idx;
            ix_accounts[16] = quote_global_idx;
            let tx = v0_tx(program_idx, static_keys, ix_accounts, create_v2_data());

            let events = parse_shred_events_like_client(&tx);

            assert_eq!(events.len(), 1, "{name}");
            match &events[0] {
                DexEvent::PumpFunCreate(event) => {
                    assert_eq!(event.ix_name, "create_v2", "{name}: {signature}");
                    assert_eq!(event.quote_mint, Pubkey::default(), "{name}: {signature}");
                    assert_eq!(event.quote_vault, Pubkey::default(), "{name}: {signature}");
                    assert_eq!(
                        event.quote_token_program,
                        Pubkey::default(),
                        "{name}: {signature}"
                    );
                }
                other => panic!("{name}: expected PumpFunCreate, got {other:?}"),
            }
        }
    }

    #[test]
    fn shred_pumpfun_create_v2_alt_quote_can_be_recovered_from_static_v2_trade() {
        let usdc = pk("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let mint = Pubkey::new_unique();
        let mut static_keys = vec![Pubkey::new_unique(); 35];
        static_keys[0] = mint;
        static_keys[16] = PROGRAM_ID_PUBKEY;
        static_keys[30] = usdc;

        let mut create_accounts = ix_accounts(19);
        create_accounts[0] = 0;
        create_accounts[15] = 16;
        create_accounts[16] = 40; // ALT in create_v2, unavailable to Shred static-only parser.

        let mut buy_v2_accounts = ix_accounts(27);
        buy_v2_accounts[1] = 0; // mint
        buy_v2_accounts[2] = 30; // quote mint is static in the follow-up v2 trade.
        buy_v2_accounts[26] = 16;

        let tx = v0_tx_with_instructions(
            static_keys,
            vec![
                (16, create_accounts, create_v2_data()),
                (16, buy_v2_accounts, instruction_data(discriminators::BUY_V2, 1, 2)),
            ],
        );

        let events = parse_shred_events_like_client(&tx);

        let create = events
            .iter()
            .find_map(|event| match event {
                DexEvent::PumpFunCreate(c) => Some(c),
                _ => None,
            })
            .expect("create event");
        assert_eq!(create.quote_mint, usdc);
    }

    #[test]
    fn shred_pumpfun_create_v2_alt_quote_stays_unknown_when_trade_quote_is_alt_too() {
        // Real compiled-index pattern from 3MVawF6... / 2dZAucK...:
        // create_v2 accounts[16] and follow-up buy_v2 accounts[2] both point to ALT-loaded USDC.
        let mint = Pubkey::new_unique();
        let mut static_keys = vec![Pubkey::new_unique(); 19];
        static_keys[0] = mint;
        static_keys[16] = PROGRAM_ID_PUBKEY;

        let mut create_accounts = ix_accounts(19);
        create_accounts[0] = 0;
        create_accounts[15] = 16;
        create_accounts[16] = 30;

        let mut buy_v2_accounts = ix_accounts(27);
        buy_v2_accounts[1] = 0;
        buy_v2_accounts[2] = 30;
        buy_v2_accounts[26] = 16;

        let tx = v0_tx_with_instructions(
            static_keys,
            vec![
                (16, create_accounts, create_v2_data()),
                (16, buy_v2_accounts, instruction_data(discriminators::BUY_V2, 1, 2)),
            ],
        );

        let events = parse_shred_events_like_client(&tx);

        let create = events
            .iter()
            .find_map(|event| match event {
                DexEvent::PumpFunCreate(c) => Some(c),
                _ => None,
            })
            .expect("create event");
        assert_eq!(create.quote_mint, Pubkey::default());
    }

    #[test]
    fn shred_best_effort_parses_when_program_id_is_alt_loaded() {
        let static_keys = vec![PROGRAM_ID_PUBKEY; 2];
        let created_mint = static_keys[0];
        let tx = v0_tx(7, static_keys, ix_accounts(10), create_data());
        let mut events = Vec::new();

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(e) => assert_eq!(e.mint, created_mint),
            other => panic!("expected PumpFunCreate, got {other:?}"),
        }
    }

    #[test]
    fn unknown_program_without_filter_stops_after_first_matching_candidate() {
        let static_keys = vec![Pubkey::new_unique(); 18];
        let ix_accounts = ix_accounts(18);
        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::BUY);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&200_u64.to_le_bytes());
        let tx = v0_tx(30, static_keys, ix_accounts, data);
        let mut events = Vec::new();

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            None,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunBuy(_)));
    }

    #[test]
    fn shred_pumpfun_sell_filter_covers_sell_v2() {
        let static_keys = vec![PROGRAM_ID_PUBKEY; 26];
        let filter =
            EventTypeFilter::include_only(vec![crate::grpc::types::EventType::PumpFunSell]);

        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::SELL_V2);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&200_u64.to_le_bytes());
        let mut events = Vec::new();

        dispatch_shred_outer(
            0,
            &ix_accounts(26),
            &data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunSell(_)));
    }

    #[test]
    fn unknown_program_outer_uses_filter_to_parse_matching_protocol() {
        let static_keys = vec![RAYDIUM_CPMM_PROGRAM_ID, Pubkey::new_unique()];
        let ix_accounts = vec![1, 42, 43, 44];
        let mut data = Vec::new();
        data.extend_from_slice(&crate::instr::raydium_cpmm::discriminators::SWAP_BASE_IN);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&90_u64.to_le_bytes());
        let tx = v0_tx(9, static_keys, ix_accounts, data);
        let filter =
            EventTypeFilter::include_only(vec![crate::grpc::types::EventType::RaydiumCpmmSwap]);
        let mut events = Vec::new();

        parse_transaction_dex_events_with_filter(
            &tx,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::RaydiumCpmmSwap(_)));
    }

    #[test]
    fn shred_pumpfun_trade_filter_normalizes_specific_variants_to_trade() {
        let static_keys = vec![PROGRAM_ID_PUBKEY; 27];
        let mut data = Vec::new();
        data.extend_from_slice(&discriminators::BUY_V2);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&200_u64.to_le_bytes());
        let filter =
            EventTypeFilter::include_only(vec![crate::grpc::types::EventType::PumpFunTrade]);
        let mut events = Vec::new();

        dispatch_shred_outer(
            0,
            &ix_accounts(27),
            &data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunTrade(_)));
    }

    #[test]
    fn shred_pumpfun_buy_filter_normalizes_all_buy_variants_to_buy() {
        let static_keys = vec![PROGRAM_ID_PUBKEY; 27];
        let filter = EventTypeFilter::include_only(vec![crate::grpc::types::EventType::PumpFunBuy]);

        let mut buy_data = Vec::new();
        buy_data.extend_from_slice(&discriminators::BUY_V2);
        buy_data.extend_from_slice(&100_u64.to_le_bytes());
        buy_data.extend_from_slice(&200_u64.to_le_bytes());
        let mut events = Vec::new();

        dispatch_shred_outer(
            0,
            &ix_accounts(27),
            &buy_data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunBuy(_)));

        let mut exact_data = Vec::new();
        exact_data.extend_from_slice(&discriminators::BUY_EXACT_SOL_IN);
        exact_data.extend_from_slice(&100_u64.to_le_bytes());
        exact_data.extend_from_slice(&200_u64.to_le_bytes());
        events.clear();

        dispatch_shred_outer(
            0,
            &ix_accounts(27),
            &exact_data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunBuy(_)));

        let mut exact_quote_data = Vec::new();
        exact_quote_data.extend_from_slice(&discriminators::BUY_EXACT_QUOTE_IN_V2);
        exact_quote_data.extend_from_slice(&100_u64.to_le_bytes());
        exact_quote_data.extend_from_slice(&200_u64.to_le_bytes());
        events.clear();

        dispatch_shred_outer(
            0,
            &ix_accounts(27),
            &exact_quote_data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            Some(&filter),
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::PumpFunBuy(_)));
    }

    #[test]
    fn non_pump_outer_accounts_keep_instruction_length_with_alt_defaults() {
        let static_keys = vec![RAYDIUM_CPMM_PROGRAM_ID, Pubkey::new_unique()];
        let ix_accounts = vec![1, 42, 43, 44];
        let mut data = Vec::new();
        data.extend_from_slice(&crate::instr::raydium_cpmm::discriminators::SWAP_BASE_IN);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&90_u64.to_le_bytes());
        let mut events = Vec::new();

        dispatch_shred_outer(
            0,
            &ix_accounts,
            &data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            None,
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::RaydiumCpmmSwap(_)));
    }

    #[test]
    fn non_pump_outer_skips_unsupported_discriminator_before_account_fill() {
        let static_keys = vec![RAYDIUM_CPMM_PROGRAM_ID, Pubkey::new_unique()];
        let ix_accounts = vec![99];
        let data = vec![0xff; 8];
        let mut events = Vec::new();

        dispatch_shred_outer(
            0,
            &ix_accounts,
            &data,
            &static_keys,
            Signature::default(),
            123,
            0,
            456,
            None,
            &PumpMintSet::new(),
            &PumpMintSet::new(),
            &mut events,
        );

        assert!(events.is_empty());
    }

    #[test]
    fn shred_pumpfun_trade_variants_are_specific_and_keep_exact_fields() {
        let accounts = unique_accounts(27);
        let no_created = PumpMintSet::new();
        let no_mayhem = PumpMintSet::new();

        let buy = parse_buy_instruction(
            &amount_data(100, 200),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
            &no_created,
            &no_mayhem,
        )
        .expect("buy");
        match buy {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.bonding_curve_v2, accounts[16]);
                assert_eq!(t.buyback_fee_recipient, accounts[17]);
                assert_eq!(t.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
            }
            other => panic!("expected buy variant, got {other:?}"),
        }

        let sell = parse_sell_instruction(
            &amount_data(300, 400),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
        )
        .expect("sell");
        match sell {
            DexEvent::PumpFunSell(t) => {
                assert_eq!(t.user_volume_accumulator, accounts[14]);
                assert_eq!(t.bonding_curve_v2, accounts[15]);
                assert_eq!(t.buyback_fee_recipient, accounts[16]);
                assert_eq!(t.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT);
            }
            other => panic!("expected sell variant, got {other:?}"),
        }

        let exact_quote = parse_buy_exact_quote_in_v2_instruction(
            &amount_data(500, 600),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
            &no_created,
            &no_mayhem,
        )
        .expect("exact quote buy");

        match exact_quote {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.ix_name, "buy_exact_quote_in_v2");
                assert_eq!(t.spendable_quote_in, 500);
                assert_eq!(t.min_tokens_out, 600);
                assert_eq!(t.max_sol_cost, 0);
                assert_eq!(t.quote_amount, 500);
                assert_eq!(t.quote_mint, accounts[2]);
                assert_eq!(t.associated_quote_user, accounts[15]);
                assert_eq!(t.associated_creator_vault, accounts[17]);
                assert_eq!(t.sharing_config, accounts[18]);
                assert_eq!(t.global_volume_accumulator, accounts[19]);
                assert_eq!(t.fee_program, accounts[23]);
            }
            other => panic!("expected exact buy variant, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_v2_buy_parses_short_account_lists_best_effort() {
        let accounts = unique_accounts(16);
        let no_created = PumpMintSet::new();
        let no_mayhem = PumpMintSet::new();

        let buy = parse_buy_v2_instruction(
            &amount_data(100, 200),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
            &no_created,
            &no_mayhem,
        )
        .expect("short buy_v2");
        match buy {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.ix_name, "buy_v2");
                assert_eq!(t.mint, accounts[1]);
                assert_eq!(t.amount, 100);
                assert_eq!(t.max_sol_cost, 200);
                assert_eq!(t.associated_creator_vault, Pubkey::default());
                assert_eq!(t.fee_program, Pubkey::default());
            }
            other => panic!("expected buy variant, got {other:?}"),
        }

        let exact_quote = parse_buy_exact_quote_in_v2_instruction(
            &amount_data(500, 600),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
            &no_created,
            &no_mayhem,
        )
        .expect("short exact quote buy");
        match exact_quote {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.ix_name, "buy_exact_quote_in_v2");
                assert_eq!(t.mint, accounts[1]);
                assert_eq!(t.spendable_quote_in, 500);
                assert_eq!(t.min_tokens_out, 600);
                assert_eq!(t.quote_amount, 500);
                assert_eq!(t.associated_creator_vault, Pubkey::default());
                assert_eq!(t.fee_program, Pubkey::default());
            }
            other => panic!("expected exact buy variant, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_v2_sell_parses_short_account_lists_best_effort() {
        let accounts = unique_accounts(16);

        let sell = parse_sell_v2_instruction(
            &amount_data(300, 400),
            &accounts,
            Signature::default(),
            1,
            0,
            9,
        )
        .expect("short sell_v2");

        match sell {
            DexEvent::PumpFunSell(t) => {
                assert_eq!(t.ix_name, "sell_v2");
                assert_eq!(t.mint, accounts[1]);
                assert_eq!(t.amount, 300);
                assert_eq!(t.min_sol_output, 400);
                assert_eq!(t.associated_creator_vault, Pubkey::default());
                assert_eq!(t.fee_program, Pubkey::default());
            }
            other => panic!("expected sell variant, got {other:?}"),
        }
    }

    #[test]
    fn shred_pumpfun_legacy_trade_rejects_short_account_lists() {
        let accounts = unique_accounts(16);
        let no_created = PumpMintSet::new();
        let no_mayhem = PumpMintSet::new();

        assert!(parse_buy_instruction(
            &amount_data(100, 200),
            &accounts[..15],
            Signature::default(),
            1,
            0,
            9,
            &no_created,
            &no_mayhem,
        )
        .is_none());

        assert!(parse_sell_instruction(
            &amount_data(300, 400),
            &accounts[..13],
            Signature::default(),
            1,
            0,
            9,
        )
        .is_none());
    }
}
