//! 零拷贝解析器 - 极致性能优化

use crate::core::events::*;
use base64::{engine::general_purpose, Engine as _};
use memchr::memmem;
use solana_sdk::signature::Signature;

/// 零分配 PumpFun Trade 事件解析（栈缓冲区）
#[inline(always)]
pub fn parse_pumpfun_trade(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    is_created_buy: bool,
) -> Option<DexEvent> {
    // 使用栈缓冲区，避免堆分配。当前 TradeEvent tail 含 shareholders Vec，
    // 需要比旧固定布局更大的缓冲区。
    const MAX_DECODE_SIZE: usize = 4096;
    let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

    // SIMD 快速查找 "Program data: "
    let log_bytes = log.as_bytes();
    let pos = memmem::find(log_bytes, b"Program data: ")?;
    let data_part = log[pos + 14..].trim();

    // 快速检查 discriminator（需要至少12个base64字符才能解码出8字节）
    // base64: 每4个字符 = 3个字节，所以12个字符 = 9个字节
    if data_part.len() < 12 {
        return None;
    }

    // 解码 discriminator 到栈缓冲区（12个字符解码为9字节，包含完整8字节discriminator）
    let disc_decoded_len = general_purpose::STANDARD
        .decode_slice(&data_part.as_bytes()[..12], &mut decode_buf[..9])
        .ok()?;

    if disc_decoded_len < 8 {
        return None;
    }

    // 检查是否为 Trade 事件 discriminator
    const TRADE_DISCRIMINATOR: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];

    if decode_buf[..8] != TRADE_DISCRIMINATOR {
        return None;
    }

    // 完整解码事件数据到栈缓冲区
    let decoded_len =
        general_purpose::STANDARD.decode_slice(data_part.as_bytes(), &mut decode_buf).ok()?;

    if decoded_len < 8 {
        return None;
    }

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: None,
    };

    crate::logs::pump::parse_trade_from_data(&decode_buf[8..decoded_len], metadata, is_created_buy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn push_u64(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(out: &mut Vec<u8>, value: i64) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn push_pubkey(out: &mut Vec<u8>, value: Pubkey) {
        out.extend_from_slice(value.as_ref());
    }

    fn trade_log_with_latest_tail(quote_mint: Pubkey, shareholder: Pubkey) -> String {
        let mut data = Vec::new();
        data.extend_from_slice(&[189, 219, 127, 211, 78, 230, 97, 238]);
        push_pubkey(&mut data, Pubkey::new_unique()); // mint
        push_u64(&mut data, 1_000); // sol_amount
        push_u64(&mut data, 2_000); // token_amount
        data.push(1); // is_buy
        push_pubkey(&mut data, Pubkey::new_unique()); // user
        push_i64(&mut data, 123); // timestamp
        push_u64(&mut data, 10); // virtual_sol_reserves
        push_u64(&mut data, 20); // virtual_token_reserves
        push_u64(&mut data, 30); // real_sol_reserves
        push_u64(&mut data, 40); // real_token_reserves
        push_pubkey(&mut data, Pubkey::new_unique()); // fee_recipient
        push_u64(&mut data, 50); // fee_basis_points
        push_u64(&mut data, 60); // fee
        push_pubkey(&mut data, Pubkey::new_unique()); // creator
        push_u64(&mut data, 70); // creator_fee_basis_points
        push_u64(&mut data, 80); // creator_fee
        data.push(1); // track_volume
        push_u64(&mut data, 90); // total_unclaimed_tokens
        push_u64(&mut data, 100); // total_claimed_tokens
        push_u64(&mut data, 110); // current_sol_volume
        push_i64(&mut data, 120); // last_update_timestamp
        data.extend_from_slice(&(6u32).to_le_bytes());
        data.extend_from_slice(b"buy_v2");
        data.push(1); // mayhem_mode
        push_u64(&mut data, 130); // cashback_fee_basis_points
        push_u64(&mut data, 140); // cashback
        push_u64(&mut data, 150); // buyback_fee_basis_points
        push_u64(&mut data, 160); // buyback_fee
        data.extend_from_slice(&(1u32).to_le_bytes()); // shareholders len
        push_pubkey(&mut data, shareholder);
        data.extend_from_slice(&(250u16).to_le_bytes());
        push_pubkey(&mut data, quote_mint);
        push_u64(&mut data, 170); // quote_amount
        push_u64(&mut data, 180); // virtual_quote_reserves
        push_u64(&mut data, 190); // real_quote_reserves

        format!("Program data: {}", general_purpose::STANDARD.encode(data))
    }

    #[test]
    fn public_zero_copy_trade_parser_keeps_latest_tail_fields() {
        let quote_mint = Pubkey::new_unique();
        let shareholder = Pubkey::new_unique();
        let log = trade_log_with_latest_tail(quote_mint, shareholder);
        let event = parse_pumpfun_trade(&log, Signature::default(), 1, 0, Some(2), 3, true)
            .expect("trade log");

        match event {
            DexEvent::PumpFunBuy(t) => {
                assert_eq!(t.sol_amount, 1_000);
                assert_eq!(t.token_amount, 2_000);
                assert_eq!(t.ix_name, "buy_v2");
                assert_eq!(t.buyback_fee_basis_points, 150);
                assert_eq!(t.buyback_fee, 160);
                assert_eq!(t.shareholders.len(), 1);
                assert_eq!(t.shareholders[0].address, shareholder);
                assert_eq!(t.shareholders[0].share_bps, 250);
                assert_eq!(t.quote_mint, quote_mint);
                assert_eq!(t.quote_amount, 170);
                assert_eq!(t.virtual_quote_reserves, 180);
                assert_eq!(t.real_quote_reserves, 190);
                assert!(t.is_created_buy);
            }
            other => panic!("expected buy event, got {other:?}"),
        }
    }
}
