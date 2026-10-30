//! Yellowstone [`Transaction`] / [`TransactionStatusMeta`] 通用工具。
//!
//! 不依赖 DEX 日志或指令解析，适用于：mentions 订阅后的 SOL/SPL 转账分析、审计、风控等。

use std::collections::HashSet;
use std::sync::Arc;

use crate::{instr::read_pubkey_fast, DexEvent};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use yellowstone_grpc_proto::prelude::{TokenBalance, Transaction, TransactionStatusMeta};

/// Transaction message version represented by Yellowstone's normalized protobuf.
///
/// Yellowstone uses `Message.config` to identify V1. The older `versioned` flag
/// alone cannot distinguish V0 from V1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YellowstoneMessageVersion {
    Legacy,
    V0,
    V1,
}

/// Returns the transaction version encoded by a Yellowstone protobuf message.
///
/// `config` must be checked first because both V0 and V1 set `versioned`.
#[inline]
pub fn yellowstone_message_version(
    message: &yellowstone_grpc_proto::prelude::Message,
) -> YellowstoneMessageVersion {
    if message.config.is_some() {
        YellowstoneMessageVersion::V1
    } else if message.versioned {
        YellowstoneMessageVersion::V0
    } else {
        YellowstoneMessageVersion::Legacy
    }
}

/// Fill creation transaction context and the recent blockhash once final events are known.
///
/// Encoding after log/instruction deduplication avoids doing the same base58
/// allocation independently on both parser branches.
#[inline]
pub(crate) fn fill_transaction_context(events: &mut [DexEvent], transaction: &Option<Transaction>) {
    let Some(message) = transaction.as_ref().and_then(|tx| tx.message.as_ref()) else {
        return;
    };

    let version = yellowstone_message_version(message);
    for event in events.iter_mut() {
        let alt_addresses = match event {
            DexEvent::PumpFunCreate(create) => {
                create.transaction_version = Some(version);
                &mut create.alt_addresses
            }
            DexEvent::PumpFunCreateV2(create) => {
                create.transaction_version = Some(version);
                &mut create.alt_addresses
            }
            _ => continue,
        };
        *alt_addresses = creation_alt_addresses(message);
    }
    let blockhash = &message.recent_blockhash;
    if blockhash.is_empty() {
        return;
    }
    let mut metadata = events.iter_mut().filter_map(DexEvent::metadata_mut);
    let Some(first) = metadata.next() else { return };
    let encoded = bs58::encode(blockhash).into_string();
    for item in metadata {
        item.recent_blockhash = Some(encoded.clone());
    }
    first.recent_blockhash = Some(encoded);
}

fn creation_alt_addresses(
    message: &yellowstone_grpc_proto::prelude::Message,
) -> Option<Vec<Pubkey>> {
    let lookups = &message.address_table_lookups;
    if lookups.is_empty() {
        return None;
    }
    let mut addresses = Vec::with_capacity(lookups.len());
    for lookup in lookups {
        // A malformed key invalidates the list instead of exposing a partial result.
        addresses.push(Pubkey::try_from(lookup.account_key.as_slice()).ok()?);
    }
    Some(addresses)
}

/// 32 字节公钥 → base58 地址字符串。
#[inline]
pub fn pubkey_bytes_to_bs58(bytes: &[u8]) -> Option<String> {
    let a: [u8; 32] = bytes.try_into().ok()?;
    Some(solana_sdk::pubkey::Pubkey::from(a).to_string())
}

/// 消息静态 `account_keys` + meta 中 `loaded_writable_addresses` / `loaded_readonly_addresses`，
/// 顺序与 `pre_balances` / `post_balances` 对齐。
pub fn collect_account_keys_bs58(
    tx: &Transaction,
    meta: &TransactionStatusMeta,
) -> Option<Vec<String>> {
    let msg = tx.message.as_ref()?;
    let mut keys: Vec<String> =
        msg.account_keys.iter().filter_map(|b| pubkey_bytes_to_bs58(b.as_slice())).collect();
    for b in &meta.loaded_writable_addresses {
        keys.push(pubkey_bytes_to_bs58(b)?);
    }
    for b in &meta.loaded_readonly_addresses {
        keys.push(pubkey_bytes_to_bs58(b)?);
    }
    Some(keys)
}

/// 每个账户索引的 lamports 变化（post - pre）。
#[inline]
pub fn lamport_balance_deltas(meta: &TransactionStatusMeta) -> Vec<i128> {
    meta.pre_balances
        .iter()
        .zip(meta.post_balances.iter())
        .map(|(pre, post)| *post as i128 - *pre as i128)
        .collect()
}

