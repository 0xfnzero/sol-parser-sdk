//! Yellowstone [`SubscribeRequest`] 构造（DEX 订阅、钱包 mentions 转账监控等共用）。

use std::collections::HashMap;

use yellowstone_grpc_proto::prelude::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterBlocksMeta, SubscribeRequestFilterTransactions,
};

use super::types::{AccountFilter, EventTypeFilter, TransactionFilter};

#[inline]
fn tx_filter_to_proto(f: &TransactionFilter) -> SubscribeRequestFilterTransactions {
    SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: Some(false),
        signature: None,
        account_include: f.account_include.clone(),
        account_exclude: f.account_exclude.clone(),
        account_required: f.account_required.clone(),
    }
}

#[inline]
fn acc_filter_to_proto(f: &AccountFilter) -> SubscribeRequestFilterAccounts {
    SubscribeRequestFilterAccounts {
        account: f.account.clone(),
        owner: f.owner.clone(),
        filters: f.filters.clone(),
        nonempty_txn_signature: None,
        cuckoo_accounts_filter: None,
    }
}

fn finalize(
    transactions: HashMap<String, SubscribeRequestFilterTransactions>,
    accounts: HashMap<String, SubscribeRequestFilterAccounts>,
    blocks_meta: HashMap<String, SubscribeRequestFilterBlocksMeta>,
    commitment: CommitmentLevel,
) -> SubscribeRequest {
    SubscribeRequest {
        slots: HashMap::new(),
        accounts,
        transactions,
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta,
        entry: HashMap::new(),
        commitment: Some(commitment as i32),
        accounts_data_slice: Vec::new(),
        ping: None,
        from_slot: None,
    }
}

/// 构建订阅请求：`tx_0`…`tx_n`、`acc_0`…、**commitment = Processed**（与历史行为一致）。
pub fn build_subscribe_request(
    tx_filters: &[TransactionFilter],
    acc_filters: &[AccountFilter],
) -> SubscribeRequest {
    build_subscribe_request_with_commitment(tx_filters, acc_filters, CommitmentLevel::Processed)
}

/// 与 [`build_subscribe_request`] 相同，可指定 commitment（例如 Confirmed）。
pub fn build_subscribe_request_with_commitment(
    tx_filters: &[TransactionFilter],
    acc_filters: &[AccountFilter],
    commitment: CommitmentLevel,
) -> SubscribeRequest {
    build_subscribe_request_with_event_filter(tx_filters, acc_filters, None, commitment)
}

/// 与 [`build_subscribe_request_with_commitment`] 相同，但会按事件过滤器订阅区块元数据。
pub fn build_subscribe_request_with_event_filter(
    tx_filters: &[TransactionFilter],
    acc_filters: &[AccountFilter],
    event_type_filter: Option<&EventTypeFilter>,
    commitment: CommitmentLevel,
) -> SubscribeRequest {
    let transactions = tx_filters
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("tx_{}", i), tx_filter_to_proto(f)))
        .collect();
    let accounts = acc_filters
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("acc_{}", i), acc_filter_to_proto(f)))
        .collect();
    let blocks_meta = if event_type_filter.map(|f| f.includes_block_meta()).unwrap_or(false) {
        HashMap::from([("block_meta".to_string(), SubscribeRequestFilterBlocksMeta {})])
    } else {
        HashMap::new()
    };
    finalize(transactions, accounts, blocks_meta, commitment)
}

/// 自定义交易订阅在 `SubscribeRequest.transactions` 中的 key（便于日志区分多条订阅）。
pub fn build_subscribe_transaction_filters_named<N: AsRef<str>>(
    named_tx_filters: &[(N, TransactionFilter)],
    acc_filters: &[AccountFilter],
    commitment: CommitmentLevel,
) -> SubscribeRequest {
    let transactions = named_tx_filters
        .iter()
        .map(|(name, f)| (name.as_ref().to_string(), tx_filter_to_proto(f)))
        .collect();
    let accounts = acc_filters
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("acc_{}", i), acc_filter_to_proto(f)))
        .collect();
    finalize(transactions, accounts, HashMap::new(), commitment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::types::EventType;

    #[test]
    fn event_filter_controls_block_meta_subscription() {
        let without_block_meta = build_subscribe_request_with_event_filter(
            &[],
            &[],
            Some(&EventTypeFilter::include_only(vec![EventType::PumpFunTrade])),
            CommitmentLevel::Processed,
        );
        assert!(without_block_meta.blocks_meta.is_empty());

        let with_block_meta = build_subscribe_request_with_event_filter(
            &[],
            &[],
            Some(&EventTypeFilter::include_only(vec![EventType::BlockMeta])),
            CommitmentLevel::Processed,
        );
        assert_eq!(with_block_meta.blocks_meta.len(), 1);
    }
}
