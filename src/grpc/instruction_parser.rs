//! Instruction 解析器 - 完整支持 instruction + inner instruction
//!
//! 设计原则：
//! - 简洁：单一入口函数，清晰的解析流程
//! - 高性能：零拷贝，内联优化，并行处理
//! - 可读性：每个步骤都有明确的注释

use crate::core::{
    events::*, merger::try_merge_events, pumpfun_fee_enrich::enrich_pumpfun_same_tx_post_merge,
};
use crate::grpc::types::EventTypeFilter;
use crate::instr::read_pubkey_fast;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

#[derive(Debug)]
struct IndexedInstructionEvent {
    outer_idx: usize,
    inner_idx: Option<usize>,
    stack_height: Option<u32>,
    is_dlmm_event_cpi: bool,
    event: DexEvent,
}

#[inline(always)]
fn is_dlmm_event(event: &DexEvent) -> bool {
    matches!(
        event,
        DexEvent::MeteoraDlmmSwap(_)
            | DexEvent::MeteoraDlmmAddLiquidity(_)
            | DexEvent::MeteoraDlmmRemoveLiquidity(_)
            | DexEvent::MeteoraDlmmInitializePool(_)
            | DexEvent::MeteoraDlmmInitializeBinArray(_)
            | DexEvent::MeteoraDlmmCreatePosition(_)
            | DexEvent::MeteoraDlmmClosePosition(_)
            | DexEvent::MeteoraDlmmClaimFee(_)
    )
}

/// 解析交易中的所有指令事件（instruction + inner instruction）
///
/// # 解析流程
/// 1. 解析主指令（outer instructions）- 8字节 discriminator
/// 2. 解析内部指令（inner instructions）- 16字节 discriminator
/// 3. 合并相关事件（instruction + inner instruction）
/// 4. 填充账户上下文
///
/// # 性能优化
/// - 零分配泄漏：`program_invokes` 全程 `Pubkey` 键，与账户填充 / `fill_data` 共用同一表
/// - 零拷贝读取指令账户字节、`read_pubkey_fast` 解码
/// - 热路径 `#[inline]`
/// - `should_parse_instructions` 提前跳过整段 ix 解析
#[inline]
pub fn parse_instructions_enhanced(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let Some(tx) = transaction else { return Vec::new() };
    let Some(msg) = &tx.message else { return Vec::new() };

    let recent_blockhash = if msg.recent_blockhash.is_empty() {
        None
    } else {
        Some(bs58::encode(&msg.recent_blockhash).into_string())
    };

    // 提前检查：是否需要解析 instruction（根据 filter）
    if !should_parse_instructions(filter) {
        return Vec::new();
    }

    // 与 log 解析一致：同笔交易内若有 PumpFun create，则本 tx 的 buy 事件标记为 is_created_buy（创建者首次买入）
    let is_created_buy = crate::logs::optimized_matcher::detect_pumpfun_create(&meta.log_messages);

    // 构建账户查找表
    let keys_len = msg.account_keys.len();
    let writable_len = meta.loaded_writable_addresses.len();
    let get_key = |i: usize| -> Option<&Vec<u8>> {
        if i < keys_len {
            msg.account_keys.get(i)
        } else if i < keys_len + writable_len {
            meta.loaded_writable_addresses.get(i - keys_len)
        } else {
            meta.loaded_readonly_addresses.get(i - keys_len - writable_len)
        }
    };

    let mut result = Vec::with_capacity(8);
    let mut invokes: HashMap<Pubkey, Vec<(i32, i32)>> = HashMap::with_capacity(8);

    // 步骤 1: 解析所有主指令
    for (i, ix) in msg.instructions.iter().enumerate() {
        let pid = get_key(ix.program_id_index as usize)
            .map_or(Pubkey::default(), |k| read_pubkey_fast(k));

        invokes.entry(pid).or_default().push((i as i32, -1));

        // 解析主指令（8字节 discriminator）
        if let Some(event) = parse_outer_instruction(
            &ix.data,
            &pid,
            sig,
            slot,
            tx_idx,
            block_us,
            grpc_us,
            &ix.accounts,
            &get_key,
            filter,
            is_created_buy,
        ) {
            result.push(IndexedInstructionEvent {
                outer_idx: i,
                inner_idx: None,
                stack_height: Some(1),
                is_dlmm_event_cpi: false,
                event,
            });
        }
    }

    // 步骤 2: 解析所有 inner instructions
    for inner in &meta.inner_instructions {
        let outer_idx = inner.index as usize;

        for (j, inner_ix) in inner.instructions.iter().enumerate() {
            let pid = get_key(inner_ix.program_id_index as usize)
                .map_or(Pubkey::default(), |k| read_pubkey_fast(k));

            invokes.entry(pid).or_default().push((outer_idx as i32, j as i32));

            let event = parse_inner_compiled_instruction_if_supported(
                &inner_ix.data,
                &pid,
                sig,
                slot,
                tx_idx,
                block_us,
                grpc_us,
                &inner_ix.accounts,
                &get_key,
                filter,
            )
            .or_else(|| {
                parse_inner_instruction(
                    &inner_ix.data,
                    &pid,
                    sig,
                    slot,
                    tx_idx,
                    block_us,
                    grpc_us,
                    filter,
                    is_created_buy,
                )
            });

            if let Some(event) = event {
                result.push(IndexedInstructionEvent {
                    outer_idx,
                    inner_idx: Some(j),
                    stack_height: inner_ix.stack_height,
                    is_dlmm_event_cpi: pid == crate::instr::program_ids::METEORA_DLMM_PROGRAM_ID
                        && crate::instr::all_inner::meteora_dlmm::is_event_cpi(&inner_ix.data),
                    event,
                });
            }
        }
    }

    // 步骤 3: 合并相关事件（instruction + inner instruction）
    let mut merged = merge_instruction_events(result);
    enrich_pumpfun_same_tx_post_merge(&mut merged);

    for e in merged.iter_mut() {
        if let Some(m) = e.metadata_mut() {
            m.recent_blockhash = recent_blockhash.clone();
        }
    }

    // 步骤 4: 填充账户上下文（invokes 与 fill_data 均使用 Pubkey 键，无堆泄漏）
    let mut final_result = Vec::with_capacity(merged.len());
    for mut event in merged {
        crate::core::account_dispatcher::fill_accounts_with_owned_keys(
            &mut event,
            meta,
            transaction,
            &invokes,
        );
        crate::core::common_filler::fill_data(&mut event, meta, transaction, &invokes);
        final_result.push(event);
    }

    final_result
}

