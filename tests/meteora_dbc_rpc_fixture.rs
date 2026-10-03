//! Meteora DBC swaps from saved mainnet transactions. The program emits its
//! events as event-CPI inner instructions; the swap's accounts come from the
//! swap instruction of the same pool.

use sol_parser_sdk::core::events::{
    MeteoraDbcCurveCompleteEvent, MeteoraDbcHookAccount, MeteoraDbcSwapEvent,
};
use sol_parser_sdk::grpc::{EventType, EventTypeFilter};
use sol_parser_sdk::{parse_rpc_transaction, DexEvent};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

/// A buy on a transfer-hook pool quoted in USDC, routed through a trading
/// app's program (`swap2_with_transfer_hook`, partial fill, v1 transaction).
const TRANSFER_HOOK_BUY: &str =
    include_str!("fixtures/meteora_dbc_transfer_hook_buy_rpc_transaction.json");
/// The partial fill that completed that pool's curve.
const TRANSFER_HOOK_CURVE_COMPLETE: &str =
    include_str!("fixtures/meteora_dbc_transfer_hook_curve_complete_rpc_transaction.json");
/// A sale that passes the instructions sysvar ahead of five hook accounts, in
/// a v0 transaction with lookup tables.
const TRANSFER_HOOK_SYSVAR_SELL: &str =
    include_str!("fixtures/meteora_dbc_transfer_hook_sysvar_sell_rpc_transaction.json");
/// A buy whose hook takes a writable account.
const TRANSFER_HOOK_WRITABLE_BUY: &str =
    include_str!("fixtures/meteora_dbc_transfer_hook_writable_buy_rpc_transaction.json");
/// A legacy `swap`, which emits `EvtSwap` and `EvtSwap2` for the one trade.
const SWAP_LEGACY_BUY: &str =
    include_str!("fixtures/meteora_dbc_swap_legacy_buy_rpc_transaction.json");
const SWAP2_SELL: &str = include_str!("fixtures/meteora_dbc_swap2_sell_rpc_transaction.json");
/// A `swap2` sent as a top-level instruction.
const SWAP2_TOP_LEVEL_BUY: &str =
    include_str!("fixtures/meteora_dbc_swap2_top_level_buy_rpc_transaction.json");

const DBC_PROGRAM: &str = "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN";
const POOL_AUTHORITY: &str = "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM";
const WSOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const INSTRUCTIONS_SYSVAR: &str = "Sysvar1nstructions1111111111111111111111111";

fn pk(key: &str) -> Pubkey {
    key.parse().expect("valid pubkey")
}

fn parse(fixture: &str, filter: Option<&EventTypeFilter>) -> Vec<DexEvent> {
    let transaction: EncodedConfirmedTransactionWithStatusMeta =
        serde_json::from_str(fixture).expect("valid RPC transaction fixture");
    parse_rpc_transaction(&transaction, filter).expect("parse RPC fixture")
}

fn swaps(fixture: &str) -> Vec<MeteoraDbcSwapEvent> {
    parse(fixture, None)
        .into_iter()
        .filter_map(|event| match event {
            DexEvent::MeteoraDbcSwap(swap) => Some(swap),
            _ => None,
        })
        .collect()
}

fn only_swap(fixture: &str) -> MeteoraDbcSwapEvent {
    let mut swaps = swaps(fixture);
    assert_eq!(swaps.len(), 1, "one DBC swap per fixture");
    swaps.remove(0)
}

fn hook(key: &str, is_writable: bool) -> MeteoraDbcHookAccount {
    MeteoraDbcHookAccount { pubkey: pk(key), is_writable }
}

