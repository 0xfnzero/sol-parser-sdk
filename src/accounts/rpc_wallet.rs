//! 通过 RPC 拉取账户并结合 [`super::utils::user_wallet_pubkey_for_onchain_account`] 做通用分类。

use std::str::FromStr;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use super::utils::user_wallet_pubkey_for_onchain_account;

/// `address_bs58` → `get_account` → 解析为系统钱包或 SPL token 账户背后的 owner。
#[inline]
pub async fn rpc_resolve_user_wallet_pubkey(rpc: &RpcClient, address_bs58: &str) -> Option<Pubkey> {
    let pk = Pubkey::from_str(address_bs58).ok()?;
    let acc = rpc.get_account(&pk).await.ok()?;
    user_wallet_pubkey_for_onchain_account(&pk, &acc.owner, &acc.data, acc.executable)
}