// ============================================================================
// 辅助函数
// ============================================================================

#[inline(always)]
fn parse_compiled_instruction<'a>(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    account_indices: &[u8],
    get_key: &dyn Fn(usize) -> Option<&'a Vec<u8>>,
    filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    // 检查指令数据长度（至少8字节 discriminator）
    if data.len() < 8 {
        return None;
    }

    // 常见 DEX 指令账户数远小于 64；栈上缓冲避免每笔 outer 一次 Vec 分配
    const STACK_CAP: usize = 64;
    if account_indices.len() <= STACK_CAP {
        let mut stack = [Pubkey::default(); STACK_CAP];
        let mut n = 0usize;
        for &idx in account_indices {
            let k = get_key(idx as usize)?;
            stack[n] = read_pubkey_fast(k);
            n += 1;
        }
        crate::instr::parse_instruction_unified(
            data,
            &stack[..n],
            sig,
            slot,
            tx_idx,
            block_us,
            grpc_us,
            filter,
            program_id,
        )
    } else {
        let accounts: Vec<Pubkey> = account_indices
            .iter()
            .map(|&idx| get_key(idx as usize).map(|k| read_pubkey_fast(k)))
            .collect::<Option<_>>()?;
        crate::instr::parse_instruction_unified(
            data, &accounts, sig, slot, tx_idx, block_us, grpc_us, filter, program_id,
        )
    }
}

#[inline(always)]
fn is_supported_inner_compiled_instruction(data: &[u8], program_id: &Pubkey) -> bool {
    crate::instr::normal_instruction_data_may_parse(program_id, data)
}

#[inline(always)]
fn parse_inner_compiled_instruction_if_supported<'a>(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    account_indices: &[u8],
    get_key: &dyn Fn(usize) -> Option<&'a Vec<u8>>,
    filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    if !is_supported_inner_compiled_instruction(data, program_id) {
        return None;
    }
    parse_compiled_instruction(
        data,
        program_id,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        account_indices,
        get_key,
        filter,
    )
}

/// 解析单个主指令（outer instruction）
///
/// 主指令使用 8 字节 discriminator
#[inline(always)]
fn parse_outer_instruction<'a>(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    account_indices: &[u8],
    get_key: &dyn Fn(usize) -> Option<&'a Vec<u8>>,
    filter: Option<&EventTypeFilter>,
    _is_created_buy: bool,
) -> Option<DexEvent> {
    parse_compiled_instruction(
        data,
        program_id,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        account_indices,
        get_key,
        filter,
    )
}

/// 解析单个 inner instruction
///
/// Inner instructions 使用 16 字节 discriminator（前8字节是event hash，后8字节是magic）
#[inline(always)]
fn parse_inner_instruction(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
) -> Option<DexEvent> {
    // 检查数据长度（至少16字节 discriminator）
    if data.len() < 16 {
        return None;
    }

    let metadata = EventMetadata {
        signature: sig,
        slot,
        tx_index: tx_idx,
        block_time_us: block_us.unwrap_or(0),
        grpc_recv_us: grpc_us,
        recent_blockhash: None, // set later on merged events in parse_instructions_enhanced
    };

    // 提取 16 字节 discriminator
    let mut discriminator = [0u8; 16];
    discriminator.copy_from_slice(&data[..16]);
    let inner_data = &data[16..];

    use crate::instr::{all_inner, program_ids, pump_amm_inner, pump_inner, raydium_clmm_inner};

    // 根据 program_id 路由到对应的 inner instruction 解析器
    let event = if *program_id == program_ids::PUMPFUN_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_pumpfun() {
                return None;
            }
        }
        pump_inner::parse_pumpfun_inner_instruction(
            &discriminator,
            inner_data,
            metadata,
            is_created_buy,
        )
    } else if *program_id == program_ids::PUMPSWAP_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_pumpswap() {
                return None;
            }
        }
        pump_amm_inner::parse_pumpswap_inner_instruction(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::PUMP_FEES_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_pump_fees() {
                return None;
            }
        }
        all_inner::pump_fees::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::RAYDIUM_CLMM_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_raydium_clmm() {
                return None;
            }
        }
        raydium_clmm_inner::parse_raydium_clmm_inner_instruction(
            &discriminator,
            inner_data,
            metadata,
        )
    } else if *program_id == program_ids::RAYDIUM_CPMM_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_raydium_cpmm() {
                return None;
            }
        }
        all_inner::raydium_cpmm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::RAYDIUM_AMM_V4_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_raydium_amm_v4() {
                return None;
            }
        }
        all_inner::raydium_amm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::ORCA_WHIRLPOOL_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_orca_whirlpool() {
                return None;
            }
        }
        all_inner::orca::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::METEORA_POOLS_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_meteora_pools() {
                return None;
            }
        }
        all_inner::meteora_amm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::METEORA_DAMM_V2_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_meteora_damm_v2() {
                return None;
            }
        }
        all_inner::meteora_damm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::METEORA_DLMM_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_meteora_dlmm() {
                return None;
            }
        }
        all_inner::meteora_dlmm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_raydium_launchlab() {
                return None;
            }
        }
        all_inner::raydium_launchlab::parse(&discriminator, inner_data, metadata)
    } else {
        None
    };

    if filter.map(|f| event.as_ref().is_some_and(|e| f.should_include_dex_event(e))).unwrap_or(true)
    {
        event
    } else {
        None
    }
}