#[test]
fn transfer_hook_buy_through_a_router_carries_amounts_and_accounts() {
    let swap = only_swap(TRANSFER_HOOK_BUY);

    assert_eq!(
        swap.metadata.signature.to_string(),
        "ysZEH25dfiiZMm94fJ1yrkSqpNFeMQsstY5ueBkxCmj8q1Q2cdneGc5iAEwx7Hhft2ZcXabj5CTV3VjGzKpxjCV"
    );
    assert_eq!(swap.metadata.slot, 453_031_142);
    assert_eq!(swap.current_timestamp, 1_791_056_640);

    assert!(swap.is_buy());
    assert!(swap.transfer_hook);
    assert_eq!(swap.swap_mode, 1);
    assert_eq!((swap.amount_0, swap.amount_1), (190_746_849, 0));
    assert_eq!(swap.amount_in, 190_746_849);
    assert_eq!(swap.actual_input_amount, 186_931_912);
    assert_eq!(swap.amount_left, 0);
    assert_eq!(swap.minimum_amount_out, 0);
    assert_eq!(swap.output_amount, 128_399_686_024);
    assert_eq!(swap.next_sqrt_price, 0x09eb_9d3b_f0a7_1eb7);
    assert_eq!(
        (swap.trading_fee, swap.protocol_fee, swap.referral_fee),
        (3_051_950, 610_390, 152_597)
    );
    assert_eq!(swap.quote_reserve_amount, 1_127_140_226);
    assert_eq!(swap.migration_threshold, 10_000_000_000);
    assert!(!swap.completed_curve());

    assert_eq!(swap.pool, pk("2Rz8zRLAqMtXKBGsxb8DwYN1Ed13TDwLtxNUrEUHtBJY"));
    assert_eq!(swap.config, pk("CchPHVPXdshYVhUd3ExZDd9vJQgW3NK5jhesoeewLZp8"));
    assert_eq!(swap.pool_authority, pk(POOL_AUTHORITY));
    assert_eq!(swap.payer, pk("864JsVttMC3vnLgzUmTjUAXjg3fAsfgAr2WbwPQsDXVi"));
    assert_eq!(swap.base_mint, pk("mAo7GAjCZ2LCW5yNoQW31Ce9kLttUMCjyjD2kP9ever"));
    assert_eq!(swap.quote_mint, pk(USDC));
    assert_eq!(swap.token_base_program, pk(TOKEN_2022_PROGRAM));
    assert_eq!(swap.token_quote_program, pk(TOKEN_PROGRAM));
    assert_eq!(swap.base_vault, pk("3CapsPu1TXoao9PASYQ25geQL2Va2X5eDb2Hxs41bWad"));
    assert_eq!(swap.quote_vault, pk("7EJZt8h4wfSoqDV4A2vtS4VSHkEq3Zcpv8X3B99KrWbQ"));
    assert_eq!(swap.input_token_account, pk("6nJWanMgmkAfxLQHsHGNGBUZinGfBjhGYrajA2yZYr16"));
    assert_eq!(swap.output_token_account, pk("H4km9dkPcc2Bag9MNHMczZv2Y1XM1wqhncPFnRR2wHTn"));
    assert!(swap.has_referral);
    assert_eq!(
        swap.referral_token_account,
        Some(pk("3kgGakJjywT1asVvg4XkDL1cJnCt7DgCRU6RWa9AmZaR"))
    );
    assert_eq!(swap.program, pk(DBC_PROGRAM));

    // The hook program and its validation account; this hook takes no more.
    assert!(!swap.has_instructions_sysvar);
    assert_eq!(
        swap.transfer_hook_accounts,
        vec![
            hook("887b3SjuJv9t9wP6fd7Fe7dqFhksC8c39a2PJRa3cGec", false),
            hook("FGZZEin9TMPnyNMPvdXtgNRPD41f8gV6sByTnDVRXiUR", false),
        ]
    );
}

#[test]
fn partial_fill_that_completes_the_curve_reports_both() {
    let events = parse(TRANSFER_HOOK_CURVE_COMPLETE, None);
    let swap = events
        .iter()
        .find_map(|event| match event {
            DexEvent::MeteoraDbcSwap(swap) => Some(swap),
            _ => None,
        })
        .expect("DBC swap");
    let complete: &MeteoraDbcCurveCompleteEvent = events
        .iter()
        .find_map(|event| match event {
            DexEvent::MeteoraDbcCurveComplete(complete) => Some(complete),
            _ => None,
        })
        .expect("DBC curve completion");

    // The curve took part of the input and gave the rest back.
    assert!(swap.is_buy());
    assert_eq!(swap.swap_mode, 1);
    assert_eq!(swap.amount_in, 53_981_573);
    assert_eq!(swap.actual_input_amount, 52_901_941);
    assert_eq!(swap.amount_left, 62_365_080);
    assert!(swap.amount_0 > swap.amount_in);
    assert_eq!(swap.output_amount, 5_898_797_192);
    assert_eq!(swap.quote_reserve_amount, 10_000_000_005);
    assert!(swap.completed_curve());

    assert_eq!(complete.pool, swap.pool);
    assert_eq!(complete.config, swap.config);
    assert_eq!(complete.base_reserve, 412_698_413_558);
    assert_eq!(complete.quote_reserve, 10_000_000_005);
}