/// 启发式原生 SOL：对 `watched_bs58` 中出现的账户，若 lamports 净减少 ≥ `min_outflow_lamports`，
/// 再与其它索引配对，要求对方 delta ≥ `min_outflow_lamports/2`（与常见 mentions 转账监控一致）。
pub fn heuristic_sol_counterparties_for_watched_keys(
    account_keys_bs58: &[String],
    lamport_deltas: &[i128],
    watched_bs58: &HashSet<&str>,
    min_outflow_lamports: u64,
) -> Vec<(String, String)> {
    let min_l = min_outflow_lamports as i128;
    let mut pairs = Vec::new();
    for (i, key) in account_keys_bs58.iter().enumerate() {
        if !watched_bs58.contains(key.as_str()) {
            continue;
        }
        let d = lamport_deltas.get(i).copied().unwrap_or(0);
        if d >= -min_l {
            continue;
        }
        for (j, dj) in lamport_deltas.iter().enumerate() {
            if i == j || *dj <= min_l / 2 {
                continue;
            }
            pairs.push((key.clone(), account_keys_bs58[j].clone()));
        }
    }
    pairs
}

/// 汇总「监控地址」在一笔交易中的转出对手方（原生 SOL 启发式 + SPL token balance 启发式）。
///
/// 返回 `None` 当账户 key 与 balance 数组长度不一致。
pub fn collect_watch_transfer_counterparty_pairs(
    tx: &Transaction,
    meta: &TransactionStatusMeta,
    watched_bs58: &[String],
    min_native_outflow_lamports: u64,
    spl_min_watch_decrease_raw: u64,
) -> Option<Vec<(String, String)>> {
    let keys = collect_account_keys_bs58(tx, meta)?;
    let n = keys.len();
    if meta.pre_balances.len() != n || meta.post_balances.len() != n {
        return None;
    }
    let deltas = lamport_balance_deltas(meta);
    let watched_h: HashSet<&str> = watched_bs58.iter().map(|s| s.as_str()).collect();

    let mut pairs = heuristic_sol_counterparties_for_watched_keys(
        &keys,
        &deltas,
        &watched_h,
        min_native_outflow_lamports,
    );
    for w in watched_bs58 {
        pairs.extend(spl_token_counterparty_by_owner(meta, w, spl_min_watch_decrease_raw));
    }
    pairs.sort_by(|a, b| a.1.cmp(&b.1));
    pairs.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    Some(pairs)
}

/// `TokenBalance.ui_token_amount.amount` 解析为原始整数；失败为 0。
#[inline]
pub fn token_balance_raw_amount(t: &TokenBalance) -> u64 {
    try_token_balance_raw_amount(t).unwrap_or(0)
}

/// Parse `TokenBalance.ui_token_amount.amount` without treating malformed metadata as zero.
#[inline]
pub fn try_token_balance_raw_amount(t: &TokenBalance) -> Option<u64> {
    t.ui_token_amount.as_ref()?.amount.parse().ok()
}