/// 合并相关的 instruction 和 inner instruction 事件
///
/// 合并策略：
/// 1. 同一个 outer_idx 的 instruction 和 inner instruction 可以合并
/// 2. Inner instruction 在 outer instruction 之后出现（排序保证主指令在前）
/// 3. 同一 outer 下若有多个 inner，依次链式合并进同一条事件，再输出
/// 4. 合并后返回更完整的事件
#[inline(always)]
fn merge_instruction_events(events: Vec<IndexedInstructionEvent>) -> Vec<DexEvent> {
    if events.is_empty() {
        return Vec::new();
    }

    // 按 (outer_idx, inner_idx) 排序，确保顺序：同一 outer 下 **主指令在前、inner 在后**
    // （`None` 若用 MAX 会把 outer 排到 inner 后面，导致无法 merge）
    let mut events = events;
    events.sort_unstable_by_key(|event| {
        (event.outer_idx, event.inner_idx.map_or(0, |inner_idx| inner_idx + 1))
    });

    let mut result = Vec::with_capacity(events.len());
    let mut outer_target: Option<(usize, usize)> = None;
    // Solana's instruction stack is shallow; keep DLMM parent candidates on
    // the stack so nested DLMM CPIs do not require a hot-path heap allocation.
    let mut dlmm_targets: [Option<(usize, Option<u32>, usize)>; 8] = [None; 8];
    let mut dlmm_targets_len = 0usize;

    for indexed in events {
        let IndexedInstructionEvent {
            outer_idx,
            inner_idx,
            stack_height,
            is_dlmm_event_cpi,
            event,
        } = indexed;
        match inner_idx {
            None => {
                let is_dlmm = is_dlmm_event(&event);
                let target_idx = result.len();
                result.push(event);
                outer_target = Some((outer_idx, target_idx));
                dlmm_targets_len = 0;
                if is_dlmm {
                    dlmm_targets[0] = Some((outer_idx, stack_height, target_idx));
                    dlmm_targets_len = 1;
                }
            }
            Some(_) => {
                if is_dlmm_event_cpi {
                    let target = (0..dlmm_targets_len).rev().find_map(|idx| {
                        let (target_outer, target_height, target_idx) = dlmm_targets[idx]?;
                        let is_direct_child = match (target_height, stack_height) {
                            (Some(parent), Some(child)) => child == parent + 1,
                            _ => true,
                        };
                        (target_outer == outer_idx && is_direct_child).then_some((idx, target_idx))
                    });
                    if let Some((candidate_idx, target_idx)) = target {
                        dlmm_targets_len = candidate_idx + 1;
                        if target_idx < result.len() {
                            match try_merge_events(&mut result[target_idx], event) {
                                Ok(()) => continue,
                                Err(event) => result.push(event),
                            }
                            continue;
                        }
                    }
                    result.push(event);
                    continue;
                }

                let is_dlmm = is_dlmm_event(&event);
                let target_idx = if let Some((target_outer, target_idx)) = outer_target {
                    if target_outer == outer_idx {
                        match try_merge_events(&mut result[target_idx], event) {
                            Ok(()) => target_idx,
                            Err(event) => {
                                let target_idx = result.len();
                                result.push(event);
                                target_idx
                            }
                        }
                    } else {
                        let target_idx = result.len();
                        result.push(event);
                        target_idx
                    }
                } else {
                    let target_idx = result.len();
                    result.push(event);
                    target_idx
                };

                if is_dlmm {
                    if let Some(height) = stack_height {
                        while dlmm_targets_len > 0 {
                            let Some((candidate_outer, candidate_height, _)) =
                                dlmm_targets[dlmm_targets_len - 1]
                            else {
                                break;
                            };
                            if candidate_outer != outer_idx
                                || candidate_height.is_some_and(|candidate| candidate >= height)
                            {
                                dlmm_targets_len -= 1;
                            } else {
                                break;
                            }
                        }
                    } else {
                        dlmm_targets_len = 0;
                    }
                    if dlmm_targets_len < dlmm_targets.len() {
                        dlmm_targets[dlmm_targets_len] =
                            Some((outer_idx, stack_height, target_idx));
                        dlmm_targets_len += 1;
                    }
                }
            }
        }
    }

    result
}