#[test]
fn sysvar_ahead_of_the_hook_accounts_is_not_one_of_them() {
    let swap = only_swap(TRANSFER_HOOK_SYSVAR_SELL);

    assert!(!swap.is_buy());
    assert!(swap.transfer_hook);
    assert_eq!(swap.swap_mode, 0);
    assert_eq!(swap.payer, pk("DNNomypAoY1vfmAufkz6zBRcWFonhM5KDBwXGd42HomR"));
    assert_eq!(swap.pool, pk("2ucmGrX8fU6cGqQXr3ojRoHo6VPWV2Z1TZu89eBjuPT6"));
    assert_eq!(swap.quote_mint, pk(USDC));
    // A sale pays its fee from the output: the whole input reaches the curve.
    assert_eq!(swap.amount_in, 2_819_747_531_971);
    assert_eq!(swap.actual_input_amount, swap.amount_in);
    assert_eq!(swap.output_amount, 90_501_498);
    assert_eq!(swap.minimum_amount_out, 1);
    assert_eq!(swap.referral_token_account, None);
    assert!(!swap.has_referral);

    // Six remaining accounts: the rate limiter's sysvar, then the slice of
    // five hook accounts, whose own first account is the sysvar again.
    assert!(swap.has_instructions_sysvar);
    assert_eq!(
        swap.transfer_hook_accounts,
        vec![
            hook(INSTRUCTIONS_SYSVAR, false),
            hook("ATmQeX8Xan7H1U5uoyTBdjeNXkNLVqHWr8h1qD5jgFj", false),
            hook("AgmLJBMDCqWynYnQiPCuj9ewsNNsBJXyzoUhD9LJzN51", false),
            hook("Ak4X3R6ybzwQedKRHzAxfS1QZVh5XYFo9KA4E53eqmHZ", false),
            hook("6rm17kp4bWPF4xMUgymJ1yz6GLyrsJWbbiVYsnt2gps5", false),
        ]
    );
}

#[test]
fn hook_accounts_keep_their_writable_flags() {
    let swap = only_swap(TRANSFER_HOOK_WRITABLE_BUY);

    assert!(swap.is_buy());
    assert_eq!(swap.payer, pk("nof5xVKHJ3GGDE3dfXf24pGV7b4VnTRaQN4CmkznMwv"));
    assert_eq!(swap.pool, pk("4hresgq1aE6F2ACeAeC3sUSZHVEaSMaRacAQnwXp76jE"));
    assert_eq!(swap.quote_mint, pk(WSOL));
    assert_eq!(swap.amount_in, 99_000_000);
    assert_eq!(swap.actual_input_amount, 98_010_000);
    assert_eq!(swap.output_amount, 1_152_780_307_055);

    // The sysvar here belongs to the hook, not to the swap.
    assert!(!swap.has_instructions_sysvar);
    assert_eq!(
        swap.transfer_hook_accounts,
        vec![
            hook("GjucFNkLjTEEMmyxfaR73273Fohb32CHuLDY5A2D4u62", true),
            hook(INSTRUCTIONS_SYSVAR, false),
            hook("C6oQ6WxUUDu5qWikbHXqjuwSEwk8oKSyXNz6AQBBTRCn", false),
            hook("C3vEdPepTPRrJdQ4nQ3ZmhdXCmpKdGRKVUqxduHZWbdR", false),
            hook("dipY1shpTXvQtAp3LwBhYVEZndJfP7ZAS6sbSH61Sei", false),
        ]
    );
}