/// SPL：对给定 owner（TokenBalance.owner，base58），当其某 mint 上余额净减少 ≥ `min_watch_decrease_raw` 时，
/// 找出同 mint 下余额增加的其它 owner，返回 `(watch_owner, counterparty_owner)`。
///
/// 用于启发式「谁转给谁」配对（非链上 Transfer 事件级精确解析）。
pub fn spl_token_counterparty_by_owner(
    meta: &TransactionStatusMeta,
    watch_owner_bs58: &str,
    min_watch_decrease_raw: u64,
) -> Vec<(String, String)> {
    use std::collections::{HashMap, HashSet};

    let pre = meta.pre_token_balances.as_slice();
    let post = meta.post_token_balances.as_slice();

    let mut pre_m: HashMap<(String, String), u64> = HashMap::new();
    for b in pre {
        if b.owner.is_empty() {
            continue;
        }
        let k = (b.mint.clone(), b.owner.clone());
        *pre_m.entry(k).or_insert(0) += token_balance_raw_amount(b);
    }
    let mut post_m: HashMap<(String, String), u64> = HashMap::new();
    for b in post {
        if b.owner.is_empty() {
            continue;
        }
        let k = (b.mint.clone(), b.owner.clone());
        *post_m.entry(k).or_insert(0) += token_balance_raw_amount(b);
    }

    let mut mints = HashSet::new();
    for (m, o) in pre_m.keys() {
        if o == watch_owner_bs58 {
            mints.insert(m.clone());
        }
    }
    for (m, o) in post_m.keys() {
        if o == watch_owner_bs58 {
            mints.insert(m.clone());
        }
    }

    let mut out = Vec::new();
    let min_l = min_watch_decrease_raw;
    for mint in mints {
        let w_pre = pre_m.get(&(mint.clone(), watch_owner_bs58.to_string())).copied().unwrap_or(0);
        let w_post =
            post_m.get(&(mint.clone(), watch_owner_bs58.to_string())).copied().unwrap_or(0);
        let lost = w_pre.saturating_sub(w_post);
        if lost < min_l.max(1) {
            continue;
        }
        for ((m, owner), po) in &post_m {
            if m != &mint || owner == watch_owner_bs58 {
                continue;
            }
            let pr = pre_m.get(&(mint.clone(), owner.clone())).copied().unwrap_or(0);
            if *po > pr {
                out.push((watch_owner_bs58.to_string(), owner.clone()));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    out
}

/// 仅消息头里的静态 `account_keys`（与 ShredStream `VersionedTransaction::static_account_keys()` 语义对齐；
/// 不含 ALT 加载地址）。
#[inline]
pub fn yellowstone_static_account_keys_arc(tx: &Option<Transaction>) -> Arc<[Pubkey]> {
    let Some(t) = tx.as_ref() else {
        return Arc::from(Vec::<Pubkey>::new().into_boxed_slice());
    };
    let Some(msg) = t.message.as_ref() else {
        return Arc::from(Vec::<Pubkey>::new().into_boxed_slice());
    };
    let keys: Vec<Pubkey> =
        msg.account_keys.iter().map(|bytes| read_pubkey_fast(bytes.as_slice())).collect();
    Arc::from(keys.into_boxed_slice())
}

/// Yellowstone 交易签名原始字节（64）→ `solana_sdk::signature::Signature`。
#[inline]
pub fn try_yellowstone_signature(sig: &[u8]) -> Option<Signature> {
    if sig.len() != 64 {
        return None;
    }
    let a: [u8; 64] = sig.try_into().ok()?;
    Some(Signature::from(a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use yellowstone_grpc_proto::prelude::{Message, TransactionConfig};

    #[test]
    fn creation_alt_addresses_keep_order_and_reject_partial_provider_lists() {
        let first = Pubkey::new_unique();
        let second = Pubkey::new_unique();
        for (keys, expected) in [
            (vec![], None),
            (
                vec![
                    second.to_bytes().to_vec(),
                    first.to_bytes().to_vec(),
                    second.to_bytes().to_vec(),
                ],
                Some(vec![second, first, second]),
            ),
            (vec![first.to_bytes().to_vec(), vec![0; 31]], None),
        ] {
            let tx = Some(Transaction {
                message: Some(Message {
                    address_table_lookups: keys
                        .into_iter()
                        .map(|account_key| {
                            yellowstone_grpc_proto::prelude::MessageAddressTableLookup {
                                account_key,
                                ..Default::default()
                            }
                        })
                        .collect(),
                    ..Default::default()
                }),
                ..Default::default()
            });
            let mut events = [
                DexEvent::PumpFunCreate(Default::default()),
                DexEvent::PumpFunCreateV2(Default::default()),
            ];
            fill_transaction_context(&mut events, &tx);
            for event in events {
                let actual = match event {
                    DexEvent::PumpFunCreate(c) => c.alt_addresses,
                    DexEvent::PumpFunCreateV2(c) => c.alt_addresses,
                    _ => unreachable!(),
                };
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn message_version_uses_config_before_versioned_flag() {
        let legacy = Message::default();
        assert_eq!(yellowstone_message_version(&legacy), YellowstoneMessageVersion::Legacy);

        let v0 = Message { versioned: true, ..Message::default() };
        assert_eq!(yellowstone_message_version(&v0), YellowstoneMessageVersion::V0);

        let v1 = Message {
            versioned: true,
            config: Some(TransactionConfig::default()),
            ..Message::default()
        };
        assert_eq!(yellowstone_message_version(&v1), YellowstoneMessageVersion::V1);

        let v1_with_inconsistent_legacy_flag = Message {
            versioned: false,
            config: Some(TransactionConfig::default()),
            ..Message::default()
        };
        assert_eq!(
            yellowstone_message_version(&v1_with_inconsistent_legacy_flag),
            YellowstoneMessageVersion::V1
        );

        // Creation metadata also works when the provider omits recent_blockhash.
        for (message, version, json_version) in [
            (legacy, YellowstoneMessageVersion::Legacy, "legacy"),
            (v0, YellowstoneMessageVersion::V0, "v0"),
            (v1, YellowstoneMessageVersion::V1, "v1"),
            (v1_with_inconsistent_legacy_flag, YellowstoneMessageVersion::V1, "v1"),
        ] {
            let mut events = [
                DexEvent::PumpFunCreate(Default::default()),
                DexEvent::PumpFunCreateV2(Default::default()),
            ];
            fill_transaction_context(&mut events, &None);
            fill_transaction_context(&mut events, &Some(Transaction::default()));
            for event in &events {
                let json = serde_json::to_value(event).unwrap();
                assert!(json.as_object().unwrap().values().next().unwrap()["transaction_version"]
                    .is_null());
            }
            fill_transaction_context(
                &mut events,
                &Some(Transaction { message: Some(message), ..Default::default() }),
            );
            for event in &events {
                let json = serde_json::to_value(event).unwrap();
                assert_eq!(
                    json.as_object().unwrap().values().next().unwrap()["transaction_version"],
                    json_version
                );
                assert!(event.metadata().recent_blockhash.is_none());
            }
            assert_eq!(
                serde_json::from_str::<YellowstoneMessageVersion>(&format!("\"{json_version}\""))
                    .unwrap(),
                version
            );
        }
    }
}
