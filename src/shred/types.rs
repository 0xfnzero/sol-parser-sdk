use solana_sdk::transaction::VersionedTransaction;

/// 携带槽位信息的交易
#[derive(Debug, Clone, Default)]
pub struct TransactionWithSlot {
    pub transaction: VersionedTransaction,
    pub slot: u64,
    pub recv_us: i64,
}

impl TransactionWithSlot {
    /// 创建新的带槽位的交易
    pub fn new(
        transaction: VersionedTransaction,
        slot: u64,
        recv_us: i64,
    ) -> Self {
        Self { transaction, slot, recv_us }
    }
}
