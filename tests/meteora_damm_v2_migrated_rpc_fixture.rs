//! A swap on the Meteora DAMM v2 pool a DBC curve migrated to, from a saved
//! mainnet transaction: the sale a wallet exits through once the curve is
//! complete. The pool keeps the curve's base token as token A and its quote as
//! token B.

use sol_parser_sdk::core::events::MeteoraDammV2SwapEvent;
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::{parse_rpc_transaction, DexEvent};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

/// A sale of a migrated transfer-hook token for USDC, routed through a
/// trading app's program with a referral account.
const MIGRATED_SELL: &str =
    include_str!("fixtures/meteora_damm_v2_migrated_sell_rpc_transaction.json");

fn pk(key: &str) -> Pubkey {
    key.parse().expect("valid pubkey")
}

fn swaps(filter: Option<&EventTypeFilter>) -> Vec<MeteoraDammV2SwapEvent> {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(MIGRATED_SELL).expect("valid RPC transaction fixture");
    parse_rpc_transaction(&transaction, filter)
        .expect("parse RPC fixture")
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::MeteoraDammV2Swap(swap) => Some(swap),
            _ => None,
        })
        .collect()
}

#[test]
fn migrated_pool_sale_through_a_router_carries_amounts_and_accounts() {
    let mut swaps = swaps(None);
    assert_eq!(swaps.len(), 1);
    let swap = swaps.remove(0);

    assert_eq!(
        swap.metadata.signature.to_string(),
        "QdiNzUmM2qBgMGtDvjoGmHbwXR2FSHG5n5uyVDE6TZrHPs1ZT1bNyyZh6inrh6jjQpSbM4bmTNBSpch5gsjucQc"
    );
    assert_eq!(swap.pool, pk("EPy3Rnwz9G1eg1wx6a9wCoEsnSFCwb3r4keFFzxauLLX"));
    // Token A for token B: a sale of the migrated token.
    assert_eq!(swap.trade_direction, 0);
    assert_eq!(swap.swap_mode, 0);
    assert_eq!(swap.collect_fee_mode, 1);
    assert_eq!(swap.included_transfer_fee_amount_in, 9_629_976_451);
    assert_eq!(swap.excluded_fee_input_amount, 9_629_976_451);
    assert_eq!(swap.included_transfer_fee_amount_out, 223_210_626);
    assert_eq!(swap.excluded_transfer_fee_amount_out, 223_210_626);
    assert_eq!(swap.output_amount, 223_210_626);
    assert_eq!(swap.next_sqrt_price, 2_771_183_070_634_004_317);
    // 2% of the output, split between the pool, the protocol and the referrer.
    assert_eq!(swap.lp_fee + swap.protocol_fee + swap.referral_fee, 4_555_319);
    assert_eq!((swap.reserve_a_amount, swap.reserve_b_amount), (210_140_637_856, 4_742_431_321));

    // The swap's own accounts, from the instruction the router invoked.
    assert_eq!(swap.payer, pk("864JsVttMC3vnLgzUmTjUAXjg3fAsfgAr2WbwPQsDXVi"));
    assert_eq!(swap.token_a_mint, pk("mAo7GAjCZ2LCW5yNoQW31Ce9kLttUMCjyjD2kP9ever"));
    assert_eq!(swap.token_b_mint, pk("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
    assert_eq!(swap.token_a_program, pk("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"));
    assert_eq!(swap.token_b_program, pk("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));
    assert_eq!(swap.token_a_vault, pk("ATXctdLcdc5RYbanSXDZdqCggHEgV8pMaheR4MFPxor2"));
    assert_eq!(swap.token_b_vault, pk("5mnnDvGY7oAQTRF2MyfVpip1SLuAKC7KfCQQrq2JW7hA"));
    assert_eq!(
        swap.referral_token_account,
        Some(pk("4FHgenmfd3YkKpCEkftVdctAGRMBpmP4My7t3tH4NoDi"))
    );
}

#[test]
fn migrated_pool_sale_passes_a_swap_only_filter() {
    let filter = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2Swap]);
    let swaps = swaps(Some(&filter));
    assert_eq!(swaps.len(), 1);
    assert_eq!(swaps[0].payer, pk("864JsVttMC3vnLgzUmTjUAXjg3fAsfgAr2WbwPQsDXVi"));
}