#[test]
fn legacy_swap_yields_one_event_for_its_two_event_cpis() {
    let swap = only_swap(SWAP_LEGACY_BUY);

    assert!(swap.is_buy());
    assert!(!swap.transfer_hook);
    assert_eq!(swap.swap_mode, 0);
    assert_eq!(swap.payer, pk("CT6n2L18XMPWZm9tdpuAPia1wJCYnwfEpWyQPWRPzTt9"));
    assert_eq!(swap.pool, pk("HLfVDQpRWL9KEHVXbGiRcgE3epYMLckY4iJa1KKaQvvD"));
    assert_eq!(swap.base_mint, pk("8fMRoytndcRCjbxa5TAY3Smmv9W7JXmFHgSxeQP9wizp"));
    assert_eq!(swap.quote_mint, pk(WSOL));
    assert_eq!(swap.token_base_program, pk(TOKEN_PROGRAM));
    // A buy pays its fee from the input.
    assert_eq!(swap.amount_in, 104_054_789);
    assert_eq!(swap.actual_input_amount, 100_933_145);
    assert_eq!(swap.minimum_amount_out, 2_883_031_391_682);
    assert_eq!(swap.output_amount, 3_603_789_239_603);
    assert_eq!((swap.trading_fee, swap.protocol_fee, swap.referral_fee), (2_497_316, 624_328, 0));
    assert_eq!(swap.referral_token_account, None);
    assert!(swap.transfer_hook_accounts.is_empty());
    assert!(!swap.has_instructions_sysvar);
}

#[test]
fn swap2_sale_reports_what_the_payer_received() {
    let swap = only_swap(SWAP2_SELL);

    assert!(!swap.is_buy());
    assert!(!swap.transfer_hook);
    assert_eq!(swap.swap_mode, 1);
    assert_eq!(swap.payer, pk("DZ1LPj7it1FJtYt6EqZX25SM8SqGxNVT9BgmKGrB966K"));
    assert_eq!(swap.quote_mint, pk(WSOL));
    assert_eq!(swap.amount_in, 6_297_406_455_448);
    assert_eq!(swap.actual_input_amount, swap.amount_in);
    assert_eq!(swap.output_amount, 824_699_561);
    assert_eq!(
        (swap.trading_fee, swap.protocol_fee, swap.referral_fee),
        (20_404_938, 4_080_988, 1_020_246)
    );
    assert!(swap.referral_token_account.is_some());
}

#[test]
fn top_level_swap2_is_parsed_like_a_routed_one() {
    let swap = only_swap(SWAP2_TOP_LEVEL_BUY);

    assert!(swap.is_buy());
    assert_eq!(swap.swap_mode, 0);
    assert_eq!(swap.payer, pk("87gVDCSif6RKf7eNZcekugkFa4mvkJoCkwc3fHgnYdvk"));
    assert_eq!(swap.quote_mint, pk(WSOL));
    assert_eq!(swap.amount_in, 1_000_000_000);
    assert_eq!(swap.actual_input_amount, 989_999_960);
    assert_eq!(swap.minimum_amount_out, 130_347_670_914);
    assert_eq!(swap.output_amount, 16_831_620_311_519);
    assert_eq!(swap.program, pk(DBC_PROGRAM));
}

#[test]
fn swap_filter_selects_dbc_swaps_alone() {
    let filter = EventTypeFilter::include_only(vec![EventType::MeteoraDbcSwap]);
    for fixture in [TRANSFER_HOOK_BUY, TRANSFER_HOOK_SYSVAR_SELL, SWAP_LEGACY_BUY] {
        let events = parse(fixture, Some(&filter));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DexEvent::MeteoraDbcSwap(_)));
    }
    // The completion is its own event type.
    let events = parse(TRANSFER_HOOK_CURVE_COMPLETE, Some(&filter));
    assert_eq!(events.len(), 1);

    let other = EventTypeFilter::include_only(vec![EventType::MeteoraDammV2Swap]);
    assert!(parse(TRANSFER_HOOK_BUY, Some(&other)).is_empty());
}