/// 检查是否需要解析 instructions（根据 filter）
#[inline(always)]
fn should_parse_instructions(filter: Option<&EventTypeFilter>) -> bool {
    // 如果没有 filter，总是解析
    let Some(filter) = filter else { return true };

    // 如果 filter.include_only 为空，总是解析
    if filter.include_only.is_none() {
        return true;
    }

    // PumpFun：outer BUY/SELL carries instruction args while inner/log TradeEvent
    // carries executed amounts. Parse both and merge them by order.
    if filter.includes_pumpfun() {
        return true;
    }

    if filter.includes_pump_fees() {
        return true;
    }

    filter.includes_pumpswap()
        || filter.includes_raydium_launchlab()
        || filter.includes_raydium_cpmm()
        || filter.includes_raydium_clmm()
        || filter.includes_raydium_amm_v4()
        || filter.includes_orca_whirlpool()
        || filter.includes_meteora_pools()
        || filter.includes_meteora_damm_v2()
        || filter.includes_meteora_dlmm()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{PUMPFUN_SOLSCAN_SOL_QUOTE_MINT, PUMPFUN_WSOL_QUOTE_MINT};
    use yellowstone_grpc_proto::prelude::{
        CompiledInstruction, InnerInstruction, InnerInstructions, Message, MessageHeader,
    };

    fn pk(s: &str) -> Pubkey {
        s.parse().unwrap()
    }

    fn usdc_mint() -> Pubkey {
        pk("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
    }

    fn pubkey_bytes(key: Pubkey) -> Vec<u8> {
        key.to_bytes().to_vec()
    }

    fn decode_b58(s: &str) -> Vec<u8> {
        bs58::decode(s).into_vec().unwrap()
    }

    fn str_arg(s: &str, out: &mut Vec<u8>) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    fn create_v2_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&crate::instr::pump::discriminators::CREATE_V2);
        str_arg("Alt Coin", &mut data);
        str_arg("ALT", &mut data);
        str_arg("https://example.invalid/alt.json", &mut data);
        data.extend_from_slice(Pubkey::new_unique().as_ref());
        data.push(1);
        data.push(1);
        data
    }

    fn grpc_pumpfun_create_v2_tx(
        static_len: usize,
        writable_len: usize,
        program_idx: u8,
        ix_accounts: Vec<u8>,
        account_overrides: Vec<(usize, Pubkey)>,
    ) -> (TransactionStatusMeta, Option<Transaction>) {
        let mut account_keys: Vec<Pubkey> = (0..static_len).map(|_| Pubkey::new_unique()).collect();
        account_keys[program_idx as usize] = crate::instr::program_ids::PUMPFUN_PROGRAM_ID;
        let readonly_len = account_overrides
            .iter()
            .filter(|(global_idx, _)| *global_idx >= static_len + writable_len)
            .map(|(global_idx, _)| global_idx - static_len - writable_len + 1)
            .max()
            .unwrap_or_default();
        let mut loaded_writable = vec![Pubkey::new_unique(); writable_len];
        let mut loaded_readonly = vec![Pubkey::new_unique(); readonly_len];
        for (global_idx, key) in account_overrides {
            if global_idx < static_len {
                account_keys[global_idx] = key;
            } else if global_idx < static_len + writable_len {
                loaded_writable[global_idx - static_len] = key;
            } else {
                loaded_readonly[global_idx - static_len - writable_len] = key;
            }
        }

        let meta = TransactionStatusMeta {
            loaded_writable_addresses: loaded_writable.into_iter().map(pubkey_bytes).collect(),
            loaded_readonly_addresses: loaded_readonly.into_iter().map(pubkey_bytes).collect(),
            ..Default::default()
        };
        let tx = Transaction {
            signatures: vec![Signature::default().as_ref().to_vec()],
            message: Some(Message {
                header: Some(MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 0,
                }),
                account_keys: account_keys.into_iter().map(pubkey_bytes).collect(),
                recent_blockhash: vec![0; 32],
                instructions: vec![CompiledInstruction {
                    program_id_index: program_idx as u32,
                    accounts: ix_accounts,
                    data: create_v2_data(),
                }],
                versioned: true,
                address_table_lookups: Vec::new(),
            }),
        };
        (meta, Some(tx))
    }

    fn create_v2_accounts(
        account_len: usize,
        program_idx: u8,
        mint_idx: u8,
        user_idx: u8,
        token_program_idx: u8,
        quote_tail: Option<(u8, u8, u8)>,
    ) -> Vec<u8> {
        let mut accounts: Vec<u8> = (0..account_len).map(|i| i as u8).collect();
        accounts[0] = mint_idx;
        accounts[5] = user_idx;
        accounts[7] = token_program_idx;
        if account_len > 15 {
            accounts[15] = program_idx;
        }
        if let Some((quote_idx, quote_vault_idx, quote_token_program_idx)) = quote_tail {
            accounts[16] = quote_idx;
            accounts[17] = quote_vault_idx;
            accounts[18] = quote_token_program_idx;
        }
        accounts
    }

    fn parse_create_v2_from_grpc(
        meta: &TransactionStatusMeta,
        tx: &Option<Transaction>,
    ) -> crate::core::events::PumpFunCreateTokenEvent {
        let events = parse_instructions_enhanced(
            meta,
            tx,
            Signature::default(),
            123,
            0,
            Some(456),
            789,
            None,
        );
        assert_eq!(events.len(), 1);
        match &events[0] {
            DexEvent::PumpFunCreate(e) => {
                assert_eq!(e.ix_name, "create_v2");
                e.clone()
            }
            DexEvent::PumpFunCreateV2(e) => {
                assert_eq!(e.ix_name, "create_v2");
                crate::core::events::PumpFunCreateTokenEvent {
                    metadata: e.metadata.clone(),
                    name: e.name.clone(),
                    symbol: e.symbol.clone(),
                    uri: e.uri.clone(),
                    mint: e.mint,
                    bonding_curve: e.bonding_curve,
                    user: e.user,
                    creator: e.creator,
                    timestamp: e.timestamp,
                    virtual_token_reserves: e.virtual_token_reserves,
                    virtual_sol_reserves: e.virtual_sol_reserves,
                    real_token_reserves: e.real_token_reserves,
                    token_total_supply: e.token_total_supply,
                    mint_authority: e.mint_authority,
                    associated_bonding_curve: e.associated_bonding_curve,
                    global: e.global,
                    system_program: e.system_program,
                    token_program: e.token_program,
                    associated_token_program: e.associated_token_program,
                    mayhem_program_id: e.mayhem_program_id,
                    global_params: e.global_params,
                    sol_vault: e.sol_vault,
                    mayhem_state: e.mayhem_state,
                    mayhem_token_vault: e.mayhem_token_vault,
                    event_authority: e.event_authority,
                    program: e.program,
                    quote_mint: e.quote_mint,
                    quote_vault: e.quote_vault,
                    quote_token_program: e.quote_token_program,
                    virtual_quote_reserves: e.virtual_quote_reserves,
                    ix_name: e.ix_name.clone(),
                    is_mayhem_mode: e.is_mayhem_mode,
                    is_cashback_enabled: e.is_cashback_enabled,
                    observed_fee_recipient: e.observed_fee_recipient,
                }
            }
            other => panic!("expected PumpFun create_v2 event, got {other:?}"),
        }
    }

    #[test]
    fn test_should_parse_instructions() {
        // 无 filter - 应该解析
        assert!(should_parse_instructions(None));

        // 有 filter 但 include_only 为空 - 应该解析
        let filter = EventTypeFilter { include_only: None, exclude_types: None };
        assert!(should_parse_instructions(Some(&filter)));

        // 包含需要 instruction 解析的事件类型
        use crate::grpc::types::EventType;
        let filter = EventTypeFilter {
            include_only: Some(vec![EventType::PumpFunMigrate]),
            exclude_types: None,
        };
        assert!(should_parse_instructions(Some(&filter)));

        // PumpFun 订阅：需要 instruction+inner，避免仅日志时截断丢腿
        let filter = EventTypeFilter {
            include_only: Some(vec![EventType::PumpFunTrade]),
            exclude_types: None,
        };
        assert!(should_parse_instructions(Some(&filter)));

        for event_type in [
            EventType::PumpSwapTrade,
            EventType::PumpFeesUpdateFeeShares,
            EventType::RaydiumLaunchlabTrade,
            EventType::RaydiumCpmmSwap,
            EventType::RaydiumClmmSwap,
            EventType::RaydiumAmmV4Swap,
            EventType::OrcaWhirlpoolSwap,
            EventType::MeteoraPoolsSwap,
            EventType::MeteoraDammV2Swap,
            EventType::MeteoraDammV2InitializePool,
            EventType::MeteoraDlmmSwap,
        ] {
            let filter = EventTypeFilter::include_only(vec![event_type]);
            assert!(
                should_parse_instructions(Some(&filter)),
                "instruction parsing should be enabled for {event_type:?}"
            );
        }

        let filter = EventTypeFilter::include_only(vec![EventType::MeteoraDbcSwap]);
        assert!(
            !should_parse_instructions(Some(&filter)),
            "DBC events are log-only until an instruction parser is implemented"
        );

        let filter = EventTypeFilter::include_only(vec![
            EventType::AccountPumpFunGlobal,
            EventType::AccountRaydiumClmmPoolState,
            EventType::AccountRaydiumCpmmPoolState,
            EventType::AccountOrcaWhirlpool,
        ]);
        assert!(
            !should_parse_instructions(Some(&filter)),
            "account-only non-Pump filters should stay on the account update path"
        );
    }

    #[test]
    fn test_merge_instruction_events() {
        use solana_sdk::signature::Signature;

        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        // 模拟：outer instruction + inner instruction（应该合并）
        let outer_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            bonding_curve: Pubkey::new_unique(),
            ..Default::default()
        });

        let inner_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            sol_amount: 1000,
            token_amount: 2000,
            ..Default::default()
        });

        let events = vec![
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: None,
                stack_height: Some(1),
                is_dlmm_event_cpi: false,
                event: outer_event,
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(0),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: inner_event,
            },
        ];

        let result = merge_instruction_events(events);

        // 应该合并为1个事件
        assert_eq!(result.len(), 1);

        // 验证合并结果包含两者的数据
        if let DexEvent::PumpFunTrade(trade) = &result[0] {
            assert_eq!(trade.sol_amount, 1000); // 来自 inner
            assert_eq!(trade.token_amount, 2000); // 来自 inner
            assert_ne!(trade.bonding_curve, Pubkey::default()); // 来自 outer
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn test_merge_instruction_events_chains_multiple_inners_same_outer() {
        use solana_sdk::signature::Signature;

        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        let bc = Pubkey::new_unique();
        let fee = Pubkey::new_unique();

        let outer_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            bonding_curve: bc,
            ..Default::default()
        });

        let inner_trade = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            sol_amount: 1000,
            token_amount: 2000,
            is_buy: true,
            ..Default::default()
        });

        // 第二段 inner 仅有 fee_recipient，无成交量 —— 不应抹掉第一段金额
        let inner_fee_only = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            fee_recipient: fee,
            ..Default::default()
        });

        let events = vec![
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: None,
                stack_height: Some(1),
                is_dlmm_event_cpi: false,
                event: outer_event,
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(0),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: inner_trade,
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(1),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: inner_fee_only,
            },
        ];

        let result = merge_instruction_events(events);
        assert_eq!(result.len(), 1);
        if let DexEvent::PumpFunTrade(trade) = &result[0] {
            assert_eq!(trade.bonding_curve, bc);
            assert_eq!(trade.sol_amount, 1000);
            assert_eq!(trade.token_amount, 2000);
            assert_eq!(trade.fee_recipient, fee);
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    fn dlmm_swap(pool: Pubkey, amount_in: u64, amount_out: u64) -> DexEvent {
        DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
            metadata: EventMetadata::default(),
            token_x_mint: Pubkey::default(),
            token_y_mint: Pubkey::default(),
            min_amount_out: 0,
            pool,
            from: Pubkey::default(),
            start_bin_id: 0,
            end_bin_id: 0,
            amount_in,
            amount_out,
            swap_for_y: false,
            fee: 0,
            protocol_fee: 0,
            fee_bps: 0,
            host_fee: 0,
        })
    }

    fn dlmm_add_liquidity() -> DexEvent {
        DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
            metadata: EventMetadata::default(),
            pool: Pubkey::default(),
            from: Pubkey::default(),
            position: Pubkey::default(),
            amounts: [0; 2],
            active_bin_id: 0,
        })
    }

    #[test]
    fn merge_preserves_unrelated_inner_events() {
        let events = vec![
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: None,
                stack_height: Some(1),
                is_dlmm_event_cpi: false,
                event: dlmm_swap(Pubkey::default(), 0, 0),
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(0),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: dlmm_add_liquidity(),
            },
        ];

        let result = merge_instruction_events(events);
        assert_eq!(result.len(), 2);
        assert!(matches!(result[0], DexEvent::MeteoraDlmmSwap(_)));
        assert!(matches!(result[1], DexEvent::MeteoraDlmmAddLiquidity(_)));
    }

    #[test]
    fn merge_dlmm_inner_instruction_with_direct_event_cpi() {
        let first_pool = Pubkey::new_unique();
        let second_pool = Pubkey::new_unique();
        let events = vec![
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(0),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: dlmm_swap(first_pool, 1, 0),
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(1),
                stack_height: Some(3),
                is_dlmm_event_cpi: true,
                event: dlmm_swap(first_pool, 10, 9),
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(2),
                stack_height: Some(2),
                is_dlmm_event_cpi: false,
                event: dlmm_swap(second_pool, 2, 0),
            },
            IndexedInstructionEvent {
                outer_idx: 0,
                inner_idx: Some(3),
                stack_height: Some(3),
                is_dlmm_event_cpi: true,
                event: dlmm_swap(second_pool, 20, 18),
            },
        ];

        let result = merge_instruction_events(events);
        assert_eq!(result.len(), 2);
        let DexEvent::MeteoraDlmmSwap(first) = &result[0] else { panic!("swap") };
        let DexEvent::MeteoraDlmmSwap(second) = &result[1] else { panic!("swap") };
        assert_eq!((first.pool, first.amount_in, first.amount_out), (first_pool, 10, 9));
        assert_eq!((second.pool, second.amount_in, second.amount_out), (second_pool, 20, 18));
    }

    #[test]
    fn grpc_pumpswap_inner_create_pool_cpi_reads_cashback_flag() {
        let signature = "v5rg9RMc6D4pMsAqD8TrmXGFwHQBePFDWXBbtsQmP5gttLBKvExSEiPcGMipaDWP61VdWaxEyJCr7oXPxFH4DQf";
        let static_keys = [
            "9C4nRvhhVquCKATjDCx5FKvNS9PNgNqgyWy9AcoDjYv5",
            "CRfzaig7jyogshSi4Lydsg3RXm3Ta9Gg4oMVTV7UcYej",
            "6sFov2ot9waASAUCLf3hUDc9UXSxw36nE1ehbJqA37XS",
            "F4brPQAt8DR6bN7DLhXzyLUJ77NFYUCokxmnS7cmgvki",
            "HJKRc3JtgmattaPBFp1XqAhymk9FtJQjZZWZ9LtCMDLC",
            "4pVPfQmUZPDUgzTC5VAuad82wpaf4yzvSWVvFQBs73sv",
            "2m3hPFQ17Vn2gdeoxCr4M8Tx9jcLtTjiLtqnpyz7Tizo",
            "HC5ix2JxmZQ9sNiPFbFsFuXfu7GHt2RT2UoQVFWskfhu",
            "GywAHNZRk8qjAiekaXgk5mBqweMibW31KGnUHzMN5Ht4",
            "H1e1uYxxkSeJpjKeqajizBTCMXc4wun1vqgNGiFgsXru",
            "56pZVJ6T5Dy3MZ56YcAcHM9YqEbyustsjS9MNNNh16cC",
            "GzZSwyjsKKmMHtEdMggC9fB1bowTm3Vzhs6hxMQfviVu",
            "ComputeBudget111111111111111111111111111111",
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
            "9JmruaWd8Dscxs1GBVbnckGWWsoVdJwg9DDFGFW9pump",
            "SysvarRent111111111111111111111111111111111",
        ];
        let loaded_writable = ["39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg"];
        let loaded_readonly = [
            "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf",
            "So11111111111111111111111111111111111111112",
            "11111111111111111111111111111111",
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
            "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR",
            "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1",
        ];
        let meta = TransactionStatusMeta {
            loaded_writable_addresses: loaded_writable.iter().map(|s| pubkey_bytes(pk(s))).collect(),
            loaded_readonly_addresses: loaded_readonly.iter().map(|s| pubkey_bytes(pk(s))).collect(),
            inner_instructions: vec![InnerInstructions {
                index: 2,
                instructions: vec![InnerInstruction {
                    program_id_index: 20,
                    accounts: vec![4, 21, 5, 14, 18, 8, 6, 7, 9, 10, 11, 19, 22, 22, 23, 24, 25, 20],
                    data: decode_b58(
                        "iPiwDbPRj3YavFpj3AxMZtPvSSQKdH3Uw8kaPUDj2NXDsWjrQx5ndF39nxYypLG2dVKDtBiBz3jsJ6gvzU",
                    ),
                    stack_height: Some(2),
                }],
            }],
            ..Default::default()
        };
        let tx = Some(Transaction {
            signatures: vec![pk("11111111111111111111111111111111").as_ref().to_vec()],
            message: Some(Message {
                header: Some(MessageHeader::default()),
                account_keys: static_keys.iter().map(|s| pubkey_bytes(pk(s))).collect(),
                recent_blockhash: vec![0; 32],
                instructions: vec![
                    CompiledInstruction { program_id_index: 12, accounts: vec![], data: vec![0] },
                    CompiledInstruction { program_id_index: 12, accounts: vec![], data: vec![0] },
                    CompiledInstruction { program_id_index: 13, accounts: vec![], data: vec![0] },
                ],
                versioned: true,
                address_table_lookups: Vec::new(),
            }),
        });

        let events = parse_instructions_enhanced(
            &meta,
            &tx,
            signature.parse().unwrap(),
            427_039_576,
            0,
            Some(1_781_687_252_000_000),
            789,
            None,
        );

        assert_eq!(events.len(), 1, "{signature}");
        match &events[0] {
            DexEvent::PumpSwapCreatePool(e) => {
                assert_eq!(e.index, 0, "{signature}");
                assert_eq!(e.base_amount_in, 206_900_000_000_000, "{signature}");
                assert_eq!(e.quote_amount_in, 84_990_359_912, "{signature}");
                assert_eq!(
                    e.coin_creator,
                    pk("4DrtsW86GarGJJeYrBwYCjoyMgDPG95QWSGhFHvCkU2s"),
                    "{signature}"
                );
                assert!(!e.is_mayhem_mode, "{signature}");
                assert!(e.is_cashback_coin, "{signature}");
                assert_eq!(
                    e.pool,
                    pk("HJKRc3JtgmattaPBFp1XqAhymk9FtJQjZZWZ9LtCMDLC"),
                    "{signature}"
                );
                assert_eq!(
                    e.creator,
                    pk("4pVPfQmUZPDUgzTC5VAuad82wpaf4yzvSWVvFQBs73sv"),
                    "{signature}"
                );
                assert_eq!(
                    e.base_mint,
                    pk("9JmruaWd8Dscxs1GBVbnckGWWsoVdJwg9DDFGFW9pump"),
                    "{signature}"
                );
                assert_eq!(
                    e.quote_mint,
                    pk("So11111111111111111111111111111111111111112"),
                    "{signature}"
                );
                assert_eq!(
                    e.lp_mint,
                    pk("GywAHNZRk8qjAiekaXgk5mBqweMibW31KGnUHzMN5Ht4"),
                    "{signature}"
                );
                assert_eq!(
                    e.user_base_token_account,
                    pk("2m3hPFQ17Vn2gdeoxCr4M8Tx9jcLtTjiLtqnpyz7Tizo"),
                    "{signature}"
                );
                assert_eq!(
                    e.user_quote_token_account,
                    pk("HC5ix2JxmZQ9sNiPFbFsFuXfu7GHt2RT2UoQVFWskfhu"),
                    "{signature}"
                );
            }
            other => panic!("expected PumpSwapCreatePool for {signature}, got {other:?}"),
        }
    }

    #[test]
    fn grpc_pumpfun_create_v2_resolves_alt_loaded_quote_mint_cases() {
        struct Case {
            signature: &'static str,
            name: &'static str,
            static_len: usize,
            writable_len: usize,
            program_idx: u8,
            account_len: usize,
            mint_idx: u8,
            mint: &'static str,
            user_idx: u8,
            user: &'static str,
            token_program_idx: u8,
            quote_idx: u8,
            quote_mint: Pubkey,
            quote_vault_idx: u8,
            quote_vault: &'static str,
            quote_token_program_idx: u8,
        }
        let token_2022_program = crate::accounts::program_ids::SPL_TOKEN_2022_PROGRAM_ID;
        let spl_token_program = crate::accounts::program_ids::SPL_TOKEN_PROGRAM_ID;
        let cases = [
            Case {
                signature: "4GCVgY2FnT1s4q5zemnPL4mzSbuhUTgQo9mc9jewhLZzsCXKe8ehz6xD4QDJE853CLrF6doJbf4JNwJVeEYLA4De",
                name: "19-account WSOL quote in ALT",
                static_len: 15,
                writable_len: 7,
                program_idx: 12,
                account_len: 19,
                mint_idx: 1,
                mint: "CGY36MoFU627gPH4TLM5NP4Xnvhz6Nesc71TQecPpump",
                user_idx: 0,
                user: "Aqje5DsN4u2PHmQxGF9PKfpsDGwQRCBhWeLKHCFhSMXk",
                token_program_idx: 24,
                quote_idx: 27,
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault_idx: 7,
                quote_vault: "CWR85PmUfzNNgmNN9Ref8L8BvMibZ1tzchiT5bTZpJhn",
                quote_token_program_idx: 28,
            },
            Case {
                signature: "5HwZKTwcGFjSBPugSX5hE9JSq5wKmUooK3tLXuEoyDDzrTvHu7op3XDbhBXuteiC5EePNPh8TC1j6Fns47YvnyeG",
                name: "19-account WSOL quote in ALT with exact quote buy",
                static_len: 20,
                writable_len: 7,
                program_idx: 15,
                account_len: 19,
                mint_idx: 1,
                mint: "7NSSfLGsjNHzKxrgggQ56C2UdKxJVJvrECJR3dsbBuuG",
                user_idx: 0,
                user: "2bBRwhGoL4fRZk6g8NnhBZywsF8PdLJnBRfWDCEMogD2",
                token_program_idx: 31,
                quote_idx: 28,
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault_idx: 4,
                quote_vault: "6jFz2oefpJUE6opjA7vxs3iXou7YYyb6e6E4LN2BFs1W",
                quote_token_program_idx: 30,
            },
            Case {
                signature: "3MVawF6EPtG7rEPXdsyQfQUBLv3epRVNpNS4tRE4uwTPMqLNPqhuABwxU3QZH4uD6CuVupcpGchpNRK5HTbHRLNK",
                name: "19-account USDC quote in ALT",
                static_len: 19,
                writable_len: 6,
                program_idx: 16,
                account_len: 19,
                mint_idx: 1,
                mint: "FUsqvH5x8QUrxmJhspt6meQZtfBr17m2YsTFuVsYpump",
                user_idx: 0,
                user: "9Gg6Mf8tq9zLSpK8qccrQiue3iE7wmyeogKkGZpnz2w5",
                token_program_idx: 27,
                quote_idx: 30,
                quote_mint: usdc_mint(),
                quote_vault_idx: 6,
                quote_vault: "7SLtvqMx4bPoWSbPcnWBWpBem3RXbKraWUsiApXjB1VL",
                quote_token_program_idx: 31,
            },
            Case {
                signature: "oY9YQbie16Bw11GsqbAPVnW6YjMHAj3kP9sufjcuQjdfcU86iUY8CiSaDrvu4QXJFnGY4jqQc2Kc1YVuAzujvyv",
                name: "20-account WSOL quote in ALT",
                static_len: 15,
                writable_len: 7,
                program_idx: 12,
                account_len: 20,
                mint_idx: 1,
                mint: "Bv3zjsdJ5KuA9KsGirqssC8pVJwCeCeyLjo4Hqpfpump",
                user_idx: 0,
                user: "2SWqdMbn1FJVUMUEpuyP2St8BPRtqJYXJPWFfmZr486q",
                token_program_idx: 24,
                quote_idx: 27,
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault_idx: 7,
                quote_vault: "9QdMAuwtpnHSzjTQcTkjU1GFSs2gNtR66sdQofFv5P7B",
                quote_token_program_idx: 28,
            },
            Case {
                signature: "3jWGFYXT5V33Qc2roEBFDRAWHeybDowr53dSdnYSRkrPdYybU7oyEH9BfgSRxkgFHVKmUjv4e5T33AEnhJvBCuP2",
                name: "19-account WSOL quote in ALT with later buy",
                static_len: 18,
                writable_len: 7,
                program_idx: 13,
                account_len: 19,
                mint_idx: 1,
                mint: "5i8AZEBc8o5dhfnTQdD3QTVejgbjitwQ1ADHg1jZpump",
                user_idx: 0,
                user: "2b2N2p7xCS9ibDqxwYgXpDSTniJwwye7n93WYuzmr74s",
                token_program_idx: 27,
                quote_idx: 30,
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault_idx: 7,
                quote_vault: "9QB9SyXGDbHUsvvF8XMbYH5ioJMHKHhXTjQDoL56uHT7",
                quote_token_program_idx: 31,
            },
            Case {
                signature: "2dZAucKwr4n5Lqu3BtJ4P8JsjCDtUXJzthadddfURraEJRTgn6XWaTNUNBbgUfP5c2wcVdubqViQhr48eWsgRqPX",
                name: "19-account USDC quote in ALT exact quote buy",
                static_len: 19,
                writable_len: 6,
                program_idx: 15,
                account_len: 19,
                mint_idx: 1,
                mint: "DsE8Ptubc1HWWethf9ant4eV9YnofEv5kfGyLdj7jk2Y",
                user_idx: 0,
                user: "easy7tXgADWkRMNjFRS2XsLXUAaKH5tEPodh9g7kcX8",
                token_program_idx: 28,
                quote_idx: 33,
                quote_mint: usdc_mint(),
                quote_vault_idx: 7,
                quote_vault: "8QTKfEBf5yChuos4eTzQPbV3jXveCu5GkNKLFoS8oS7t",
                quote_token_program_idx: 27,
            },
            Case {
                signature: "4h9kYjzYpqqyYZuFnjf14zRwrGyChCuKAYVy6a4ZBig19bydEYsHwp6VbiKqTzT3pLf6NXnf6E25dn1NiU8LR4YB",
                name: "20-account WSOL quote in ALT with jit account",
                static_len: 15,
                writable_len: 7,
                program_idx: 12,
                account_len: 20,
                mint_idx: 1,
                mint: "6EvDE4a7Yw8F65oy6UhhN3JBshGk9tV3b2yxNyhypump",
                user_idx: 0,
                user: "2SWqdMbn1FJVUMUEpuyP2St8BPRtqJYXJPWFfmZr486q",
                token_program_idx: 24,
                quote_idx: 27,
                quote_mint: PUMPFUN_WSOL_QUOTE_MINT,
                quote_vault_idx: 7,
                quote_vault: "27jyvk4PUYjcDQkKn8VGT9zNdAxZWWjqALpRUpjMqc2y",
                quote_token_program_idx: 28,
            },
        ];

        for case in cases {
            let (meta, tx) = grpc_pumpfun_create_v2_tx(
                case.static_len,
                case.writable_len,
                case.program_idx,
                create_v2_accounts(
                    case.account_len,
                    case.program_idx,
                    case.mint_idx,
                    case.user_idx,
                    case.token_program_idx,
                    Some((case.quote_idx, case.quote_vault_idx, case.quote_token_program_idx)),
                ),
                vec![
                    (case.mint_idx as usize, pk(case.mint)),
                    (case.user_idx as usize, pk(case.user)),
                    (case.token_program_idx as usize, token_2022_program),
                    (case.quote_idx as usize, case.quote_mint),
                    (case.quote_vault_idx as usize, pk(case.quote_vault)),
                    (case.quote_token_program_idx as usize, spl_token_program),
                ],
            );
            let loaded_key_location = |global_idx: u8| -> (&'static str, usize) {
                let idx = global_idx as usize;
                if idx < case.static_len {
                    ("static", idx)
                } else if idx < case.static_len + case.writable_len {
                    ("writable", idx - case.static_len)
                } else {
                    ("readonly", idx - case.static_len - case.writable_len)
                }
            };
            assert_eq!(
                meta.loaded_writable_addresses.len(),
                case.writable_len,
                "{}: {}",
                case.name,
                case.signature
            );
            for (global_idx, expected_key) in [
                (case.token_program_idx, token_2022_program),
                (case.quote_idx, case.quote_mint),
                (case.quote_token_program_idx, spl_token_program),
            ] {
                match loaded_key_location(global_idx) {
                    ("static", _) => {}
                    ("writable", offset) => assert_eq!(
                        read_pubkey_fast(&meta.loaded_writable_addresses[offset]),
                        expected_key,
                        "{}: writable loaded key {global_idx}: {}",
                        case.name,
                        case.signature
                    ),
                    ("readonly", offset) => assert_eq!(
                        read_pubkey_fast(&meta.loaded_readonly_addresses[offset]),
                        expected_key,
                        "{}: readonly loaded key {global_idx}: {}",
                        case.name,
                        case.signature
                    ),
                    _ => unreachable!(),
                }
            }

            let create = parse_create_v2_from_grpc(&meta, &tx);

            assert_eq!(create.mint, pk(case.mint), "{}: {}", case.name, case.signature);
            assert_eq!(create.user, pk(case.user), "{}: {}", case.name, case.signature);
            assert_eq!(
                create.token_program, token_2022_program,
                "{}: {}",
                case.name, case.signature
            );
            assert_eq!(create.quote_mint, case.quote_mint, "{}: {}", case.name, case.signature);
            assert_eq!(
                create.quote_vault,
                pk(case.quote_vault),
                "{}: {}",
                case.name,
                case.signature
            );
            assert_eq!(
                create.quote_token_program, spl_token_program,
                "{}: {}",
                case.name, case.signature
            );
        }
    }

    #[test]
    fn grpc_pumpfun_create_v2_16_account_uses_sol_sentinel_without_quote_tail() {
        let signature = "H6azwLqtRtrnVNC5iwcjYM9idU3e9SRyLZXTwjfJGJxA4X7dZL7vyhFAJNvQy7bb6bmQNmFHUt1KkkPPmhdge3G";
        let mint = pk("HhL4NuFWAfHScNBUksxN6YNXbMNbcSkH4LJaWgZkpump");
        let user = pk("25jZ7EwnKfZo2DZgHM27pbU5Tf54PYG8jc7qNL3gtkxG");
        let token_program = crate::accounts::program_ids::SPL_TOKEN_2022_PROGRAM_ID;
        let (meta, tx) = grpc_pumpfun_create_v2_tx(
            16,
            5,
            12,
            create_v2_accounts(16, 12, 1, 0, 24, None),
            vec![(1, mint), (0, user), (24, token_program)],
        );

        let create = parse_create_v2_from_grpc(&meta, &tx);

        assert_eq!(create.mint, mint, "{signature}");
        assert_eq!(create.user, user, "{signature}");
        assert_eq!(create.token_program, token_program, "{signature}");
        assert_eq!(create.quote_mint, PUMPFUN_SOLSCAN_SOL_QUOTE_MINT, "{signature}");
        assert_eq!(create.quote_vault, Pubkey::default(), "{signature}");
        assert_eq!(create.quote_token_program, Pubkey::default(), "{signature}");
    }

    #[test]
    fn grpc_pumpfun_create_v2_rejects_program_id_as_quote_mint() {
        let quote_vault = Pubkey::new_unique();
        let quote_token_program = crate::accounts::program_ids::SPL_TOKEN_PROGRAM_ID;
        let (meta, tx) = grpc_pumpfun_create_v2_tx(
            19,
            6,
            16,
            create_v2_accounts(19, 16, 1, 0, 27, Some((30, 6, 31))),
            vec![
                (27, crate::accounts::program_ids::SPL_TOKEN_2022_PROGRAM_ID),
                (30, crate::instr::program_ids::PUMPFUN_PROGRAM_ID),
                (6, quote_vault),
                (31, quote_token_program),
            ],
        );

        let create = parse_create_v2_from_grpc(&meta, &tx);

        assert_eq!(create.quote_mint, Pubkey::default());
        assert_eq!(create.quote_vault, Pubkey::default());
        assert_eq!(create.quote_token_program, Pubkey::default());
    }
}
